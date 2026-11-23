//! SQLite persistence layer for memory and episode storage.
//!
//! This module provides a unified storage backend for all memory types,
//! including episode-based memories. The schema supports efficient queries
//! by type, tags, importance, and creation time.

use crate::{Memory, MemoryType};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

/// SQLite-backed persistent storage for memories
///
/// Stores all memory types (Episodic, Semantic, Procedural, etc.) in a
/// unified schema with efficient indexing for common query patterns.
pub struct MemoryStorage {
    conn: Connection,
}

impl MemoryStorage {
    /// Create new storage with file-backed SQLite database
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref()).context("Failed to open SQLite database")?;

        let storage = Self { conn };
        storage
            .initialize_schema()
            .context("Failed to initialize database schema")?;

        Ok(storage)
    }

    /// Create in-memory storage (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("Failed to create in-memory database")?;

        let storage = Self { conn };
        storage
            .initialize_schema()
            .context("Failed to initialize database schema")?;

        Ok(storage)
    }

    /// Initialize database schema with tables and indexes
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            -- Main memories table
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                memory_type TEXT NOT NULL,
                content_json TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                embedding_blob BLOB,
                created_at INTEGER NOT NULL,
                importance REAL NOT NULL,
                CHECK (importance >= 0.0 AND importance <= 1.0)
            );

            -- Indexes for common queries
            CREATE INDEX IF NOT EXISTS idx_memory_type ON memories(memory_type);
            CREATE INDEX IF NOT EXISTS idx_created_at ON memories(created_at);
            CREATE INDEX IF NOT EXISTS idx_importance ON memories(importance);
            CREATE INDEX IF NOT EXISTS idx_type_importance ON memories(memory_type, importance DESC);

            -- Tags table for many-to-many relationship
            CREATE TABLE IF NOT EXISTS memory_tags (
                memory_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (memory_id, tag),
                FOREIGN KEY (memory_id) REFERENCES memories(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_tags ON memory_tags(tag);

            -- Metadata table for schema versioning
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            INSERT OR IGNORE INTO metadata (key, value) VALUES ('schema_version', '1');
            INSERT OR IGNORE INTO metadata (key, value) VALUES ('created_at', datetime('now'));
            "#
        )?;

        Ok(())
    }

    /// Store memory in database (insert or replace)
    pub fn store_memory(&mut self, memory: &Memory) -> Result<()> {
        let content_json =
            serde_json::to_string(&memory.content).context("Failed to serialize memory content")?;

        let metadata_json = serde_json::to_string(&memory.metadata)
            .context("Failed to serialize memory metadata")?;

        let embedding_blob = memory.embedding.as_ref().map(|emb| {
            // Convert Vec<f32> to bytes
            emb.iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<u8>>()
        });

        let created_ts = memory.metadata.created_at.unix_timestamp();
        let memory_type_str = format!("{:?}", memory.memory_type);

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO memories 
                (id, memory_type, content_json, metadata_json, embedding_blob, created_at, importance)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                &memory.id,
                memory_type_str,
                content_json,
                metadata_json,
                embedding_blob,
                created_ts,
                memory.metadata.importance,
            ],
        ).context("Failed to insert memory")?;

        // Store tags
        for tag in &memory.metadata.tags {
            self.conn
                .execute(
                    "INSERT OR IGNORE INTO memory_tags (memory_id, tag) VALUES (?1, ?2)",
                    params![&memory.id, tag],
                )
                .context("Failed to insert memory tag")?;
        }

        Ok(())
    }

    /// Retrieve memory by ID
    pub fn get_memory(&self, id: &str) -> Result<Option<Memory>> {
        let mut stmt = self.conn.prepare(
            "SELECT memory_type, content_json, metadata_json, embedding_blob FROM memories WHERE id = ?1"
        )?;

        let result = stmt.query_row(params![id], |row| {
            let memory_type_str: String = row.get(0)?;
            let content_json: String = row.get(1)?;
            let metadata_json: String = row.get(2)?;
            let embedding_blob: Option<Vec<u8>> = row.get(3)?;

            Ok((memory_type_str, content_json, metadata_json, embedding_blob))
        });

        match result {
            Ok((memory_type_str, content_json, metadata_json, embedding_blob)) => {
                let memory_type = Self::parse_memory_type(&memory_type_str)?;
                let content = serde_json::from_str(&content_json)
                    .context("Failed to deserialize memory content")?;
                let metadata = serde_json::from_str(&metadata_json)
                    .context("Failed to deserialize memory metadata")?;

                let embedding = embedding_blob.map(|blob| {
                    blob.chunks_exact(4)
                        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                        .collect()
                });

                Ok(Some(Memory {
                    id: id.to_string(),
                    memory_type,
                    content,
                    metadata,
                    associations: vec![],
                    embedding,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e).context("Failed to query memory"),
        }
    }

    /// Query memories by type
    pub fn query_by_type(&self, memory_type: MemoryType) -> Result<Vec<String>> {
        let memory_type_str = format!("{:?}", memory_type);
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM memories WHERE memory_type = ?1 ORDER BY created_at DESC")?;

        let ids = stmt
            .query_map(params![memory_type_str], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(ids)
    }

    /// Query memories by tag
    pub fn query_by_tag(&self, tag: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT memory_id FROM memory_tags WHERE tag = ?1")?;

        let ids = stmt
            .query_map(params![tag], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(ids)
    }

    /// Query most recent memories
    pub fn query_recent(&self, limit: usize) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM memories ORDER BY created_at DESC LIMIT ?1")?;

        let ids = stmt
            .query_map(params![limit as i64], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(ids)
    }

    /// Query most important memories
    pub fn query_important(&self, min_importance: f32, limit: usize) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM memories WHERE importance >= ?1 ORDER BY importance DESC LIMIT ?2",
        )?;

        let ids = stmt
            .query_map(params![min_importance, limit as i64], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(ids)
    }

    /// Query memories by type and importance
    pub fn query_by_type_and_importance(
        &self,
        memory_type: MemoryType,
        min_importance: f32,
        limit: usize,
    ) -> Result<Vec<String>> {
        let memory_type_str = format!("{:?}", memory_type);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id FROM memories 
            WHERE memory_type = ?1 AND importance >= ?2 
            ORDER BY importance DESC 
            LIMIT ?3
            "#,
        )?;

        let ids = stmt
            .query_map(
                params![memory_type_str, min_importance, limit as i64],
                |row| row.get(0),
            )?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(ids)
    }

    /// Count total memories
    pub fn count_memories(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;

        Ok(count as usize)
    }

    /// Count memories by type
    pub fn count_by_type(&self, memory_type: MemoryType) -> Result<usize> {
        let memory_type_str = format!("{:?}", memory_type);
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE memory_type = ?1",
            params![memory_type_str],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    /// Delete memory by ID
    pub fn delete_memory(&mut self, id: &str) -> Result<bool> {
        let rows_affected = self
            .conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])?;

        Ok(rows_affected > 0)
    }

    /// Prune old memories before timestamp
    pub fn prune_old(&mut self, before_timestamp: i64) -> Result<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM memories WHERE created_at < ?1",
            params![before_timestamp],
        )?;

        Ok(deleted)
    }

    /// Prune low-importance memories
    pub fn prune_unimportant(&mut self, max_importance: f32) -> Result<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM memories WHERE importance < ?1",
            params![max_importance],
        )?;

        Ok(deleted)
    }

    /// Get all unique tags
    pub fn get_all_tags(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT tag FROM memory_tags ORDER BY tag")?;

        let tags = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(tags)
    }

    /// Get schema version
    pub fn get_schema_version(&self) -> Result<String> {
        let version: String = self.conn.query_row(
            "SELECT value FROM metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )?;

        Ok(version)
    }

    /// Parse memory type from string
    fn parse_memory_type(s: &str) -> Result<MemoryType> {
        match s {
            "Sensory" => Ok(MemoryType::Sensory),
            "Working" => Ok(MemoryType::Working),
            "Episodic" => Ok(MemoryType::Episodic),
            "Semantic" => Ok(MemoryType::Semantic),
            "Procedural" => Ok(MemoryType::Procedural),
            "Emotional" => Ok(MemoryType::Emotional),
            "Social" => Ok(MemoryType::Social),
            _ => Err(anyhow::anyhow!("Unknown memory type: {}", s)),
        }
    }

    /// Optimize database (VACUUM and ANALYZE)
    pub fn optimize(&mut self) -> Result<()> {
        self.conn.execute_batch("VACUUM; ANALYZE;")?;
        Ok(())
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<StorageStats> {
        let total_memories = self.count_memories()?;

        let episodic_count = self.count_by_type(MemoryType::Episodic)?;
        let semantic_count = self.count_by_type(MemoryType::Semantic)?;
        let procedural_count = self.count_by_type(MemoryType::Procedural)?;
        let emotional_count = self.count_by_type(MemoryType::Emotional)?;

        let total_tags = self.get_all_tags()?.len();

        let db_size: i64 = self
            .conn
            .query_row(
                "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(StorageStats {
            total_memories,
            episodic_count,
            semantic_count,
            procedural_count,
            emotional_count,
            total_tags,
            db_size_bytes: db_size as u64,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_memories: usize,
    pub episodic_count: usize,
    pub semantic_count: usize,
    pub procedural_count: usize,
    pub emotional_count: usize,
    pub total_tags: usize,
    pub db_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryContent, MemoryMetadata, MemorySource, SpatialTemporalContext};

    fn create_test_memory(id: &str, memory_type: MemoryType, importance: f32) -> Memory {
        Memory {
            id: id.to_string(),
            memory_type,
            content: MemoryContent {
                text: format!("Test memory {}", id),
                data: serde_json::json!({"test": true}),
                sensory_data: None,
                emotional_context: None,
                context: SpatialTemporalContext {
                    location: None,
                    time_period: None,
                    duration: None,
                    participants: vec![],
                    related_events: vec![],
                },
            },
            metadata: MemoryMetadata {
                created_at: time::OffsetDateTime::now_utc(),
                last_accessed: time::OffsetDateTime::now_utc(),
                access_count: 0,
                importance,
                confidence: 1.0,
                source: MemorySource::DirectExperience,
                tags: vec![],
                permanent: false,
                strength: 1.0,
                decay_factor: 1.0,
            },
            associations: vec![],
            embedding: None,
        }
    }

    #[test]
    fn test_storage_creation() {
        let storage = MemoryStorage::in_memory().expect("Failed to create storage");
        assert_eq!(storage.count_memories().unwrap(), 0);
    }

    #[test]
    fn test_store_and_retrieve() {
        let mut storage = MemoryStorage::in_memory().unwrap();
        let memory = create_test_memory("test_001", MemoryType::Episodic, 0.8);

        storage
            .store_memory(&memory)
            .expect("Failed to store memory");

        let retrieved = storage
            .get_memory("test_001")
            .expect("Failed to retrieve memory")
            .expect("Memory not found");

        assert_eq!(retrieved.id, "test_001");
        assert_eq!(retrieved.memory_type, MemoryType::Episodic);
        assert!((retrieved.metadata.importance - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_query_by_type() {
        let mut storage = MemoryStorage::in_memory().unwrap();

        storage
            .store_memory(&create_test_memory("ep1", MemoryType::Episodic, 0.8))
            .unwrap();
        storage
            .store_memory(&create_test_memory("ep2", MemoryType::Episodic, 0.7))
            .unwrap();
        storage
            .store_memory(&create_test_memory("sem1", MemoryType::Semantic, 0.9))
            .unwrap();

        let episodic_ids = storage.query_by_type(MemoryType::Episodic).unwrap();
        assert_eq!(episodic_ids.len(), 2);
        assert!(episodic_ids.contains(&"ep1".to_string()));
        assert!(episodic_ids.contains(&"ep2".to_string()));
    }

    #[test]
    fn test_tags() {
        let mut storage = MemoryStorage::in_memory().unwrap();
        let mut memory = create_test_memory("tagged", MemoryType::Episodic, 0.8);
        memory.metadata.tags = vec!["combat".to_string(), "boss".to_string()];

        storage.store_memory(&memory).unwrap();

        let combat_memories = storage.query_by_tag("combat").unwrap();
        assert_eq!(combat_memories.len(), 1);
        assert_eq!(combat_memories[0], "tagged");

        let all_tags = storage.get_all_tags().unwrap();
        assert_eq!(all_tags.len(), 2);
        assert!(all_tags.contains(&"combat".to_string()));
        assert!(all_tags.contains(&"boss".to_string()));
    }

    #[test]
    fn test_importance_query() {
        let mut storage = MemoryStorage::in_memory().unwrap();

        storage
            .store_memory(&create_test_memory("low", MemoryType::Episodic, 0.3))
            .unwrap();
        storage
            .store_memory(&create_test_memory("high", MemoryType::Episodic, 0.9))
            .unwrap();

        let important = storage.query_important(0.5, 10).unwrap();
        assert_eq!(important.len(), 1);
        assert_eq!(important[0], "high");
    }

    #[test]
    fn test_delete() {
        let mut storage = MemoryStorage::in_memory().unwrap();
        storage
            .store_memory(&create_test_memory("delete_me", MemoryType::Episodic, 0.8))
            .unwrap();

        assert_eq!(storage.count_memories().unwrap(), 1);

        let deleted = storage.delete_memory("delete_me").unwrap();
        assert!(deleted);

        assert_eq!(storage.count_memories().unwrap(), 0);
    }

    #[test]
    fn test_stats() {
        let mut storage = MemoryStorage::in_memory().unwrap();

        storage
            .store_memory(&create_test_memory("ep1", MemoryType::Episodic, 0.8))
            .unwrap();
        storage
            .store_memory(&create_test_memory("sem1", MemoryType::Semantic, 0.7))
            .unwrap();

        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.total_memories, 2);
        assert_eq!(stats.episodic_count, 1);
        assert_eq!(stats.semantic_count, 1);
    }
}
