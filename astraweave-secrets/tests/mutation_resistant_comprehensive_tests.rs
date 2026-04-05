//! Mutation-resistant comprehensive tests for astraweave-secrets.

use anyhow::Result;
use astraweave_secrets::*;
use std::collections::HashMap;
use std::sync::RwLock;

// ═══════════════════════════════════════════════════════════════════════════
// Test mock backend (internal MockBackend is #[cfg(test)] only)
// ═══════════════════════════════════════════════════════════════════════════

struct TestBackend {
    store: RwLock<HashMap<String, Vec<u8>>>,
}

impl TestBackend {
    fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }
}

impl SecretBackend for TestBackend {
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

// ═══════════════════════════════════════════════════════════════════════════
// SecretValue construction & accessors
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn secret_value_new_empty() {
    let sv = SecretValue::new(vec![]);
    assert_eq!(sv.as_bytes(), &[]);
}

#[test]
fn secret_value_new_preserves_data() {
    let data = vec![1, 2, 3, 4, 5];
    let sv = SecretValue::new(data.clone());
    assert_eq!(sv.as_bytes(), &[1, 2, 3, 4, 5]);
}

#[test]
fn secret_value_new_large() {
    let data = vec![0xAB; 10_000];
    let sv = SecretValue::new(data);
    assert_eq!(sv.as_bytes().len(), 10_000);
    assert!(sv.as_bytes().iter().all(|&b| b == 0xAB));
}

#[test]
fn secret_value_from_str_hello() {
    let sv = SecretValue::from_str("hello");
    assert_eq!(sv.as_bytes(), b"hello");
}

#[test]
fn secret_value_from_str_empty() {
    let sv = SecretValue::from_str("");
    assert_eq!(sv.as_bytes(), &[]);
}

#[test]
fn secret_value_from_str_unicode() {
    let sv = SecretValue::from_str("héllo wörld 🌍");
    assert_eq!(sv.as_str().unwrap(), "héllo wörld 🌍");
}

#[test]
fn secret_value_as_str_valid_utf8() {
    let sv = SecretValue::from_str("test_secret_123");
    let s = sv.as_str().unwrap();
    assert_eq!(s, "test_secret_123");
}

#[test]
fn secret_value_as_str_invalid_utf8() {
    let sv = SecretValue::new(vec![0xFF, 0xFE, 0xFD]);
    let result = sv.as_str();
    assert!(result.is_err(), "Invalid UTF-8 should return error");
}

#[test]
fn secret_value_as_str_empty_is_ok() {
    let sv = SecretValue::new(vec![]);
    assert_eq!(sv.as_str().unwrap(), "");
}

#[test]
fn secret_value_as_bytes_exact() {
    let sv = SecretValue::new(vec![10, 20, 30]);
    let bytes = sv.as_bytes();
    assert_eq!(bytes.len(), 3);
    assert_eq!(bytes[0], 10);
    assert_eq!(bytes[1], 20);
    assert_eq!(bytes[2], 30);
}

#[test]
fn secret_value_from_str_roundtrip() {
    let original = "my_secret_password_42!@#";
    let sv = SecretValue::from_str(original);
    assert_eq!(sv.as_str().unwrap(), original);
    assert_eq!(sv.as_bytes(), original.as_bytes());
}

#[test]
fn secret_value_from_str_whitespace() {
    let sv = SecretValue::from_str("  spaces  ");
    assert_eq!(sv.as_str().unwrap(), "  spaces  ");
    assert_eq!(sv.as_bytes().len(), 10);
}

#[test]
fn secret_value_from_str_newlines() {
    let sv = SecretValue::from_str("line1\nline2\n");
    assert_eq!(sv.as_str().unwrap(), "line1\nline2\n");
}

#[test]
fn secret_value_binary_data_len() {
    let data: Vec<u8> = (0..=255).collect();
    let sv = SecretValue::new(data);
    assert_eq!(sv.as_bytes().len(), 256);
}

// ═══════════════════════════════════════════════════════════════════════════
// SecretValue drop (zeroize) behavior
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn secret_value_drops_without_panic() {
    let sv = SecretValue::from_str("sensitive data");
    drop(sv);
    // If zeroize panics, this test fails
}

#[test]
fn secret_value_empty_drops_without_panic() {
    let sv = SecretValue::new(vec![]);
    drop(sv);
}

// ═══════════════════════════════════════════════════════════════════════════
// KeyringBackend construction
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn keyring_backend_new() {
    let _kb = KeyringBackend::new();
    // Should not panic
}

#[test]
#[allow(clippy::default_constructed_unit_structs)]
fn keyring_backend_default() {
    let _kb = KeyringBackend::default();
    // Should not panic — derived Default
}

#[test]
fn keyring_backend_list_keys_always_empty() {
    let kb = KeyringBackend::new();
    let keys = kb.list_keys().unwrap();
    assert!(keys.is_empty(), "list_keys() stub always returns empty vec");
}

// ═══════════════════════════════════════════════════════════════════════════
// SecretManager with custom backend
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn manager_with_backend_set_and_get() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    mgr.set("key1", SecretValue::from_str("value1")).unwrap();
    let retrieved = mgr.get("key1").unwrap();
    assert_eq!(retrieved.as_str().unwrap(), "value1");
}

#[test]
fn manager_get_nonexistent_errors() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    let result = mgr.get("nonexistent");
    assert!(result.is_err());
}

#[test]
fn manager_set_overwrite() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    mgr.set("k", SecretValue::from_str("v1")).unwrap();
    mgr.set("k", SecretValue::from_str("v2")).unwrap();
    assert_eq!(mgr.get("k").unwrap().as_str().unwrap(), "v2");
}

#[test]
fn manager_delete_existing() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    mgr.set("to_del", SecretValue::from_str("val")).unwrap();
    assert!(mgr.delete("to_del").is_ok());
    assert!(mgr.get("to_del").is_err());
}

#[test]
fn manager_delete_nonexistent_errors() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    let result = mgr.delete("nonexistent");
    assert!(result.is_err());
}

#[test]
fn manager_multiple_keys() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    mgr.set("a", SecretValue::from_str("alpha")).unwrap();
    mgr.set("b", SecretValue::from_str("bravo")).unwrap();
    mgr.set("c", SecretValue::from_str("charlie")).unwrap();
    assert_eq!(mgr.get("a").unwrap().as_str().unwrap(), "alpha");
    assert_eq!(mgr.get("b").unwrap().as_str().unwrap(), "bravo");
    assert_eq!(mgr.get("c").unwrap().as_str().unwrap(), "charlie");
}

#[test]
fn manager_set_empty_value() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    mgr.set("empty", SecretValue::from_str("")).unwrap();
    assert_eq!(mgr.get("empty").unwrap().as_str().unwrap(), "");
}

#[test]
fn manager_set_binary_value() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    let binary = SecretValue::new(vec![0, 1, 2, 255, 254, 253]);
    mgr.set("binary", binary).unwrap();
    let retrieved = mgr.get("binary").unwrap();
    assert_eq!(retrieved.as_bytes(), &[0, 1, 2, 255, 254, 253]);
}

#[test]
fn manager_set_and_delete_then_set_again() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    mgr.set("reuse", SecretValue::from_str("first")).unwrap();
    mgr.delete("reuse").unwrap();
    mgr.set("reuse", SecretValue::from_str("second")).unwrap();
    assert_eq!(mgr.get("reuse").unwrap().as_str().unwrap(), "second");
}

#[test]
fn manager_long_key_name() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    let long_key = "a".repeat(1000);
    mgr.set(&long_key, SecretValue::from_str("long_key_value"))
        .unwrap();
    assert_eq!(
        mgr.get(&long_key).unwrap().as_str().unwrap(),
        "long_key_value"
    );
}

#[test]
fn manager_special_chars_in_key() {
    let backend = std::sync::Arc::new(TestBackend::new());
    let mgr = SecretManager::with_backend(backend);
    let key = "key/with:special!@#chars";
    mgr.set(key, SecretValue::from_str("val")).unwrap();
    assert_eq!(mgr.get(key).unwrap().as_str().unwrap(), "val");
}

// ═══════════════════════════════════════════════════════════════════════════
// SecretManager::global() exists and doesn't panic
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn manager_global_exists() {
    let _mgr = SecretManager::global();
    // Should not panic — Lazy initialization
}

// ═══════════════════════════════════════════════════════════════════════════
// Boundary & edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn secret_value_single_byte() {
    let sv = SecretValue::new(vec![42]);
    assert_eq!(sv.as_bytes().len(), 1);
    assert_eq!(sv.as_bytes()[0], 42);
}

#[test]
fn secret_value_single_char_str() {
    let sv = SecretValue::from_str("X");
    assert_eq!(sv.as_str().unwrap(), "X");
    assert_eq!(sv.as_bytes(), b"X");
}

#[test]
fn secret_value_null_bytes() {
    let sv = SecretValue::new(vec![0, 0, 0]);
    assert_eq!(sv.as_bytes(), &[0, 0, 0]);
    // Null bytes are valid as bytes
    assert_eq!(sv.as_str().unwrap(), "\0\0\0");
}

#[test]
fn secret_value_mixed_ascii_binary() {
    let mut data = b"hello".to_vec();
    data.push(0xFF); // Invalid UTF-8 continuation
    let sv = SecretValue::new(data);
    assert_eq!(sv.as_bytes().len(), 6);
    assert!(sv.as_str().is_err());
}
