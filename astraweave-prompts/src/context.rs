//! Context management for prompt templates
//!
//! This module provides context and variable management for templates.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prompt context manager
#[derive(Debug, Clone, Default)]
pub struct PromptContext {
    /// Context variables
    variables: HashMap<String, ContextValue>,
    /// Nested contexts
    scopes: Vec<HashMap<String, ContextValue>>,
}

/// Context value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ContextValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ContextValue>),
    Object(HashMap<String, ContextValue>),
}

impl PromptContext {
    /// Create a new context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a variable in the current context
    pub fn set(&mut self, key: String, value: ContextValue) {
        self.variables.insert(key, value);
    }

    /// Get a variable from the context
    pub fn get(&self, key: &str) -> Option<&ContextValue> {
        // Check current scope first
        if let Some(value) = self.variables.get(key) {
            return Some(value);
        }

        // Check parent scopes
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(key) {
                return Some(value);
            }
        }

        None
    }

    /// Set a variable using dot notation for nested objects
    pub fn set_path(&mut self, path: &str, value: ContextValue) {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return;
        }

        let key = parts[0];
        if parts.len() == 1 {
            self.set(key.to_string(), value);
        } else {
            let entry = self
                .variables
                .entry(key.to_string())
                .or_insert_with(|| ContextValue::Object(HashMap::new()));
            entry.insert_path(&parts[1..], value);
        }
    }

    /// Push a new scope
    pub fn push_scope(&mut self) {
        let old_vars = std::mem::take(&mut self.variables);
        self.scopes.push(old_vars);
    }

    /// Pop the current scope
    pub fn pop_scope(&mut self) -> Option<HashMap<String, ContextValue>> {
        if let Some(parent_vars) = self.scopes.pop() {
            let popped_vars = std::mem::replace(&mut self.variables, parent_vars);
            Some(popped_vars)
        } else {
            None
        }
    }

    /// Merge another context into this one
    pub fn merge(&mut self, other: PromptContext) {
        // Merge scopes
        for scope in other.scopes {
            self.scopes.push(scope);
        }

        // Merge variables
        for (key, value) in other.variables {
            self.variables.insert(key, value);
        }
    }

    /// Convert context to string variables for template processing
    pub fn to_string_map(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();

        // Add variables from all scopes
        for scope in &self.scopes {
            for (key, value) in scope {
                result.insert(key.clone(), value.to_string());
            }
        }

        // Add current variables (override scope values)
        for (key, value) in &self.variables {
            result.insert(key.clone(), value.to_string());
        }

        result
    }

    /// Convert context to JSON value for Handlebars processing
    pub fn to_json(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();

        // Add variables from all scopes
        for scope in &self.scopes {
            for (key, value) in scope {
                map.insert(key.clone(), value.to_json());
            }
        }

        // Add current variables (override scope values)
        for (key, value) in &self.variables {
            map.insert(key.clone(), value.to_json());
        }

        serde_json::Value::Object(map)
    }
}

impl ContextValue {
    /// Insert a value at a nested path
    pub fn insert_path(&mut self, path: &[&str], value: ContextValue) {
        if path.is_empty() {
            *self = value;
            return;
        }

        match self {
            ContextValue::Object(map) => {
                let key = path[0];
                if path.len() == 1 {
                    map.insert(key.to_string(), value);
                } else {
                    let entry = map
                        .entry(key.to_string())
                        .or_insert_with(|| ContextValue::Object(HashMap::new()));
                    entry.insert_path(&path[1..], value);
                }
            }
            _ => {
                // Overwrite non-object with object if we need to traverse
                let mut map = HashMap::new();
                let key = path[0];
                if path.len() == 1 {
                    map.insert(key.to_string(), value);
                } else {
                    let mut next = ContextValue::Object(HashMap::new());
                    next.insert_path(&path[1..], value);
                    map.insert(key.to_string(), next);
                }
                *self = ContextValue::Object(map);
            }
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            ContextValue::String(s) => serde_json::Value::String(s.clone()),
            ContextValue::Number(n) => serde_json::Number::from_f64(*n)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            ContextValue::Boolean(b) => serde_json::Value::Bool(*b),
            ContextValue::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| v.to_json()).collect())
            }
            ContextValue::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, v) in obj {
                    map.insert(k.clone(), v.to_json());
                }
                serde_json::Value::Object(map)
            }
        }
    }
}

impl std::fmt::Display for ContextValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextValue::String(s) => write!(f, "{}", s),
            ContextValue::Number(n) => write!(f, "{}", n),
            ContextValue::Boolean(b) => write!(f, "{}", b),
            ContextValue::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            ContextValue::Object(obj) => {
                write!(f, "{{")?;
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl From<String> for ContextValue {
    fn from(value: String) -> Self {
        ContextValue::String(value)
    }
}

impl From<&str> for ContextValue {
    fn from(value: &str) -> Self {
        ContextValue::String(value.to_string())
    }
}

impl From<f64> for ContextValue {
    fn from(value: f64) -> Self {
        ContextValue::Number(value)
    }
}

impl From<bool> for ContextValue {
    fn from(value: bool) -> Self {
        ContextValue::Boolean(value)
    }
}

impl From<Vec<ContextValue>> for ContextValue {
    fn from(value: Vec<ContextValue>) -> Self {
        ContextValue::Array(value)
    }
}

impl From<Vec<String>> for ContextValue {
    fn from(value: Vec<String>) -> Self {
        ContextValue::Array(value.into_iter().map(ContextValue::String).collect())
    }
}

impl From<Vec<&str>> for ContextValue {
    fn from(value: Vec<&str>) -> Self {
        ContextValue::Array(
            value
                .into_iter()
                .map(|s| ContextValue::String(s.to_string()))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // PromptContext tests
    #[test]
    fn test_context_new() {
        let ctx = PromptContext::new();
        assert!(ctx.variables.is_empty());
        assert!(ctx.scopes.is_empty());
    }

    #[test]
    fn test_context_set_get() {
        let mut ctx = PromptContext::new();
        ctx.set(
            "name".to_string(),
            ContextValue::String("Alice".to_string()),
        );

        let value = ctx.get("name").unwrap();
        assert!(matches!(value, ContextValue::String(s) if s == "Alice"));
    }

    #[test]
    fn test_context_get_nonexistent() {
        let ctx = PromptContext::new();
        assert!(ctx.get("missing").is_none());
    }

    #[test]
    fn test_context_push_pop_scope() {
        let mut ctx = PromptContext::new();
        ctx.set(
            "outer".to_string(),
            ContextValue::String("outer_value".to_string()),
        );

        ctx.push_scope();
        ctx.set(
            "inner".to_string(),
            ContextValue::String("inner_value".to_string()),
        );

        // Both should be accessible
        assert!(ctx.get("inner").is_some());
        assert!(ctx.get("outer").is_some());

        // Pop scope
        let popped = ctx.pop_scope();
        assert!(popped.is_some());

        // Inner should no longer be accessible directly, outer should be back
        assert!(ctx.get("outer").is_some());
    }

    #[test]
    fn test_context_scope_shadowing() {
        let mut ctx = PromptContext::new();
        ctx.set("x".to_string(), ContextValue::Number(1.0));

        ctx.push_scope();
        ctx.set("x".to_string(), ContextValue::Number(2.0));

        // Should get inner scope value
        let value = ctx.get("x").unwrap();
        assert!(matches!(value, ContextValue::Number(n) if *n == 2.0));
    }

    #[test]
    fn test_context_pop_empty() {
        let mut ctx = PromptContext::new();
        assert!(ctx.pop_scope().is_none());
    }

    #[test]
    fn test_context_set_path_simple() {
        let mut ctx = PromptContext::new();
        ctx.set_path("name", ContextValue::String("Bob".to_string()));

        assert!(ctx.get("name").is_some());
    }

    #[test]
    fn test_context_set_path_nested() {
        let mut ctx = PromptContext::new();
        ctx.set_path("user.name", ContextValue::String("Charlie".to_string()));
        ctx.set_path("user.age", ContextValue::Number(30.0));

        let user = ctx.get("user").unwrap();
        assert!(matches!(user, ContextValue::Object(_)));
    }

    #[test]
    fn test_context_set_path_empty() {
        let mut ctx = PromptContext::new();
        ctx.set_path("", ContextValue::String("value".to_string()));
        // Should not crash, just do nothing
    }

    #[test]
    fn test_context_merge() {
        let mut ctx1 = PromptContext::new();
        ctx1.set("a".to_string(), ContextValue::String("1".to_string()));

        let mut ctx2 = PromptContext::new();
        ctx2.set("b".to_string(), ContextValue::String("2".to_string()));

        ctx1.merge(ctx2);

        assert!(ctx1.get("a").is_some());
        assert!(ctx1.get("b").is_some());
    }

    #[test]
    fn test_context_to_string_map() {
        let mut ctx = PromptContext::new();
        ctx.set("name".to_string(), ContextValue::String("Test".to_string()));
        ctx.set("count".to_string(), ContextValue::Number(42.0));

        let map = ctx.to_string_map();
        assert_eq!(map.get("name").unwrap(), "Test");
        assert_eq!(map.get("count").unwrap(), "42");
    }

    #[test]
    fn test_context_to_json() {
        let mut ctx = PromptContext::new();
        ctx.set("name".to_string(), ContextValue::String("Test".to_string()));
        ctx.set("active".to_string(), ContextValue::Boolean(true));

        let json = ctx.to_json();
        assert!(json.is_object());
        assert_eq!(json["name"], "Test");
        assert_eq!(json["active"], true);
    }

    // ContextValue tests
    #[test]
    fn test_context_value_string() {
        let value = ContextValue::String("hello".to_string());
        assert_eq!(value.to_string(), "hello");
    }

    #[test]
    fn test_context_value_number() {
        let value = ContextValue::Number(4.56);
        assert!(value.to_string().contains("4.56"));
    }

    #[test]
    fn test_context_value_boolean() {
        let true_val = ContextValue::Boolean(true);
        let false_val = ContextValue::Boolean(false);

        assert_eq!(true_val.to_string(), "true");
        assert_eq!(false_val.to_string(), "false");
    }

    #[test]
    fn test_context_value_array() {
        let value = ContextValue::Array(vec![
            ContextValue::String("a".to_string()),
            ContextValue::String("b".to_string()),
        ]);

        let s = value.to_string();
        assert!(s.contains("a"));
        assert!(s.contains("b"));
    }

    #[test]
    fn test_context_value_object() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), ContextValue::String("value".to_string()));
        let value = ContextValue::Object(map);

        let s = value.to_string();
        assert!(s.contains("key"));
        assert!(s.contains("value"));
    }

    #[test]
    fn test_context_value_to_json_string() {
        let value = ContextValue::String("test".to_string());
        let json = value.to_json();
        assert_eq!(json, serde_json::Value::String("test".to_string()));
    }

    #[test]
    fn test_context_value_to_json_number() {
        let value = ContextValue::Number(42.0);
        let json = value.to_json();
        assert!(json.is_number());
    }

    #[test]
    fn test_context_value_to_json_boolean() {
        let value = ContextValue::Boolean(true);
        let json = value.to_json();
        assert_eq!(json, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_context_value_to_json_array() {
        let value = ContextValue::Array(vec![ContextValue::Number(1.0)]);
        let json = value.to_json();
        assert!(json.is_array());
    }

    #[test]
    fn test_context_value_to_json_object() {
        let mut map = HashMap::new();
        map.insert("x".to_string(), ContextValue::Number(10.0));
        let value = ContextValue::Object(map);
        let json = value.to_json();
        assert!(json.is_object());
    }

    #[test]
    fn test_context_value_insert_path_single() {
        let mut value = ContextValue::Object(HashMap::new());
        value.insert_path(&["key"], ContextValue::String("val".to_string()));

        if let ContextValue::Object(map) = value {
            assert!(map.contains_key("key"));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_context_value_insert_path_nested() {
        let mut value = ContextValue::Object(HashMap::new());
        value.insert_path(&["a", "b", "c"], ContextValue::Number(123.0));

        if let ContextValue::Object(map) = &value {
            assert!(map.contains_key("a"));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_context_value_insert_path_overwrite_non_object() {
        let mut value = ContextValue::String("old".to_string());
        value.insert_path(&["new", "path"], ContextValue::Boolean(true));

        // Should have been converted to object
        assert!(matches!(value, ContextValue::Object(_)));
    }

    #[test]
    fn test_context_value_insert_path_empty() {
        let mut value = ContextValue::String("old".to_string());
        value.insert_path(&[], ContextValue::String("new".to_string()));

        // Empty path should overwrite the value
        assert!(matches!(value, ContextValue::String(s) if s == "new"));
    }

    // From trait tests
    #[test]
    fn test_from_string() {
        let value: ContextValue = "hello".to_string().into();
        assert!(matches!(value, ContextValue::String(s) if s == "hello"));
    }

    #[test]
    fn test_from_str() {
        let value: ContextValue = "world".into();
        assert!(matches!(value, ContextValue::String(s) if s == "world"));
    }

    #[test]
    fn test_from_f64() {
        let value: ContextValue = 1.23f64.into();
        assert!(matches!(value, ContextValue::Number(n) if (n - 1.23).abs() < 0.01));
    }

    #[test]
    fn test_from_bool() {
        let value: ContextValue = true.into();
        assert!(matches!(value, ContextValue::Boolean(true)));
    }

    #[test]
    fn test_from_vec_context_value() {
        let vec = vec![ContextValue::Number(1.0), ContextValue::Number(2.0)];
        let value: ContextValue = vec.into();

        if let ContextValue::Array(arr) = value {
            assert_eq!(arr.len(), 2);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_from_vec_string() {
        let vec = vec!["a".to_string(), "b".to_string()];
        let value: ContextValue = vec.into();

        if let ContextValue::Array(arr) = value {
            assert_eq!(arr.len(), 2);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_from_vec_str() {
        let vec = vec!["x", "y", "z"];
        let value: ContextValue = vec.into();

        if let ContextValue::Array(arr) = value {
            assert_eq!(arr.len(), 3);
        } else {
            panic!("Expected array");
        }
    }

    // Serialization tests
    #[test]
    fn test_context_value_serialization() {
        let value = ContextValue::String("test".to_string());
        let serialized = serde_json::to_string(&value).unwrap();
        let deserialized: ContextValue = serde_json::from_str(&serialized).unwrap();

        assert!(matches!(deserialized, ContextValue::String(s) if s == "test"));
    }

    #[test]
    fn test_context_value_complex_serialization() {
        let mut map = HashMap::new();
        map.insert(
            "items".to_string(),
            ContextValue::Array(vec![ContextValue::Number(1.0), ContextValue::Number(2.0)]),
        );
        let value = ContextValue::Object(map);

        let serialized = serde_json::to_string(&value).unwrap();
        let deserialized: ContextValue = serde_json::from_str(&serialized).unwrap();

        assert!(matches!(deserialized, ContextValue::Object(_)));
    }

    // ===== Mutation-resistant tests for insert_path and Display =====

    #[test]
    fn insert_path_preserves_existing_keys_in_object() {
        // Mutant: delete Object(map) match arm → falls to _ arm which creates new Object, losing "existing"
        let mut map = HashMap::new();
        map.insert(
            "existing".to_string(),
            ContextValue::String("keep_me".to_string()),
        );
        let mut value = ContextValue::Object(map);

        value.insert_path(&["new_key"], ContextValue::Number(42.0));

        if let ContextValue::Object(m) = &value {
            assert!(m.contains_key("existing"), "Existing key must be preserved");
            assert!(
                matches!(m.get("existing"), Some(ContextValue::String(s)) if s == "keep_me"),
                "Existing value must be unchanged"
            );
            assert!(m.contains_key("new_key"), "New key must be inserted");
        } else {
            panic!("Expected Object variant");
        }
    }

    #[test]
    fn insert_path_single_depth_inserts_directly() {
        // Mutant: `path.len() == 1` → `!= 1` would recurse with empty path[1..] instead
        let mut value = ContextValue::Object(HashMap::new());
        value.insert_path(&["direct"], ContextValue::String("value".to_string()));

        if let ContextValue::Object(m) = &value {
            assert!(
                matches!(m.get("direct"), Some(ContextValue::String(s)) if s == "value"),
                "Single-depth insert must place value directly"
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn insert_path_multi_depth_creates_nested_structure() {
        // Mutant: `== 1` → `!= 1` would insert directly instead of recursing for multi-depth
        let mut value = ContextValue::Object(HashMap::new());
        value.insert_path(&["a", "b"], ContextValue::Boolean(true));

        // "a" should be an Object containing "b" → true
        if let ContextValue::Object(outer) = &value {
            if let Some(ContextValue::Object(inner)) = outer.get("a") {
                assert!(
                    matches!(inner.get("b"), Some(ContextValue::Boolean(true))),
                    "Nested path must create nested Object structure"
                );
            } else {
                panic!("'a' must be Object, got {:?}", outer.get("a"));
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn insert_path_overwrite_non_object_single_depth() {
        // Tests the _ arm with path.len() == 1
        let mut value = ContextValue::Number(99.0);
        value.insert_path(&["key"], ContextValue::String("val".to_string()));

        if let ContextValue::Object(m) = &value {
            assert!(matches!(m.get("key"), Some(ContextValue::String(s)) if s == "val"));
        } else {
            panic!("Non-object should be overwritten with Object");
        }
    }

    #[test]
    fn insert_path_overwrite_non_object_multi_depth() {
        // Tests the _ arm with path.len() > 1,  `!= 1` mutant
        let mut value = ContextValue::Boolean(false);
        value.insert_path(&["x", "y"], ContextValue::Number(7.0));

        if let ContextValue::Object(outer) = &value {
            if let Some(ContextValue::Object(inner)) = outer.get("x") {
                assert!(
                    matches!(inner.get("y"), Some(ContextValue::Number(n)) if (*n - 7.0).abs() < 0.001)
                );
            } else {
                panic!("'x' must be Object");
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn display_array_separator_correctness() {
        // Mutant: `i > 0` → `i == 0`, `i < 0`, `i >= 0` in Array Display
        // With `> 0`: output = "[a, b, c]" (comma before 2nd and 3rd)
        // With `== 0`: output = "[, ab, c]" (comma before 1st only — wrong)
        // With `< 0`: output = "[abc]" (no commas — wrong)
        // With `>= 0`: output = "[, a, b, c]" (comma before every element — wrong)
        let arr = ContextValue::Array(vec![
            ContextValue::String("a".to_string()),
            ContextValue::String("b".to_string()),
            ContextValue::String("c".to_string()),
        ]);
        let display = format!("{}", arr);
        assert_eq!(
            display, "[a, b, c]",
            "Array Display must have commas only between elements, got: {}",
            display
        );
        assert!(
            !display.starts_with("[, "),
            "Must not start with comma-space"
        );
    }

    #[test]
    fn display_object_separator_correctness() {
        // Mutant: `i > 0` → `i == 0`, `i < 0`, `i >= 0` in Object Display
        // Use single-entry object to simplify (no ordering issues with HashMap)
        let mut map = HashMap::new();
        map.insert("key".to_string(), ContextValue::Number(42.0));
        let obj = ContextValue::Object(map);
        let display = format!("{}", obj);
        assert_eq!(
            display, "{key: 42}",
            "Single-entry object must not have leading comma, got: {}",
            display
        );
        assert!(
            !display.starts_with("{, "),
            "Must not start with comma-space after brace"
        );
    }

    #[test]
    fn display_array_two_elements() {
        let arr = ContextValue::Array(vec![ContextValue::Number(1.0), ContextValue::Number(2.0)]);
        let display = format!("{}", arr);
        assert_eq!(display, "[1, 2]");
    }

    #[test]
    fn display_empty_array() {
        let arr = ContextValue::Array(vec![]);
        let display = format!("{}", arr);
        assert_eq!(display, "[]");
    }
}
