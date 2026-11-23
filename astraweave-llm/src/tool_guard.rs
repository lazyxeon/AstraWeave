//! Tool Guard - Security validation layer for LLM-generated actions
//!
//! Prevents LLMs from executing invalid, dangerous, or impossible actions by validating
//! tool calls against world state and security policies.
//!
//! # Features
//! - Allowlist/denylist for tool invocations
//! - World state validation (e.g., "attack" requires enemy in range)
//! - Resource limits (cooldowns, inventory checks)
//! - Audit logging for security monitoring
//!
//! # Example
//! ```
//! use astraweave_llm::tool_guard::{ToolGuard, ToolPolicy};
//! use astraweave_core::ActionStep;
//!
//! let mut guard = ToolGuard::new();
//! guard.set_policy("CoverFire", ToolPolicy::Restricted);
//!
//! let action = ActionStep::CoverFire { target_id: 99, duration: 2.0 };
//!
//! // Validate against world state
//! let is_valid = guard.validate_action(&action, &|_action| {
//!     // Custom validation logic
//!     true
//! });
//! ```

use astraweave_core::ActionStep;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, warn};

/// Security policy for a specific tool/action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ToolPolicy {
    /// Action is always allowed
    Allowed,
    /// Action requires validation against world state
    Restricted,
    /// Action is never allowed
    Denied,
}

/// Validation result for a tool invocation
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
#[must_use]
pub enum ValidationResult {
    /// Action is valid and can be executed
    Valid,
    /// Action is invalid with a reason
    Invalid { reason: String },
    /// Action is denied by policy
    Denied { action: String },
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            ValidationResult::Invalid { reason } => Some(reason),
            ValidationResult::Denied { action } => Some(action),
            ValidationResult::Valid => None,
        }
    }
}

/// Audit log entry for tool invocations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub timestamp: time::OffsetDateTime,
    pub action_type: String, // "MoveTo", "CoverFire", etc.
    pub result: String,      // "valid", "invalid", "denied"
    pub reason: Option<String>,
}

/// Tool Guard validates LLM-generated actions against security policies
pub struct ToolGuard {
    /// Policy map: action name -> policy
    policies: Arc<DashMap<String, ToolPolicy>>,
    /// Default policy for unknown actions
    default_policy: ToolPolicy,
    /// Audit log (limited to last N entries)
    audit_log: Arc<DashMap<uuid::Uuid, AuditEntry>>,
    max_audit_entries: usize,
}

impl Default for ToolGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolGuard {
    /// Create a new ToolGuard with default secure settings
    pub fn new() -> Self {
        let guard = Self {
            policies: Arc::new(DashMap::new()),
            default_policy: ToolPolicy::Restricted, // Secure by default
            audit_log: Arc::new(DashMap::new()),
            max_audit_entries: 1000,
        };

        // Set up default policies for common actions
        guard.set_policy("MoveTo", ToolPolicy::Restricted);
        guard.set_policy("Attack", ToolPolicy::Restricted);
        guard.set_policy("UseItem", ToolPolicy::Restricted);
        guard.set_policy("Throw", ToolPolicy::Restricted);
        guard.set_policy("CoverFire", ToolPolicy::Restricted);
        guard.set_policy("Wait", ToolPolicy::Allowed);
        guard.set_policy("Look", ToolPolicy::Allowed);
        guard.set_policy("ExecuteCode", ToolPolicy::Denied); // Never allow arbitrary code
        guard.set_policy("DeleteFile", ToolPolicy::Denied); // Never allow file operations
        guard.set_policy("ModifyWorld", ToolPolicy::Denied); // Never allow direct world edits

        guard
    }

    /// Set the policy for a specific action
    pub fn set_policy(&self, action: &str, policy: ToolPolicy) {
        self.policies.insert(action.to_string(), policy);
        debug!("Set policy for '{}': {:?}", action, policy);
    }

    /// Get the policy for an action
    pub fn get_policy(&self, action: &str) -> ToolPolicy {
        self.policies
            .get(action)
            .map(|p| *p)
            .unwrap_or(self.default_policy)
    }

    /// Set the default policy for unknown actions
    pub fn set_default_policy(&mut self, policy: ToolPolicy) {
        self.default_policy = policy;
    }

    /// Validate an action against security policies
    ///
    /// # Arguments
    /// * `action` - The action to validate
    /// * `validator` - Custom validation function (checks world state, cooldowns, etc.)
    pub fn validate_action<F>(&self, action: &ActionStep, validator: &F) -> ValidationResult
    where
        F: Fn(&ActionStep) -> bool,
    {
        let action_name = Self::action_name(action);
        let policy = self.get_policy(action_name);

        let result = match policy {
            ToolPolicy::Denied => {
                warn!("Denied action: {}", action_name);
                ValidationResult::Denied {
                    action: action_name.to_string(),
                }
            }
            ToolPolicy::Allowed => ValidationResult::Valid,
            ToolPolicy::Restricted => {
                // Run custom validation
                if validator(action) {
                    ValidationResult::Valid
                } else {
                    ValidationResult::Invalid {
                        reason: format!("Validation failed for action '{}'", action_name),
                    }
                }
            }
        };

        // Log to audit trail
        self.log_validation(action, &result);

        result
    }

    /// Get the action name from an ActionStep enum
    fn action_name(action: &ActionStep) -> &str {
        match action {
            ActionStep::MoveTo { .. } => "MoveTo",
            ActionStep::Throw { .. } => "Throw",
            ActionStep::CoverFire { .. } => "CoverFire",
            ActionStep::Revive { .. } => "Revive",
            // Phase 7: Handle all new tools
            ActionStep::Approach { .. } => "Approach",
            ActionStep::Retreat { .. } => "Retreat",
            ActionStep::TakeCover { .. } => "TakeCover",
            ActionStep::Strafe { .. } => "Strafe",
            ActionStep::Patrol { .. } => "Patrol",
            ActionStep::Attack { .. } => "Attack",
            ActionStep::AimedShot { .. } => "AimedShot",
            ActionStep::QuickAttack { .. } => "QuickAttack",
            ActionStep::HeavyAttack { .. } => "HeavyAttack",
            ActionStep::AoEAttack { .. } => "AoEAttack",
            ActionStep::ThrowExplosive { .. } => "ThrowExplosive",
            ActionStep::Charge { .. } => "Charge",
            ActionStep::Block => "Block",
            ActionStep::Dodge { .. } => "Dodge",
            ActionStep::Parry => "Parry",
            ActionStep::ThrowSmoke { .. } => "ThrowSmoke",
            ActionStep::Heal { .. } => "Heal",
            ActionStep::UseDefensiveAbility { .. } => "UseDefensiveAbility",
            ActionStep::EquipWeapon { .. } => "EquipWeapon",
            ActionStep::SwitchWeapon { .. } => "SwitchWeapon",
            ActionStep::Reload => "Reload",
            ActionStep::UseItem { .. } => "UseItem",
            ActionStep::DropItem { .. } => "DropItem",
            ActionStep::CallReinforcements { .. } => "CallReinforcements",
            ActionStep::MarkTarget { .. } => "MarkTarget",
            ActionStep::RequestCover { .. } => "RequestCover",
            ActionStep::CoordinateAttack { .. } => "CoordinateAttack",
            ActionStep::SetAmbush { .. } => "SetAmbush",
            ActionStep::Distract { .. } => "Distract",
            ActionStep::Regroup { .. } => "Regroup",
            ActionStep::Scan { .. } => "Scan",
            ActionStep::Wait { .. } => "Wait",
            ActionStep::Interact { .. } => "Interact",
            ActionStep::UseAbility { .. } => "UseAbility",
            ActionStep::Taunt { .. } => "Taunt",
            ActionStep::ModifyTerrain { .. } => "ModifyTerrain",
            _ => "Unknown",
        }
    }

    /// Validate a batch of actions
    pub fn validate_actions<F>(
        &self,
        actions: &[ActionStep],
        validator: &F,
    ) -> Vec<ValidationResult>
    where
        F: Fn(&ActionStep) -> bool,
    {
        actions
            .iter()
            .map(|action| self.validate_action(action, validator))
            .collect()
    }

    /// Check if all actions in a batch are valid
    pub fn all_valid<F>(&self, actions: &[ActionStep], validator: &F) -> bool
    where
        F: Fn(&ActionStep) -> bool,
    {
        self.validate_actions(actions, validator)
            .iter()
            .all(|r| r.is_valid())
    }

    /// Log a validation result to the audit trail
    fn log_validation(&self, action: &ActionStep, result: &ValidationResult) {
        let action_type = Self::action_name(action).to_string();

        let entry = AuditEntry {
            timestamp: time::OffsetDateTime::now_utc(),
            action_type,
            result: match result {
                ValidationResult::Valid => "valid".to_string(),
                ValidationResult::Invalid { .. } => "invalid".to_string(),
                ValidationResult::Denied { .. } => "denied".to_string(),
            },
            reason: result.reason().map(|s| s.to_string()),
        };

        let id = uuid::Uuid::new_v4();
        self.audit_log.insert(id, entry);

        // Trim log if too large
        if self.audit_log.len() > self.max_audit_entries {
            // Remove oldest entries (simple approach: remove random entries)
            // In production, use a proper ring buffer or time-based eviction
            let keys: Vec<_> = self.audit_log.iter().take(100).map(|e| *e.key()).collect();
            for key in keys {
                self.audit_log.remove(&key);
            }
        }
    }

    /// Get recent audit entries
    pub fn get_audit_log(&self, limit: usize) -> Vec<AuditEntry> {
        self.audit_log
            .iter()
            .take(limit)
            .map(|e| e.value().clone())
            .collect()
    }

    /// Get statistics about validation results
    pub fn get_stats(&self) -> ValidationStats {
        let mut stats = ValidationStats::default();

        for entry in self.audit_log.iter() {
            match entry.result.as_str() {
                "valid" => stats.valid += 1,
                "invalid" => stats.invalid += 1,
                "denied" => stats.denied += 1,
                _ => {}
            }
        }

        stats
    }

    /// Clear the audit log
    pub fn clear_audit_log(&self) {
        self.audit_log.clear();
    }
}

/// Statistics about validation results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationStats {
    pub valid: usize,
    pub invalid: usize,
    pub denied: usize,
}

impl ValidationStats {
    pub fn total(&self) -> usize {
        self.valid + self.invalid + self.denied
    }

    pub fn rejection_rate(&self) -> f64 {
        if self.total() == 0 {
            0.0
        } else {
            (self.invalid + self.denied) as f64 / self.total() as f64
        }
    }
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_guard_basic() {
        let guard = ToolGuard::new();

        let action = ActionStep::MoveTo {
            x: 5,
            y: 10,
            speed: None,
        };

        let result = guard.validate_action(&action, &|_| true);
        assert!(result.is_valid());
    }

    #[test]
    fn test_denied_action() {
        let guard = ToolGuard::new();
        guard.set_policy("ExecuteCode", ToolPolicy::Denied);

        // Since ActionStep doesn't have an ExecuteCode variant,
        // we'll test with a regular action first, then override its policy
        let action = ActionStep::CoverFire {
            target_id: 99,
            duration: 2.0,
        };

        // Set CoverFire as denied for this test
        guard.set_policy("CoverFire", ToolPolicy::Denied);

        let result = guard.validate_action(&action, &|_| true);
        assert!(!result.is_valid());
        assert!(matches!(result, ValidationResult::Denied { .. }));
    }

    #[test]
    fn test_restricted_action_valid() {
        let guard = ToolGuard::new();

        let action = ActionStep::CoverFire {
            target_id: 99,
            duration: 2.0,
        };

        // Validator says it's valid
        let result = guard.validate_action(&action, &|_| true);
        assert!(result.is_valid());
    }

    #[test]
    fn test_restricted_action_invalid() {
        let guard = ToolGuard::new();

        let action = ActionStep::CoverFire {
            target_id: 99,
            duration: 2.0,
        };

        // Validator says it's invalid (e.g., target out of range)
        let result = guard.validate_action(&action, &|_| false);
        assert!(!result.is_valid());
        assert!(matches!(result, ValidationResult::Invalid { .. }));
    }

    #[test]
    fn test_batch_validation() {
        let guard = ToolGuard::new();

        let actions = vec![
            ActionStep::MoveTo {
                x: 5,
                y: 10,
                speed: None,
            },
            ActionStep::CoverFire {
                target_id: 99,
                duration: 2.0,
            },
        ];

        // All valid
        assert!(guard.all_valid(&actions, &|_| true));

        // CoverFire is invalid
        assert!(!guard.all_valid(&actions, &|a| !matches!(a, ActionStep::CoverFire { .. })));
    }

    #[test]
    fn test_audit_log() {
        let guard = ToolGuard::new();

        let action = ActionStep::CoverFire {
            target_id: 99,
            duration: 2.0,
        };

        guard.validate_action(&action, &|_| true);

        let log = guard.get_audit_log(10);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].action_type, "CoverFire");
        assert_eq!(log[0].result, "valid");
    }

    #[test]
    fn test_validation_stats() {
        let guard = ToolGuard::new();

        // Valid
        guard.validate_action(
            &ActionStep::MoveTo {
                x: 5,
                y: 10,
                speed: None,
            },
            &|_| true,
        );

        // Invalid
        guard.validate_action(
            &ActionStep::CoverFire {
                target_id: 99,
                duration: 2.0,
            },
            &|_| false,
        );

        // Denied
        guard.set_policy("Revive", ToolPolicy::Denied);
        guard.validate_action(&ActionStep::Revive { ally_id: 42 }, &|_| true);

        let stats = guard.get_stats();
        assert_eq!(stats.valid, 1);
        assert_eq!(stats.invalid, 1);
        assert_eq!(stats.denied, 1);
        assert_eq!(stats.total(), 3);
        assert!((stats.rejection_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_custom_policy() {
        let guard = ToolGuard::new();
        guard.set_policy("MoveTo", ToolPolicy::Allowed);

        let action = ActionStep::MoveTo {
            x: 5,
            y: 10,
            speed: None,
        };

        let result = guard.validate_action(&action, &|_| false); // Validator says no
        assert!(result.is_valid()); // But policy says yes (Allowed overrides validator)
    }

    // ============================================================
    // ValidationResult Tests
    // ============================================================

    #[test]
    fn test_validation_result_is_valid() {
        assert!(ValidationResult::Valid.is_valid());

        assert!(!ValidationResult::Invalid {
            reason: "test".to_string(),
        }
        .is_valid());

        assert!(!ValidationResult::Denied {
            action: "test".to_string(),
        }
        .is_valid());
    }

    #[test]
    fn test_validation_result_reason() {
        assert!(ValidationResult::Valid.reason().is_none());

        let invalid = ValidationResult::Invalid {
            reason: "test error".to_string(),
        };
        assert_eq!(invalid.reason(), Some("test error"));

        let denied = ValidationResult::Denied {
            action: "Forbidden".to_string(),
        };
        assert_eq!(denied.reason(), Some("Forbidden"));
    }

    // ============================================================
    // ToolPolicy Tests
    // ============================================================

    #[test]
    fn test_tool_policy_serialization() {
        let allowed = ToolPolicy::Allowed;
        let restricted = ToolPolicy::Restricted;
        let denied = ToolPolicy::Denied;

        assert_eq!(serde_json::to_string(&allowed).unwrap(), "\"Allowed\"");
        assert_eq!(
            serde_json::to_string(&restricted).unwrap(),
            "\"Restricted\""
        );
        assert_eq!(serde_json::to_string(&denied).unwrap(), "\"Denied\"");
    }

    #[test]
    fn test_tool_policy_deserialization() {
        let allowed: ToolPolicy = serde_json::from_str("\"Allowed\"").unwrap();
        let restricted: ToolPolicy = serde_json::from_str("\"Restricted\"").unwrap();
        let denied: ToolPolicy = serde_json::from_str("\"Denied\"").unwrap();

        assert_eq!(allowed, ToolPolicy::Allowed);
        assert_eq!(restricted, ToolPolicy::Restricted);
        assert_eq!(denied, ToolPolicy::Denied);
    }

    #[test]
    fn test_tool_policy_equality() {
        assert_eq!(ToolPolicy::Allowed, ToolPolicy::Allowed);
        assert_eq!(ToolPolicy::Restricted, ToolPolicy::Restricted);
        assert_eq!(ToolPolicy::Denied, ToolPolicy::Denied);
        assert_ne!(ToolPolicy::Allowed, ToolPolicy::Denied);
    }

    #[test]
    fn test_tool_policy_copy() {
        let policy = ToolPolicy::Allowed;
        let copied = policy; // Copy
        assert_eq!(policy, copied);
    }

    // ============================================================
    // AuditEntry Tests
    // ============================================================

    #[test]
    fn test_audit_entry_serialization() {
        let entry = AuditEntry {
            timestamp: time::OffsetDateTime::now_utc(),
            action_type: "MoveTo".to_string(),
            result: "valid".to_string(),
            reason: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("MoveTo"));
        assert!(json.contains("valid"));
    }

    #[test]
    fn test_audit_entry_with_reason() {
        let entry = AuditEntry {
            timestamp: time::OffsetDateTime::now_utc(),
            action_type: "Attack".to_string(),
            result: "invalid".to_string(),
            reason: Some("Target out of range".to_string()),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("Target out of range"));
    }

    #[test]
    fn test_audit_entry_clone() {
        let entry = AuditEntry {
            timestamp: time::OffsetDateTime::now_utc(),
            action_type: "Wait".to_string(),
            result: "valid".to_string(),
            reason: None,
        };

        let cloned = entry.clone();
        assert_eq!(entry.action_type, cloned.action_type);
        assert_eq!(entry.result, cloned.result);
    }

    // ============================================================
    // Default Policy Tests
    // ============================================================

    #[test]
    fn test_default_policy() {
        let guard = ToolGuard::new();

        // Unknown action should use default policy (Restricted)
        let unknown_policy = guard.get_policy("UnknownAction");
        assert_eq!(unknown_policy, ToolPolicy::Restricted);
    }

    #[test]
    fn test_set_default_policy() {
        let mut guard = ToolGuard::new();
        guard.set_default_policy(ToolPolicy::Denied);

        let unknown_policy = guard.get_policy("UnknownAction");
        assert_eq!(unknown_policy, ToolPolicy::Denied);
    }

    // ============================================================
    // Clear Audit Log Test
    // ============================================================

    #[test]
    fn test_clear_audit_log() {
        let guard = ToolGuard::new();

        // Add some entries
        guard.validate_action(
            &ActionStep::MoveTo {
                x: 0,
                y: 0,
                speed: None,
            },
            &|_| true,
        );
        guard.validate_action(&ActionStep::Wait { duration: 1.0 }, &|_| true);

        // Verify entries exist
        let log_before = guard.get_audit_log(10);
        assert_eq!(log_before.len(), 2);

        // Clear log
        guard.clear_audit_log();

        // Verify empty
        let log_after = guard.get_audit_log(10);
        assert_eq!(log_after.len(), 0);
    }

    // ============================================================
    // ValidationStats Tests
    // ============================================================

    #[test]
    fn test_validation_stats_default() {
        let stats = ValidationStats::default();
        assert_eq!(stats.valid, 0);
        assert_eq!(stats.invalid, 0);
        assert_eq!(stats.denied, 0);
    }

    #[test]
    fn test_validation_stats_total() {
        let stats = ValidationStats {
            valid: 10,
            invalid: 3,
            denied: 2,
        };
        assert_eq!(stats.total(), 15);
    }

    #[test]
    fn test_validation_stats_rejection_rate() {
        let stats = ValidationStats {
            valid: 8,
            invalid: 1,
            denied: 1,
        };
        assert!((stats.rejection_rate() - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_validation_stats_rejection_rate_zero_total() {
        let stats = ValidationStats::default();
        assert_eq!(stats.rejection_rate(), 0.0);
    }

    #[test]
    fn test_validation_stats_serialization() {
        let stats = ValidationStats {
            valid: 100,
            invalid: 5,
            denied: 2,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let parsed: ValidationStats = serde_json::from_str(&json).unwrap();

        assert_eq!(stats.valid, parsed.valid);
        assert_eq!(stats.invalid, parsed.invalid);
        assert_eq!(stats.denied, parsed.denied);
    }

    // ============================================================
    // Action Name Coverage Tests
    // ============================================================

    #[test]
    fn test_action_name_all_variants() {
        use astraweave_core::IVec2;
        // Test a variety of action types to ensure action_name() handles them
        let actions: Vec<ActionStep> = vec![
            ActionStep::Approach {
                target_id: 1,
                distance: 5.0,
            },
            ActionStep::Retreat {
                target_id: 1,
                distance: 10.0,
            },
            ActionStep::TakeCover { position: None },
            ActionStep::Strafe {
                target_id: 1,
                direction: astraweave_core::StrafeDirection::Left,
            },
            ActionStep::Patrol { waypoints: vec![] },
            ActionStep::Attack { target_id: 1 },
            ActionStep::AimedShot { target_id: 1 },
            ActionStep::QuickAttack { target_id: 1 },
            ActionStep::HeavyAttack { target_id: 1 },
            ActionStep::AoEAttack {
                x: 0,
                y: 0,
                radius: 5.0,
            },
            ActionStep::ThrowExplosive { x: 0, y: 0 },
            ActionStep::Charge { target_id: 1 },
            ActionStep::Block,
            ActionStep::Dodge { direction: None },
            ActionStep::Parry,
            ActionStep::ThrowSmoke { x: 0, y: 0 },
            ActionStep::Heal { target_id: Some(1) },
            ActionStep::UseDefensiveAbility {
                ability_name: "shield".to_string(),
            },
            ActionStep::EquipWeapon {
                weapon_name: "sword".to_string(),
            },
            ActionStep::SwitchWeapon { slot: 1 },
            ActionStep::Reload,
            ActionStep::UseItem {
                item_name: "potion".to_string(),
            },
            ActionStep::DropItem {
                item_name: "junk".to_string(),
            },
            ActionStep::CallReinforcements { count: 3 },
            ActionStep::MarkTarget { target_id: 1 },
            ActionStep::RequestCover { duration: 5.0 },
            ActionStep::CoordinateAttack { target_id: 1 },
            ActionStep::SetAmbush {
                position: IVec2 { x: 10, y: 10 },
            },
            ActionStep::Distract { target_id: 1 },
            ActionStep::Regroup {
                rally_point: IVec2 { x: 5, y: 5 },
            },
            ActionStep::Scan { radius: 15.0 },
            ActionStep::Wait { duration: 2.0 },
            ActionStep::Interact { target_id: 1 },
            ActionStep::UseAbility {
                ability_name: "fireball".to_string(),
            },
            ActionStep::Taunt { target_id: 1 },
        ];

        let guard = ToolGuard::new();
        for action in actions {
            let _ = guard.validate_action(&action, &|_| true);
        }

        // Verify we logged all actions
        let log = guard.get_audit_log(50);
        assert!(log.len() >= 30, "Should have logged many actions");
    }

    // ============================================================
    // Edge Cases
    // ============================================================

    #[test]
    fn test_validate_empty_batch() {
        let guard = ToolGuard::new();
        let results = guard.validate_actions(&[], &|_| true);
        assert!(results.is_empty());
    }

    #[test]
    fn test_all_valid_empty_batch() {
        let guard = ToolGuard::new();
        let result = guard.all_valid(&[], &|_| true);
        assert!(result); // Empty batch is considered all valid
    }

    #[test]
    fn test_default_trait_impl() {
        let guard = ToolGuard::default();
        assert_eq!(guard.get_policy("Wait"), ToolPolicy::Allowed);
    }
}
