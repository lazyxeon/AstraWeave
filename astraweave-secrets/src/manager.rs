use super::backend::SecretBackend;
use super::keyring_backend::KeyringBackend;
use once_cell::sync::Lazy;
use std::sync::Arc;

pub struct SecretManager {
    backend: Arc<dyn SecretBackend>,
}

static GLOBAL: Lazy<SecretManager> = Lazy::new(|| SecretManager {
    backend: Arc::new(KeyringBackend::new()),
});

impl SecretManager {
    pub fn global() -> &'static Self {
        &GLOBAL
    }

    /// Create a new SecretManager with a custom backend (useful for testing)
    pub fn with_backend(backend: Arc<dyn SecretBackend>) -> Self {
        Self { backend }
    }

    pub fn get(&self, key: &str) -> anyhow::Result<super::backend::SecretValue> {
        self.backend.get(key)
    }

    pub fn set(&self, key: &str, value: super::backend::SecretValue) -> anyhow::Result<()> {
        self.backend.set(key, value)
    }

    pub fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.backend.delete(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::mock::MockBackend;
    use crate::backend::SecretValue;

    #[test]
    fn test_manager_with_mock_backend() {
        let mock = Arc::new(MockBackend::new());
        let manager = SecretManager::with_backend(mock);

        // Set a secret
        manager
            .set("test_key", SecretValue::from_str("test_value"))
            .unwrap();

        // Get the secret
        let retrieved = manager.get("test_key").unwrap();
        assert_eq!(retrieved.as_str().unwrap(), "test_value");
    }

    #[test]
    fn test_manager_get_nonexistent() {
        let mock = Arc::new(MockBackend::new());
        let manager = SecretManager::with_backend(mock);

        let result = manager.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_delete() {
        let mock = Arc::new(MockBackend::new());
        let manager = SecretManager::with_backend(mock);

        manager
            .set("to_delete", SecretValue::from_str("value"))
            .unwrap();
        assert!(manager.delete("to_delete").is_ok());
        assert!(manager.get("to_delete").is_err());
    }

    #[test]
    fn test_manager_delete_nonexistent() {
        let mock = Arc::new(MockBackend::new());
        let manager = SecretManager::with_backend(mock);

        let result = manager.delete("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_overwrite() {
        let mock = Arc::new(MockBackend::new());
        let manager = SecretManager::with_backend(mock);

        manager.set("key", SecretValue::from_str("first")).unwrap();
        manager.set("key", SecretValue::from_str("second")).unwrap();

        let retrieved = manager.get("key").unwrap();
        assert_eq!(retrieved.as_str().unwrap(), "second");
    }

    #[test]
    fn test_global_exists() {
        // Just verify global() returns something without panicking
        // We can't actually use it in tests since it uses system keyring
        let _ = SecretManager::global();
    }
}
