use anyhow::Result;
use zeroize::Zeroize;

pub struct SecretValue(Vec<u8>);

impl SecretValue {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    #[allow(clippy::should_implement_trait)] // This is not the std::str::FromStr trait
    pub fn from_str(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    pub fn as_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.0).map_err(|e| anyhow::anyhow!(e))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SecretValue {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

pub trait SecretBackend: Send + Sync {
    fn get(&self, key: &str) -> Result<SecretValue>;
    fn set(&self, key: &str, value: SecretValue) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
    fn list_keys(&self) -> Result<Vec<String>>;
}

/// In-memory mock backend for testing
#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    pub struct MockBackend {
        store: RwLock<HashMap<String, Vec<u8>>>,
    }

    impl MockBackend {
        pub fn new() -> Self {
            Self {
                store: RwLock::new(HashMap::new()),
            }
        }
    }

    impl Default for MockBackend {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SecretBackend for MockBackend {
        fn get(&self, key: &str) -> Result<SecretValue> {
            let store = self.store.read().unwrap();
            store
                .get(key)
                .cloned()
                .map(SecretValue::new)
                .ok_or_else(|| anyhow::anyhow!("Key not found: {}", key))
        }

        fn set(&self, key: &str, value: SecretValue) -> Result<()> {
            let mut store = self.store.write().unwrap();
            store.insert(key.to_string(), value.as_bytes().to_vec());
            Ok(())
        }

        fn delete(&self, key: &str) -> Result<()> {
            let mut store = self.store.write().unwrap();
            if store.remove(key).is_some() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Key not found: {}", key))
            }
        }

        fn list_keys(&self) -> Result<Vec<String>> {
            let store = self.store.read().unwrap();
            Ok(store.keys().cloned().collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_value_new() {
        let data = vec![1, 2, 3, 4];
        let secret = SecretValue::new(data.clone());
        assert_eq!(secret.as_bytes(), &data);
    }

    #[test]
    fn test_secret_value_from_str() {
        let secret = SecretValue::from_str("hello");
        assert_eq!(secret.as_str().unwrap(), "hello");
        assert_eq!(secret.as_bytes(), b"hello");
    }

    #[test]
    fn test_secret_value_as_str_valid_utf8() {
        let secret = SecretValue::from_str("valid utf8 string");
        assert!(secret.as_str().is_ok());
        assert_eq!(secret.as_str().unwrap(), "valid utf8 string");
    }

    #[test]
    fn test_secret_value_as_str_invalid_utf8() {
        let invalid = vec![0xff, 0xfe, 0x00, 0x01];
        let secret = SecretValue::new(invalid);
        assert!(secret.as_str().is_err());
    }

    #[test]
    fn test_secret_value_empty() {
        let secret = SecretValue::new(vec![]);
        assert!(secret.as_bytes().is_empty());
        assert_eq!(secret.as_str().unwrap(), "");
    }

    #[test]
    fn test_mock_backend_set_and_get() {
        let backend = mock::MockBackend::new();
        let secret = SecretValue::from_str("my_password");
        backend.set("api_key", secret).unwrap();

        let retrieved = backend.get("api_key").unwrap();
        assert_eq!(retrieved.as_str().unwrap(), "my_password");
    }

    #[test]
    fn test_mock_backend_get_nonexistent() {
        let backend = mock::MockBackend::new();
        let result = backend.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_backend_delete() {
        let backend = mock::MockBackend::new();
        let secret = SecretValue::from_str("to_delete");
        backend.set("temp_key", secret).unwrap();

        assert!(backend.delete("temp_key").is_ok());
        assert!(backend.get("temp_key").is_err());
    }

    #[test]
    fn test_mock_backend_delete_nonexistent() {
        let backend = mock::MockBackend::new();
        let result = backend.delete("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_backend_list_keys() {
        let backend = mock::MockBackend::new();
        backend.set("key1", SecretValue::from_str("val1")).unwrap();
        backend.set("key2", SecretValue::from_str("val2")).unwrap();
        backend.set("key3", SecretValue::from_str("val3")).unwrap();

        let keys = backend.list_keys().unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));
    }

    #[test]
    fn test_mock_backend_list_keys_empty() {
        let backend = mock::MockBackend::new();
        let keys = backend.list_keys().unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_mock_backend_overwrite() {
        let backend = mock::MockBackend::new();
        backend.set("key", SecretValue::from_str("first")).unwrap();
        backend.set("key", SecretValue::from_str("second")).unwrap();

        let retrieved = backend.get("key").unwrap();
        assert_eq!(retrieved.as_str().unwrap(), "second");
    }

    #[test]
    fn test_mock_backend_default() {
        let backend = mock::MockBackend::default();
        assert!(backend.list_keys().unwrap().is_empty());
    }
}
