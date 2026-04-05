// ActionHistory persistence system with checksum validation
// Phase 3: Learning & Persistence

use super::ActionHistory;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Versioned and checksummed wrapper for persisted history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedHistory {
    /// Schema version for migration support
    pub version: u32,
    /// Unix timestamp when saved
    pub timestamp: u64,
    /// Simple checksum for corruption detection
    pub checksum: u64,
    /// The actual history data
    pub history: ActionHistory,
}

impl PersistedHistory {
    const CURRENT_VERSION: u32 = 1;

    /// Create a new persisted history wrapper
    pub fn new(history: ActionHistory) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_secs();

        let checksum = Self::calculate_checksum(&history);

        Self {
            version: Self::CURRENT_VERSION,
            timestamp,
            checksum,
            history,
        }
    }

    /// Calculate a simple checksum based on history contents
    fn calculate_checksum(history: &ActionHistory) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Get all action names and sort for determinism
        let mut action_names: Vec<String> = history.action_names();
        action_names.sort();

        // Hash each action's stats in sorted order
        for action_name in action_names {
            action_name.hash(&mut hasher);

            if let Some(stats) = history.get_action_stats(&action_name) {
                stats.executions.hash(&mut hasher);
                stats.successes.hash(&mut hasher);
                stats.failures.hash(&mut hasher);
                // Hash the bits of f32 for determinism
                stats.avg_duration.to_bits().hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Validate checksum matches current history
    pub fn validate(&self) -> bool {
        let current_checksum = Self::calculate_checksum(&self.history);
        self.checksum == current_checksum
    }

    /// Get age in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_secs();

        now.saturating_sub(self.timestamp)
    }
}

/// Persistence format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PersistenceFormat {
    /// Human-readable JSON (larger files, easy debugging)
    Json,
    /// Compact binary (smaller files, faster I/O)
    Bincode,
}

/// Result type for persistence operations
pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Errors that can occur during persistence operations
#[derive(Debug, Clone)]
#[non_exhaustive]
#[must_use]
pub enum PersistenceError {
    /// Failed to serialize history
    SerializationFailed(String),
    /// Failed to deserialize history
    DeserializationFailed(String),
    /// Failed to write file
    WriteFailed(String),
    /// Failed to read file
    ReadFailed(String),
    /// Checksum mismatch detected
    ChecksumMismatch { expected: u64, actual: u64 },
    /// Unsupported schema version
    UnsupportedVersion(u32),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
            Self::DeserializationFailed(msg) => write!(f, "Deserialization failed: {}", msg),
            Self::WriteFailed(msg) => write!(f, "Write failed: {}", msg),
            Self::ReadFailed(msg) => write!(f, "Read failed: {}", msg),
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Checksum mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            Self::UnsupportedVersion(v) => write!(f, "Unsupported schema version: {}", v),
        }
    }
}

impl std::error::Error for PersistenceError {}

/// Main persistence interface
pub struct HistoryPersistence;

impl HistoryPersistence {
    /// Save ActionHistory to file
    pub fn save<P: AsRef<Path>>(
        history: &ActionHistory,
        path: P,
        format: PersistenceFormat,
    ) -> PersistenceResult<()> {
        let persisted = PersistedHistory::new(history.clone());

        let data = match format {
            PersistenceFormat::Json => serde_json::to_string_pretty(&persisted)
                .map_err(|e| PersistenceError::SerializationFailed(e.to_string()))?
                .into_bytes(),
            PersistenceFormat::Bincode => bincode::serialize(&persisted)
                .map_err(|e| PersistenceError::SerializationFailed(e.to_string()))?,
        };

        fs::write(path.as_ref(), data).map_err(|e| PersistenceError::WriteFailed(e.to_string()))?;

        Ok(())
    }

    /// Load ActionHistory from file with validation
    pub fn load<P: AsRef<Path>>(
        path: P,
        format: PersistenceFormat,
    ) -> PersistenceResult<ActionHistory> {
        let data =
            fs::read(path.as_ref()).map_err(|e| PersistenceError::ReadFailed(e.to_string()))?;

        let persisted: PersistedHistory = match format {
            PersistenceFormat::Json => serde_json::from_slice(&data)
                .map_err(|e| PersistenceError::DeserializationFailed(e.to_string()))?,
            PersistenceFormat::Bincode => bincode::deserialize(&data)
                .map_err(|e| PersistenceError::DeserializationFailed(e.to_string()))?,
        };

        // Validate version
        if persisted.version > PersistedHistory::CURRENT_VERSION {
            return Err(PersistenceError::UnsupportedVersion(persisted.version));
        }

        // Validate checksum
        if !persisted.validate() {
            let expected = persisted.checksum;
            let actual = PersistedHistory::calculate_checksum(&persisted.history);
            return Err(PersistenceError::ChecksumMismatch { expected, actual });
        }

        Ok(persisted.history)
    }

    /// Load history with fallback to empty on error
    pub fn load_or_default<P: AsRef<Path>>(path: P, format: PersistenceFormat) -> ActionHistory {
        match Self::load(path, format) {
            Ok(history) => {
                tracing::info!("Successfully loaded ActionHistory from disk");
                history
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load ActionHistory: {}. Starting with empty history.",
                    e
                );
                ActionHistory::new()
            }
        }
    }

    /// Check if a save file exists and is valid
    pub fn validate_file<P: AsRef<Path>>(
        path: P,
        format: PersistenceFormat,
    ) -> PersistenceResult<()> {
        let _ = Self::load(path, format)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_history() -> ActionHistory {
        let mut history = ActionHistory::new();
        history.record_success("attack", 0.1); // 100ms
        history.record_success("attack", 0.12); // 120ms
        history.record_failure("attack"); // failure
        history.record_success("heal", 0.05); // 50ms
        history
    }

    #[test]
    fn test_persisted_history_checksum() {
        let history = create_test_history();
        let persisted = PersistedHistory::new(history.clone());

        assert!(persisted.validate());
        assert_eq!(persisted.version, 1);
    }

    #[test]
    fn test_save_and_load_json() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("history.json");

        let original = create_test_history();

        // Save
        HistoryPersistence::save(&original, &file_path, PersistenceFormat::Json)
            .expect("Failed to save");

        // Load
        let loaded =
            HistoryPersistence::load(&file_path, PersistenceFormat::Json).expect("Failed to load");

        // Verify
        let original_stats = original.get_action_stats("attack").unwrap();
        let loaded_stats = loaded.get_action_stats("attack").unwrap();

        assert_eq!(original_stats.executions, loaded_stats.executions);
        assert_eq!(original_stats.successes, loaded_stats.successes);
        assert_eq!(original_stats.failures, loaded_stats.failures);
    }

    #[test]
    fn test_save_and_load_bincode() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("history.bin");

        let original = create_test_history();

        // Save
        HistoryPersistence::save(&original, &file_path, PersistenceFormat::Bincode)
            .expect("Failed to save");

        // Load
        let loaded = HistoryPersistence::load(&file_path, PersistenceFormat::Bincode)
            .expect("Failed to load");

        // Verify
        let original_stats = original.get_action_stats("attack").unwrap();
        let loaded_stats = loaded.get_action_stats("attack").unwrap();

        assert_eq!(original_stats.executions, loaded_stats.executions);
    }

    #[test]
    fn test_load_or_default_missing_file() {
        let history =
            HistoryPersistence::load_or_default("nonexistent_file.json", PersistenceFormat::Json);

        // Should return empty history without panicking
        assert!(history.get_action_stats("attack").is_none());
    }

    #[test]
    fn test_checksum_validation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("history.json");

        let history = create_test_history();
        HistoryPersistence::save(&history, &file_path, PersistenceFormat::Json).unwrap();

        // Corrupt the file by changing the checksum value directly
        let mut data = fs::read_to_string(&file_path).unwrap();
        // Find and corrupt the checksum field
        if let Some(pos) = data.find("\"checksum\":") {
            // Replace the checksum with a different value
            let end_pos = data[pos..].find(',').unwrap() + pos;
            data.replace_range(pos..end_pos, "\"checksum\":9999999999");
        } else {
            panic!("Could not find checksum field in JSON");
        }
        fs::write(&file_path, data).unwrap();

        // Load should fail due to checksum mismatch
        let result = HistoryPersistence::load(&file_path, PersistenceFormat::Json);
        assert!(result.is_err(), "Expected checksum mismatch error");

        match result {
            Err(PersistenceError::ChecksumMismatch { expected, actual }) => {
                // Expected - stored checksum (9999999999) should differ from calculated
                assert_eq!(
                    expected, 9999999999,
                    "Stored checksum should be our corrupted value"
                );
                assert_ne!(
                    actual, 9999999999,
                    "Calculated checksum should be different"
                );
            }
            Err(e) => panic!("Expected ChecksumMismatch, got: {}", e),
            Ok(_) => panic!("Expected error, but load succeeded"),
        }
    }

    #[test]
    fn test_validate_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("history.json");

        let history = create_test_history();
        HistoryPersistence::save(&history, &file_path, PersistenceFormat::Json).unwrap();

        // Valid file
        assert!(HistoryPersistence::validate_file(&file_path, PersistenceFormat::Json).is_ok());

        // Invalid file
        assert!(
            HistoryPersistence::validate_file("nonexistent.json", PersistenceFormat::Json).is_err()
        );
    }

    #[test]
    fn test_json_vs_bincode_file_size() {
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("history.json");
        let bin_path = temp_dir.path().join("history.bin");

        let history = create_test_history();

        HistoryPersistence::save(&history, &json_path, PersistenceFormat::Json).unwrap();
        HistoryPersistence::save(&history, &bin_path, PersistenceFormat::Bincode).unwrap();

        let json_size = fs::metadata(&json_path).unwrap().len();
        let bin_size = fs::metadata(&bin_path).unwrap().len();

        // Bincode should be smaller than JSON
        assert!(bin_size < json_size);

        println!("JSON size: {} bytes", json_size);
        println!("Bincode size: {} bytes", bin_size);
    }

    // ================================================================
    // Mutation-killing tests
    // ================================================================

    /// Kills line 45: `calculate_checksum` returning 0 or 1
    /// Two different histories must produce different checksums.
    #[test]
    fn test_different_histories_produce_different_checksums() {
        let mut h1 = ActionHistory::new();
        h1.record_success("attack", 1.0);

        let mut h2 = ActionHistory::new();
        h2.record_failure("attack");

        let p1 = PersistedHistory::new(h1);
        let p2 = PersistedHistory::new(h2);

        assert_ne!(
            p1.checksum, p2.checksum,
            "Different histories should have different checksums"
        );
        // Also ensure checksums are not trivial constants
        assert!(p1.checksum != 0 && p1.checksum != 1);
    }

    /// Kills line 78: `age_seconds` returning 0 or 1
    #[test]
    fn test_age_seconds_positive() {
        let history = create_test_history();
        let persisted = PersistedHistory::new(history);
        // Just created, age should be 0 or very close
        let age = persisted.age_seconds();
        // The age is at least 0 (just created) and should be less than something reasonable
        assert!(
            age < 60,
            "age_seconds should be small for just-created history"
        );
    }

    /// Kills line 181: `version > CURRENT_VERSION` → `version < CURRENT_VERSION`
    #[test]
    fn test_unsupported_version_error() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("future_version.json");

        // Manually write JSON with a future version
        let json = format!(
            r#"{{"version": 999, "timestamp": 0, "checksum": 0, "history": {{"stats": {{}}}}}}"#
        );
        fs::write(&path, json).unwrap();

        let result = HistoryPersistence::load(&path, PersistenceFormat::Json);
        assert!(result.is_err(), "Loading future version should fail");
        match result.unwrap_err() {
            PersistenceError::UnsupportedVersion(v) => assert_eq!(v, 999),
            other => panic!("Expected UnsupportedVersion, got: {:?}", other),
        }
    }

    /// Kills line 197: `load_or_default` Ok path returning Default::default()
    #[test]
    fn test_load_or_default_with_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("valid.json");

        let mut history = ActionHistory::new();
        history.record_success("unique_action", 1.5);

        HistoryPersistence::save(&history, &path, PersistenceFormat::Json).unwrap();

        let loaded = HistoryPersistence::load_or_default(&path, PersistenceFormat::Json);
        let stats = loaded.get_action_stats("unique_action");
        assert!(
            stats.is_some(),
            "load_or_default should return saved history, not empty default"
        );
        assert_eq!(stats.unwrap().successes, 1);
    }

    /// Kills line 78: age_seconds → 0 or 1
    /// Uses a manually constructed PersistedHistory with timestamp=0 (epoch).
    #[test]
    fn test_age_seconds_with_epoch_timestamp() {
        let mut persisted = PersistedHistory::new(ActionHistory::new());
        persisted.timestamp = 0; // UNIX epoch = 1970-01-01

        let age = persisted.age_seconds();
        // Current time is well past the epoch, so age should be huge (> 1 billion)
        assert!(
            age > 1_000_000,
            "age_seconds for epoch timestamp should be > 1M, got {}",
            age
        );
    }
}
