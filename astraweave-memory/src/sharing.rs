//! Memory sharing and collaborative memory systems
//!
//! This module handles sharing memories between different agents/entities
//! while respecting privacy levels and sharing permissions.

use crate::memory_types::*;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Configuration for memory sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingConfig {
    /// Default sharing permissions for new memories
    pub default_sharing_type: SharingType,
    /// Default privacy level
    pub default_privacy_level: PrivacyLevel,
    /// Whether to allow automatic sharing based on context
    pub auto_sharing_enabled: bool,
    /// Maximum number of entities that can access a shared memory
    pub max_authorized_entities: usize,
}

impl Default for SharingConfig {
    fn default() -> Self {
        Self {
            default_sharing_type: SharingType::Restricted,
            default_privacy_level: PrivacyLevel::Personal,
            auto_sharing_enabled: false,
            max_authorized_entities: 10,
        }
    }
}

/// Request to share a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareRequest {
    /// ID of the memory to share
    pub memory_id: String,
    /// Target entity to share with
    pub target_entity: String,
    /// Type of sharing requested
    pub sharing_type: SharingType,
    /// Reason for sharing (for audit trail)
    pub reason: String,
    /// Conditions for sharing
    pub conditions: Vec<String>,
}

/// Result of a sharing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingResult {
    /// Whether the sharing was successful
    pub success: bool,
    /// Error message if unsuccessful
    pub error_message: Option<String>,
    /// Shared memory content (based on sharing type)
    pub shared_content: Option<SharedMemoryContent>,
    /// Sharing metadata
    pub sharing_metadata: SharingMetadata,
}

/// Content of a shared memory (filtered based on sharing type)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedMemoryContent {
    /// Filtered memory content
    pub content: String,
    /// Memory type
    pub memory_type: MemoryType,
    /// Timestamp
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub created_at: time::OffsetDateTime,
    /// Importance level
    pub importance: f32,
    /// Associated entities (filtered)
    pub entities: Vec<String>,
}

/// Audit log entry for memory sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingAuditEntry {
    /// Unique ID for this sharing event
    pub event_id: String,
    /// Memory that was shared
    pub memory_id: String,
    /// Source entity (sharer)
    pub source_entity: String,
    /// Target entity (receiver)
    pub target_entity: String,
    /// Type of sharing
    pub sharing_type: SharingType,
    /// When the sharing occurred
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub timestamp: time::OffsetDateTime,
    /// Success status
    pub success: bool,
    /// Reason for sharing
    pub reason: String,
}

/// Memory sharing engine
#[derive(Debug)]
pub struct SharingEngine {
    config: SharingConfig,
    audit_log: Vec<SharingAuditEntry>,
}

impl SharingEngine {
    /// Create a new sharing engine
    pub fn new(config: SharingConfig) -> Self {
        Self {
            config,
            audit_log: Vec::new(),
        }
    }

    /// Share a memory with another entity
    pub fn share_memory(
        &mut self,
        request: &ShareRequest,
        memory: &Memory,
        requesting_entity: &str,
    ) -> Result<SharingResult> {
        // Check if the memory has sharing metadata
        let sharing_metadata = self.get_or_create_sharing_metadata(memory);

        // Validate sharing permissions
        if let Err(error) =
            self.validate_sharing_request(request, &sharing_metadata, requesting_entity)
        {
            let result = SharingResult {
                success: false,
                error_message: Some(error.to_string()),
                shared_content: None,
                sharing_metadata: sharing_metadata.clone(),
            };

            // Log the failed attempt
            self.log_sharing_event(request, requesting_entity, false, &error.to_string());

            return Ok(result);
        }

        // Generate shared content based on sharing type
        let shared_content = self.generate_shared_content(memory, &request.sharing_type)?;

        // Create successful result
        let mut updated_metadata = sharing_metadata.clone();
        if !updated_metadata
            .authorized_entities
            .contains(&request.target_entity)
        {
            updated_metadata
                .authorized_entities
                .push(request.target_entity.clone());
        }

        let result = SharingResult {
            success: true,
            error_message: None,
            shared_content: Some(shared_content),
            sharing_metadata: updated_metadata,
        };

        // Log successful sharing
        self.log_sharing_event(request, requesting_entity, true, "Success");

        Ok(result)
    }

    /// Validate a sharing request
    fn validate_sharing_request(
        &self,
        request: &ShareRequest,
        metadata: &SharingMetadata,
        requesting_entity: &str,
    ) -> Result<()> {
        // Check if sharing is allowed at all
        if !metadata.shareable {
            return Err(anyhow!("Memory is not shareable"));
        }

        // Check privacy level
        match metadata.privacy_level {
            PrivacyLevel::Secret => {
                return Err(anyhow!(
                    "Memory has secret privacy level and cannot be shared"
                ));
            }
            PrivacyLevel::Personal => {
                if !metadata
                    .authorized_entities
                    .contains(&requesting_entity.to_string())
                {
                    return Err(anyhow!(
                        "Requesting entity not authorized for personal memory"
                    ));
                }
            }
            PrivacyLevel::Group => {
                // For group level, check if both entities are in authorized list
                if !metadata
                    .authorized_entities
                    .contains(&requesting_entity.to_string())
                    || !metadata
                        .authorized_entities
                        .contains(&request.target_entity)
                {
                    return Err(anyhow!("Entity not in authorized group"));
                }
            }
            PrivacyLevel::Public => {
                // Public memories can be shared by anyone
            }
        }

        // Check sharing type compatibility
        match (&metadata.sharing_type, &request.sharing_type) {
            (SharingType::Restricted, _) => {
                return Err(anyhow!("Memory has restricted sharing type"));
            }
            (SharingType::Metadata, SharingType::Full | SharingType::Summary) => {
                return Err(anyhow!("Memory only allows metadata sharing"));
            }
            (SharingType::Summary, SharingType::Full) => {
                return Err(anyhow!("Memory only allows summary sharing"));
            }
            _ => {} // Compatible sharing types
        }

        // Check entity limit
        if metadata.authorized_entities.len() >= self.config.max_authorized_entities {
            return Err(anyhow!("Maximum authorized entities limit reached"));
        }

        Ok(())
    }

    /// Generate shared content based on sharing type
    fn generate_shared_content(
        &self,
        memory: &Memory,
        sharing_type: &SharingType,
    ) -> Result<SharedMemoryContent> {
        let content = match sharing_type {
            SharingType::Full => memory.content.text.clone(),
            SharingType::Summary => self.generate_summary(&memory.content.text),
            SharingType::Metadata => format!(
                "Memory of type {:?} created on {}",
                memory.memory_type,
                crate::utc_stamp::format_utc_date(memory.metadata.created_at)
            ),
            SharingType::Restricted => {
                return Err(anyhow!(
                    "Cannot generate content for restricted sharing type"
                ));
            }
        };

        // Filter entities based on sharing type
        let entities = match sharing_type {
            SharingType::Full => memory.content.context.participants.clone(),
            SharingType::Summary | SharingType::Metadata => {
                // Only include non-sensitive entities
                memory
                    .content
                    .context
                    .participants
                    .iter()
                    .filter(|entity| !entity.starts_with("private:"))
                    .cloned()
                    .collect()
            }
            SharingType::Restricted => Vec::new(),
        };

        Ok(SharedMemoryContent {
            content,
            memory_type: memory.memory_type.clone(),
            created_at: memory.metadata.created_at,
            importance: memory.metadata.importance,
            entities,
        })
    }

    /// Generate a summary of memory content
    fn generate_summary(&self, content: &str) -> String {
        // Simple summarization (in practice, use LLM)
        let words: Vec<&str> = content.split_whitespace().collect();
        if words.len() <= 20 {
            return content.to_string();
        }

        let summary_length = (words.len() / 3).max(10);
        let mut summary_words = Vec::new();
        summary_words.extend_from_slice(&words[..summary_length]);
        summary_words.push("[...]");

        summary_words.join(" ")
    }

    /// Get or create sharing metadata for a memory
    fn get_or_create_sharing_metadata(&self, _memory: &Memory) -> SharingMetadata {
        // In practice, this would be stored with the memory
        // For now, create default metadata
        SharingMetadata {
            shareable: true,
            authorized_entities: vec!["owner".to_string()],
            sharing_type: self.config.default_sharing_type.clone(),
            privacy_level: self.config.default_privacy_level.clone(),
            sharing_conditions: Vec::new(),
        }
    }

    /// Log a sharing event
    fn log_sharing_event(
        &mut self,
        request: &ShareRequest,
        requesting_entity: &str,
        success: bool,
        message: &str,
    ) {
        let entry = SharingAuditEntry {
            event_id: uuid::Uuid::new_v4().to_string(),
            memory_id: request.memory_id.clone(),
            source_entity: requesting_entity.to_string(),
            target_entity: request.target_entity.clone(),
            sharing_type: request.sharing_type.clone(),
            timestamp: time::OffsetDateTime::now_utc(),
            success,
            reason: if success {
                request.reason.clone()
            } else {
                message.to_string()
            },
        };

        self.audit_log.push(entry);
    }

    /// Get sharing audit log for a memory
    pub fn get_audit_log(&self, memory_id: &str) -> Vec<&SharingAuditEntry> {
        self.audit_log
            .iter()
            .filter(|entry| entry.memory_id == memory_id)
            .collect()
    }

    /// Get all memories that an entity has access to
    pub fn get_accessible_memories(&self, entity_id: &str, all_memories: &[Memory]) -> Vec<String> {
        let mut accessible = Vec::new();

        for memory in all_memories {
            let metadata = self.get_or_create_sharing_metadata(memory);

            // Check if entity has access
            let has_access = match metadata.privacy_level {
                PrivacyLevel::Public => true,
                PrivacyLevel::Group | PrivacyLevel::Personal => metadata
                    .authorized_entities
                    .contains(&entity_id.to_string()),
                PrivacyLevel::Secret => false,
            };

            if has_access {
                accessible.push(memory.id.clone());
            }
        }

        accessible
    }

    /// Create a shared memory cluster
    pub fn create_shared_cluster(
        &mut self,
        cluster_name: String,
        memory_ids: Vec<String>,
        _authorized_entities: Vec<String>,
        _privacy_level: PrivacyLevel,
    ) -> Result<MemoryCluster> {
        // Validate that all memories can be shared
        // In practice, you would check each memory's sharing permissions

        let mut cluster = MemoryCluster::new(
            cluster_name,
            ClusterType::Concept, // Default cluster type for shared clusters
            "shared_cluster".to_string(),
        );

        // Add memories to cluster
        for memory_id in memory_ids {
            cluster.add_memory(memory_id);
        }

        // Set cluster importance based on included memories
        // This would be calculated based on the actual memories
        cluster.importance = 0.7;

        Ok(cluster)
    }

    /// Revoke sharing access for an entity
    pub fn revoke_access(
        &mut self,
        memory_id: &str,
        target_entity: &str,
        revoking_entity: &str,
    ) -> Result<()> {
        // Log the revocation
        let entry = SharingAuditEntry {
            event_id: uuid::Uuid::new_v4().to_string(),
            memory_id: memory_id.to_string(),
            source_entity: revoking_entity.to_string(),
            target_entity: target_entity.to_string(),
            sharing_type: SharingType::Restricted,
            timestamp: time::OffsetDateTime::now_utc(),
            success: true,
            reason: "Access revoked".to_string(),
        };

        self.audit_log.push(entry);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryContent, SpatialTemporalContext};

    // Helper function to create a Working memory (no public constructor exists)
    fn create_working_memory(text: &str) -> Memory {
        let content = MemoryContent {
            text: text.to_string(),
            data: serde_json::json!({}),
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location: None,
                time_period: None,
                duration: None,
                participants: vec![],
                related_events: vec![],
            },
        };
        Memory::new(MemoryType::Working, content)
    }

    #[test]
    fn test_sharing_engine_creation() {
        let config = SharingConfig::default();
        let engine = SharingEngine::new(config);
        assert_eq!(engine.config.max_authorized_entities, 10);
    }

    #[test]
    fn test_memory_sharing() {
        // Create config that allows sharing (default is Restricted)
        let config = SharingConfig {
            default_sharing_type: SharingType::Full,
            default_privacy_level: PrivacyLevel::Personal,
            auto_sharing_enabled: false,
            max_authorized_entities: 10,
        };
        let mut engine = SharingEngine::new(config);
        let memory = Memory::episodic(
            "Met with the team today".to_string(),
            vec!["Alice".to_string(), "Bob".to_string()],
            Some("office".to_string()),
        );

        let request = ShareRequest {
            memory_id: memory.id.clone(),
            target_entity: "colleague".to_string(),
            sharing_type: SharingType::Summary,
            reason: "Project collaboration".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.share_memory(&request, &memory, "owner").unwrap();
        if !result.success {
            eprintln!("Sharing failed: {:?}", result.error_message);
        }
        assert!(result.success, "Sharing failed: {:?}", result.error_message);
        assert!(result.shared_content.is_some());
    }

    #[test]
    fn test_sharing_validation() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: false,
            authorized_entities: vec!["owner".to_string()],
            sharing_type: SharingType::Restricted,
            privacy_level: PrivacyLevel::Personal,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "other".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "owner");
        assert!(result.is_err());
    }

    #[test]
    fn test_audit_logging() {
        // Create config that allows sharing (default is Restricted)
        let config = SharingConfig {
            default_sharing_type: SharingType::Full,
            default_privacy_level: PrivacyLevel::Personal,
            auto_sharing_enabled: false,
            max_authorized_entities: 10,
        };
        let mut engine = SharingEngine::new(config);
        let memory = Memory::semantic("Test knowledge".to_string(), "test".to_string());

        let request = ShareRequest {
            memory_id: memory.id.clone(),
            target_entity: "researcher".to_string(),
            sharing_type: SharingType::Full,
            reason: "Research collaboration".to_string(),
            conditions: Vec::new(),
        };

        engine.share_memory(&request, &memory, "owner").unwrap();

        let audit_entries = engine.get_audit_log(&memory.id);
        assert_eq!(audit_entries.len(), 1);
        assert!(audit_entries[0].success);
    }

    // ======================= NEW TESTS =======================

    // --- SharingConfig Tests ---
    #[test]
    fn test_sharing_config_default_values() {
        let config = SharingConfig::default();
        assert_eq!(config.default_sharing_type, SharingType::Restricted);
        assert_eq!(config.default_privacy_level, PrivacyLevel::Personal);
        assert!(!config.auto_sharing_enabled);
        assert_eq!(config.max_authorized_entities, 10);
    }

    #[test]
    fn test_sharing_config_custom_values() {
        let config = SharingConfig {
            default_sharing_type: SharingType::Summary,
            default_privacy_level: PrivacyLevel::Group,
            auto_sharing_enabled: true,
            max_authorized_entities: 50,
        };
        assert_eq!(config.default_sharing_type, SharingType::Summary);
        assert_eq!(config.default_privacy_level, PrivacyLevel::Group);
        assert!(config.auto_sharing_enabled);
        assert_eq!(config.max_authorized_entities, 50);
    }

    // --- validate_sharing_request Tests ---
    #[test]
    fn test_validate_not_shareable() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: false,
            authorized_entities: vec!["owner".to_string()],
            sharing_type: SharingType::Full,
            privacy_level: PrivacyLevel::Public,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "other".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "owner");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not shareable"));
    }

    #[test]
    fn test_validate_secret_privacy() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["owner".to_string()],
            sharing_type: SharingType::Full,
            privacy_level: PrivacyLevel::Secret,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "other".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "owner");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("secret"));
    }

    #[test]
    fn test_validate_personal_unauthorized() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["owner".to_string()],
            sharing_type: SharingType::Full,
            privacy_level: PrivacyLevel::Personal,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "other".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        // Unauthorized entity trying to share
        let result = engine.validate_sharing_request(&request, &metadata, "unauthorized");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not authorized"));
    }

    #[test]
    fn test_validate_group_both_authorized() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["owner".to_string(), "target".to_string()],
            sharing_type: SharingType::Full,
            privacy_level: PrivacyLevel::Group,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "target".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "owner");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_group_target_not_authorized() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["owner".to_string()], // target not in group
            sharing_type: SharingType::Full,
            privacy_level: PrivacyLevel::Group,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "outside_target".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "owner");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not in authorized group"));
    }

    #[test]
    fn test_validate_public_allows_anyone() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec![], // No one specifically authorized
            sharing_type: SharingType::Full,
            privacy_level: PrivacyLevel::Public,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "anyone".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "random_entity");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_restricted_sharing_type() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["owner".to_string()],
            sharing_type: SharingType::Restricted,
            privacy_level: PrivacyLevel::Personal,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "other".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "owner");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("restricted"));
    }

    #[test]
    fn test_validate_metadata_only_rejects_full() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["owner".to_string()],
            sharing_type: SharingType::Metadata,
            privacy_level: PrivacyLevel::Personal,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "other".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "owner");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("only allows metadata"));
    }

    #[test]
    fn test_validate_summary_only_rejects_full() {
        let engine = SharingEngine::new(SharingConfig::default());
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["owner".to_string()],
            sharing_type: SharingType::Summary,
            privacy_level: PrivacyLevel::Personal,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "other".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "owner");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("only allows summary"));
    }

    #[test]
    fn test_validate_max_entities_limit() {
        let config = SharingConfig {
            max_authorized_entities: 2,
            ..Default::default()
        };
        let engine = SharingEngine::new(config);
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["a".to_string(), "b".to_string()], // Already at limit
            sharing_type: SharingType::Full,
            privacy_level: PrivacyLevel::Public,
            sharing_conditions: Vec::new(),
        };

        let request = ShareRequest {
            memory_id: "test".to_string(),
            target_entity: "c".to_string(), // Trying to add a third
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.validate_sharing_request(&request, &metadata, "a");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("limit reached"));
    }

    // --- generate_shared_content Tests ---
    #[test]
    fn test_generate_content_full_sharing() {
        let engine = SharingEngine::new(SharingConfig::default());
        let memory = Memory::episodic(
            "Full content here".to_string(),
            vec!["Alice".to_string()],
            None,
        );

        let content = engine
            .generate_shared_content(&memory, &SharingType::Full)
            .unwrap();

        assert_eq!(content.content, "Full content here");
        assert!(content.entities.contains(&"Alice".to_string()));
    }

    #[test]
    fn test_generate_content_summary_truncates() {
        let engine = SharingEngine::new(SharingConfig::default());
        let long_text = "word ".repeat(100); // 100 words
        let memory = create_working_memory(&long_text);

        let content = engine
            .generate_shared_content(&memory, &SharingType::Summary)
            .unwrap();

        // Summary should be shorter and end with [...]
        assert!(content.content.len() < long_text.len());
        assert!(content.content.contains("[...]"));
    }

    #[test]
    fn test_generate_content_summary_short_text_unchanged() {
        let engine = SharingEngine::new(SharingConfig::default());
        let short_text = "This is a short memory".to_string();
        let memory = create_working_memory(&short_text);

        let content = engine
            .generate_shared_content(&memory, &SharingType::Summary)
            .unwrap();

        assert_eq!(content.content, short_text);
    }

    #[test]
    fn test_generate_content_metadata_only() {
        let engine = SharingEngine::new(SharingConfig::default());
        let memory = Memory::semantic("Secret knowledge".to_string(), "facts".to_string());

        let content = engine
            .generate_shared_content(&memory, &SharingType::Metadata)
            .unwrap();

        // Should NOT contain the actual content
        assert!(!content.content.contains("Secret knowledge"));
        // Should contain type and date info
        assert!(content.content.contains("Semantic"));
    }

    #[test]
    fn test_generate_content_restricted_fails() {
        let engine = SharingEngine::new(SharingConfig::default());
        let memory = create_working_memory("Test");

        let result = engine.generate_shared_content(&memory, &SharingType::Restricted);

        assert!(result.is_err());
    }

    #[test]
    fn test_generate_content_filters_private_entities() {
        let engine = SharingEngine::new(SharingConfig::default());
        let memory = Memory::episodic(
            "Meeting".to_string(),
            vec!["Alice".to_string(), "private:Bob".to_string()],
            None,
        );

        let content = engine
            .generate_shared_content(&memory, &SharingType::Summary)
            .unwrap();

        assert!(content.entities.contains(&"Alice".to_string()));
        assert!(!content.entities.iter().any(|e| e.starts_with("private:")));
    }

    // --- share_memory Tests ---
    #[test]
    fn test_share_memory_success_updates_metadata() {
        let config = SharingConfig {
            default_sharing_type: SharingType::Full,
            default_privacy_level: PrivacyLevel::Personal,
            ..Default::default()
        };
        let mut engine = SharingEngine::new(config);
        let memory = create_working_memory("Test content");

        let request = ShareRequest {
            memory_id: memory.id.clone(),
            target_entity: "new_entity".to_string(),
            sharing_type: SharingType::Full,
            reason: "Sharing for test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.share_memory(&request, &memory, "owner").unwrap();

        assert!(result.success);
        assert!(result
            .sharing_metadata
            .authorized_entities
            .contains(&"new_entity".to_string()));
    }

    #[test]
    fn test_share_memory_failure_logs_error() {
        let mut engine = SharingEngine::new(SharingConfig::default()); // Default is Restricted
        let memory = create_working_memory("Test");

        let request = ShareRequest {
            memory_id: memory.id.clone(),
            target_entity: "target".to_string(),
            sharing_type: SharingType::Full,
            reason: "Should fail".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.share_memory(&request, &memory, "owner").unwrap();

        assert!(!result.success);
        assert!(result.error_message.is_some());

        // Check audit log recorded the failure
        let audit = engine.get_audit_log(&memory.id);
        assert!(!audit.is_empty());
        assert!(!audit[0].success);
    }

    #[test]
    fn test_share_memory_returns_shared_content() {
        let config = SharingConfig {
            default_sharing_type: SharingType::Full,
            default_privacy_level: PrivacyLevel::Personal,
            ..Default::default()
        };
        let mut engine = SharingEngine::new(config);
        let memory = create_working_memory("Actual content");

        let request = ShareRequest {
            memory_id: memory.id.clone(),
            target_entity: "target".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let result = engine.share_memory(&request, &memory, "owner").unwrap();

        let content = result.shared_content.unwrap();
        assert_eq!(content.content, "Actual content");
    }

    // --- get_audit_log Tests ---
    #[test]
    fn test_get_audit_log_empty() {
        let engine = SharingEngine::new(SharingConfig::default());
        let audit = engine.get_audit_log("nonexistent");
        assert!(audit.is_empty());
    }

    #[test]
    fn test_get_audit_log_filters_by_memory_id() {
        let config = SharingConfig {
            default_sharing_type: SharingType::Full,
            default_privacy_level: PrivacyLevel::Personal,
            ..Default::default()
        };
        let mut engine = SharingEngine::new(config);

        let memory1 = create_working_memory("Mem1");
        let memory2 = create_working_memory("Mem2");

        // Share both memories
        for mem in [&memory1, &memory2] {
            let request = ShareRequest {
                memory_id: mem.id.clone(),
                target_entity: "target".to_string(),
                sharing_type: SharingType::Full,
                reason: "test".to_string(),
                conditions: Vec::new(),
            };
            engine.share_memory(&request, mem, "owner").unwrap();
        }

        let audit1 = engine.get_audit_log(&memory1.id);
        let audit2 = engine.get_audit_log(&memory2.id);

        assert_eq!(audit1.len(), 1);
        assert_eq!(audit2.len(), 1);
        assert_eq!(audit1[0].memory_id, memory1.id);
        assert_eq!(audit2[0].memory_id, memory2.id);
    }

    // --- get_accessible_memories Tests ---
    #[test]
    fn test_get_accessible_memories_public() {
        let config = SharingConfig {
            default_privacy_level: PrivacyLevel::Public,
            ..Default::default()
        };
        let engine = SharingEngine::new(config);

        let memories = vec![create_working_memory("Mem1"), create_working_memory("Mem2")];

        let accessible = engine.get_accessible_memories("anyone", &memories);

        // Public memories should be accessible to anyone
        assert_eq!(accessible.len(), 2);
    }

    #[test]
    fn test_get_accessible_memories_personal_authorized() {
        let config = SharingConfig {
            default_privacy_level: PrivacyLevel::Personal,
            ..Default::default()
        };
        let engine = SharingEngine::new(config);

        let memories = vec![create_working_memory("Test")];

        // "owner" is in default authorized list
        let accessible = engine.get_accessible_memories("owner", &memories);
        assert_eq!(accessible.len(), 1);
    }

    #[test]
    fn test_get_accessible_memories_personal_unauthorized() {
        let config = SharingConfig {
            default_privacy_level: PrivacyLevel::Personal,
            ..Default::default()
        };
        let engine = SharingEngine::new(config);

        let memories = vec![create_working_memory("Test")];

        // "stranger" is not in default authorized list
        let accessible = engine.get_accessible_memories("stranger", &memories);
        assert!(accessible.is_empty());
    }

    // --- create_shared_cluster Tests ---
    #[test]
    fn test_create_shared_cluster_basic() {
        let mut engine = SharingEngine::new(SharingConfig::default());

        let cluster = engine
            .create_shared_cluster(
                "Test Cluster".to_string(),
                vec!["mem1".to_string(), "mem2".to_string()],
                vec!["user1".to_string()],
                PrivacyLevel::Group,
            )
            .unwrap();

        assert_eq!(cluster.name, "Test Cluster");
        assert!(cluster.memory_ids.contains(&"mem1".to_string()));
        assert!(cluster.memory_ids.contains(&"mem2".to_string()));
    }

    #[test]
    fn test_create_shared_cluster_importance() {
        let mut engine = SharingEngine::new(SharingConfig::default());

        let cluster = engine
            .create_shared_cluster(
                "Important Cluster".to_string(),
                vec!["mem1".to_string()],
                vec!["user1".to_string()],
                PrivacyLevel::Public,
            )
            .unwrap();

        assert!((cluster.importance - 0.7).abs() < 0.001);
    }

    // --- revoke_access Tests ---
    #[test]
    fn test_revoke_access_logs_event() {
        let mut engine = SharingEngine::new(SharingConfig::default());

        engine
            .revoke_access("mem123", "target_user", "admin")
            .unwrap();

        let audit = engine.get_audit_log("mem123");
        assert_eq!(audit.len(), 1);
        assert!(audit[0].success);
        assert_eq!(audit[0].target_entity, "target_user");
        assert_eq!(audit[0].source_entity, "admin");
        assert_eq!(audit[0].reason, "Access revoked");
    }

    #[test]
    fn test_revoke_access_multiple() {
        let mut engine = SharingEngine::new(SharingConfig::default());

        engine.revoke_access("mem123", "user1", "admin").unwrap();
        engine.revoke_access("mem123", "user2", "admin").unwrap();
        engine.revoke_access("mem456", "user1", "admin").unwrap();

        let audit123 = engine.get_audit_log("mem123");
        let audit456 = engine.get_audit_log("mem456");

        assert_eq!(audit123.len(), 2);
        assert_eq!(audit456.len(), 1);
    }

    // --- ShareRequest Tests ---
    #[test]
    fn test_share_request_with_conditions() {
        let request = ShareRequest {
            memory_id: "mem1".to_string(),
            target_entity: "target".to_string(),
            sharing_type: SharingType::Summary,
            reason: "Testing".to_string(),
            conditions: vec!["NDA signed".to_string(), "Time limited".to_string()],
        };

        assert_eq!(request.conditions.len(), 2);
        assert!(request.conditions.contains(&"NDA signed".to_string()));
    }

    // --- SharedMemoryContent Tests ---
    #[test]
    fn test_shared_memory_content_preserves_metadata() {
        let engine = SharingEngine::new(SharingConfig::default());
        let mut memory = create_working_memory("Content");
        memory.metadata.importance = 0.95;

        let content = engine
            .generate_shared_content(&memory, &SharingType::Full)
            .unwrap();

        assert!((content.importance - 0.95).abs() < 0.001);
        assert_eq!(content.memory_type, MemoryType::Working);
    }

    // --- SharingAuditEntry Tests ---
    #[test]
    fn test_audit_entry_has_timestamp() {
        let config = SharingConfig {
            default_sharing_type: SharingType::Full,
            default_privacy_level: PrivacyLevel::Personal,
            ..Default::default()
        };
        let mut engine = SharingEngine::new(config);
        let memory = create_working_memory("Test");

        let request = ShareRequest {
            memory_id: memory.id.clone(),
            target_entity: "target".to_string(),
            sharing_type: SharingType::Full,
            reason: "test".to_string(),
            conditions: Vec::new(),
        };

        let before = time::OffsetDateTime::now_utc();
        engine.share_memory(&request, &memory, "owner").unwrap();
        let after = time::OffsetDateTime::now_utc();

        let audit = engine.get_audit_log(&memory.id);
        assert!(audit[0].timestamp >= before);
        assert!(audit[0].timestamp <= after);
    }

    // --- generate_summary Tests ---
    #[test]
    fn test_generate_summary_very_short() {
        let engine = SharingEngine::new(SharingConfig::default());
        let short = "Hi";
        let summary = engine.generate_summary(short);
        assert_eq!(summary, "Hi");
    }

    #[test]
    fn test_generate_summary_exactly_20_words() {
        let engine = SharingEngine::new(SharingConfig::default());
        let text = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty";
        let summary = engine.generate_summary(text);
        assert_eq!(summary, text); // 20 words, unchanged
    }

    #[test]
    fn test_generate_summary_truncates_long() {
        let engine = SharingEngine::new(SharingConfig::default());
        let long = "word ".repeat(60); // 60 words
        let summary = engine.generate_summary(&long);

        assert!(summary.contains("[...]"));
        assert!(summary.split_whitespace().count() < 60);
    }
}
