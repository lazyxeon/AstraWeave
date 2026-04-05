//! Template library management
//!
//! This module provides template library and collection management.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Template library
#[derive(Debug, Clone)]
pub struct TemplateLibrary {
    /// Library name
    #[allow(dead_code)]
    name: String,
    /// Template collections
    #[allow(dead_code)]
    collections: HashMap<String, TemplateCollection>,
    /// Library metadata
    #[allow(dead_code)]
    metadata: LibraryMetadata,
}

/// Backwards-compatible PromptLibrary used by consumers
#[derive(Debug, Clone)]
pub struct PromptLibrary {
    templates: HashMap<String, crate::template::PromptTemplate>,
}

impl Default for PromptLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptLibrary {
    /// Create a new prompt library
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Add a template to the library
    pub fn add_template(&mut self, name: &str, template: crate::template::PromptTemplate) {
        self.templates.insert(name.to_string(), template);
    }

    /// Get a template by name (cloned)
    pub fn get_template(&self, name: &str) -> anyhow::Result<crate::template::PromptTemplate> {
        if let Some(t) = self.templates.get(name) {
            Ok(t.clone())
        } else {
            anyhow::bail!("template not found: {}", name)
        }
    }

    /// Delete a template from the library
    pub fn delete_template(&mut self, name: &str) -> Option<crate::template::PromptTemplate> {
        self.templates.remove(name)
    }

    /// List all templates
    pub fn list_templates(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }

    /// Returns the number of templates in the library.
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// Returns true if the library contains a template with the given name.
    pub fn has_template(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Returns true if the library is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Clears all templates from the library.
    pub fn clear(&mut self) {
        self.templates.clear();
    }

    /// Returns a summary of the library.
    pub fn summary(&self) -> String {
        format!("PromptLibrary: {} templates", self.template_count())
    }
}

impl std::fmt::Display for PromptLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Template collection
#[derive(Debug, Clone)]
pub struct TemplateCollection {
    /// Collection name
    name: String,
    /// Templates in this collection
    templates: HashMap<String, String>,
    /// Collection metadata
    #[allow(dead_code)]
    metadata: CollectionMetadata,
}

/// Library metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryMetadata {
    /// Library version
    pub version: String,
    /// Library description
    pub description: String,
    /// Library author
    pub author: String,
    /// Creation date
    pub created_at: String,
}

impl LibraryMetadata {
    /// Creates a new library metadata.
    pub fn new(
        version: impl Into<String>,
        description: impl Into<String>,
        author: impl Into<String>,
    ) -> Self {
        Self {
            version: version.into(),
            description: description.into(),
            author: author.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Creates a default library metadata.
    pub fn default_metadata() -> Self {
        Self {
            version: "1.0.0".to_string(),
            description: "AstraWeave Template Library".to_string(),
            author: "AstraWeave".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Returns true if this metadata has an author.
    pub fn has_author(&self) -> bool {
        !self.author.is_empty()
    }

    /// Returns true if this metadata has a description.
    pub fn has_description(&self) -> bool {
        !self.description.is_empty()
    }

    /// Returns a summary of the metadata.
    pub fn summary(&self) -> String {
        format!("v{} by {}", self.version, self.author)
    }
}

impl std::fmt::Display for LibraryMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl Default for LibraryMetadata {
    fn default() -> Self {
        Self::default_metadata()
    }
}

/// Collection metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetadata {
    /// Collection version
    pub version: String,
    /// Collection description
    pub description: String,
    /// Collection tags
    pub tags: Vec<String>,
}

impl CollectionMetadata {
    /// Creates a new collection metadata.
    pub fn new(version: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            description: description.into(),
            tags: Vec::new(),
        }
    }

    /// Creates a new collection metadata with tags.
    pub fn with_tags(
        version: impl Into<String>,
        description: impl Into<String>,
        tags: Vec<String>,
    ) -> Self {
        Self {
            version: version.into(),
            description: description.into(),
            tags,
        }
    }

    /// Returns true if this collection has tags.
    pub fn has_tags(&self) -> bool {
        !self.tags.is_empty()
    }

    /// Returns true if this collection has a specific tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Returns the number of tags.
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    /// Returns true if this collection has a description.
    pub fn has_description(&self) -> bool {
        !self.description.is_empty()
    }

    /// Returns a summary of the metadata.
    pub fn summary(&self) -> String {
        if self.tags.is_empty() {
            format!("v{}", self.version)
        } else {
            format!("v{} [{}]", self.version, self.tags.join(", "))
        }
    }
}

impl std::fmt::Display for CollectionMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl Default for CollectionMetadata {
    fn default() -> Self {
        Self::new("1.0.0", "")
    }
}

impl TemplateLibrary {
    /// Create a new template library
    pub fn new(name: String, metadata: LibraryMetadata) -> Self {
        Self {
            name,
            collections: HashMap::new(),
            metadata,
        }
    }

    /// Add a collection to the library
    pub fn add_collection(&mut self, collection: TemplateCollection) {
        self.collections.insert(collection.name.clone(), collection);
    }

    /// Get a collection by name
    pub fn get_collection(&self, name: &str) -> Option<&TemplateCollection> {
        self.collections.get(name)
    }

    /// List all collections
    pub fn list_collections(&self) -> Vec<&String> {
        self.collections.keys().collect()
    }

    /// Returns the number of collections.
    pub fn collection_count(&self) -> usize {
        self.collections.len()
    }

    /// Returns true if the library has a collection with the given name.
    pub fn has_collection(&self, name: &str) -> bool {
        self.collections.contains_key(name)
    }

    /// Returns true if the library is empty.
    pub fn is_empty(&self) -> bool {
        self.collections.is_empty()
    }

    /// Returns the library name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the library metadata.
    pub fn metadata(&self) -> &LibraryMetadata {
        &self.metadata
    }

    /// Returns the total number of templates across all collections.
    pub fn total_template_count(&self) -> usize {
        self.collections.values().map(|c| c.template_count()).sum()
    }

    /// Returns a summary of the library.
    pub fn summary(&self) -> String {
        format!(
            "TemplateLibrary '{}': {} collections, {} templates",
            self.name,
            self.collection_count(),
            self.total_template_count()
        )
    }

    /// Load library from directory
    pub fn load_from_directory(path: PathBuf) -> Result<Self> {
        let mut library = Self::new(
            "default".to_string(),
            LibraryMetadata {
                version: "1.0.0".to_string(),
                description: "Loaded from directory".to_string(),
                author: "AstraWeave".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        );

        let mut default_collection = TemplateCollection::new(
            "default".to_string(),
            CollectionMetadata {
                version: "1.0.0".to_string(),
                description: "Default collection".to_string(),
                tags: vec![],
            },
        );

        if path.exists() && path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "hbs" {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let content = std::fs::read_to_string(&path)?;
                            default_collection.add_template(name, content);
                        }
                    }
                }
            }
        }

        library.add_collection(default_collection);
        Ok(library)
    }

    /// Save library to directory
    pub fn save_to_directory(&self, path: PathBuf) -> Result<()> {
        // Stub implementation - would actually write to filesystem
        let _ = path;
        Ok(())
    }
}

impl std::fmt::Display for TemplateLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl TemplateCollection {
    /// Create a new template collection
    pub fn new(name: String, metadata: CollectionMetadata) -> Self {
        Self {
            name,
            templates: HashMap::new(),
            metadata,
        }
    }

    /// Add a template to the collection
    pub fn add_template(&mut self, name: String, template: String) {
        self.templates.insert(name, template);
    }

    /// Get a template by name
    pub fn get_template(&self, name: &str) -> Option<&String> {
        self.templates.get(name)
    }

    /// List all templates in the collection
    pub fn list_templates(&self) -> Vec<&String> {
        self.templates.keys().collect()
    }

    /// Remove a template from the collection
    pub fn remove_template(&mut self, name: &str) -> Option<String> {
        self.templates.remove(name)
    }

    /// Returns the collection name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the collection metadata.
    pub fn metadata(&self) -> &CollectionMetadata {
        &self.metadata
    }

    /// Returns the number of templates in the collection.
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// Returns true if the collection contains a template with the given name.
    pub fn has_template(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Clears all templates from the collection.
    pub fn clear(&mut self) {
        self.templates.clear();
    }

    /// Returns a summary of the collection.
    pub fn summary(&self) -> String {
        format!(
            "TemplateCollection '{}': {} templates ({})",
            self.name,
            self.template_count(),
            self.metadata.summary()
        )
    }
}

impl std::fmt::Display for TemplateCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_library_new() {
        let lib = PromptLibrary::new();
        assert!(lib.list_templates().is_empty());
    }

    #[test]
    fn test_prompt_library_add_and_get() {
        let mut lib = PromptLibrary::new();
        let template =
            crate::template::PromptTemplate::new("test".to_string(), "Hello {{name}}".to_string());

        lib.add_template("greeting", template);

        let result = lib.get_template("greeting");
        assert!(result.is_ok());
    }

    #[test]
    fn test_prompt_library_get_nonexistent() {
        let lib = PromptLibrary::new();
        let result = lib.get_template("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_prompt_library_delete_template() {
        let mut lib = PromptLibrary::new();
        let template =
            crate::template::PromptTemplate::new("test".to_string(), "content".to_string());

        lib.add_template("test", template);
        assert!(!lib.list_templates().is_empty());

        let deleted = lib.delete_template("test");
        assert!(deleted.is_some());
        assert!(lib.list_templates().is_empty());
    }

    #[test]
    fn test_prompt_library_delete_nonexistent() {
        let mut lib = PromptLibrary::new();
        let deleted = lib.delete_template("nonexistent");
        assert!(deleted.is_none());
    }

    #[test]
    fn test_prompt_library_list_templates() {
        let mut lib = PromptLibrary::new();

        lib.add_template(
            "a",
            crate::template::PromptTemplate::new("a".to_string(), "A".to_string()),
        );
        lib.add_template(
            "b",
            crate::template::PromptTemplate::new("b".to_string(), "B".to_string()),
        );
        lib.add_template(
            "c",
            crate::template::PromptTemplate::new("c".to_string(), "C".to_string()),
        );

        let list = lib.list_templates();
        assert_eq!(list.len(), 3);
        assert!(list.contains(&"a".to_string()));
        assert!(list.contains(&"b".to_string()));
        assert!(list.contains(&"c".to_string()));
    }

    #[test]
    fn test_template_collection_new() {
        let meta = CollectionMetadata {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            tags: vec!["test".to_string()],
        };
        let collection = TemplateCollection::new("test_collection".to_string(), meta);
        assert!(collection.list_templates().is_empty());
    }

    #[test]
    fn test_template_collection_add_and_get() {
        let meta = CollectionMetadata {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            tags: vec![],
        };
        let mut collection = TemplateCollection::new("test".to_string(), meta);

        collection.add_template("greeting".to_string(), "Hello {{name}}".to_string());

        let result = collection.get_template("greeting");
        assert_eq!(result, Some(&"Hello {{name}}".to_string()));
    }

    #[test]
    fn test_template_collection_get_nonexistent() {
        let meta = CollectionMetadata {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            tags: vec![],
        };
        let collection = TemplateCollection::new("test".to_string(), meta);

        let result = collection.get_template("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_template_collection_remove() {
        let meta = CollectionMetadata {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            tags: vec![],
        };
        let mut collection = TemplateCollection::new("test".to_string(), meta);

        collection.add_template("greeting".to_string(), "Hello".to_string());

        let removed = collection.remove_template("greeting");
        assert_eq!(removed, Some("Hello".to_string()));
        assert!(collection.get_template("greeting").is_none());
    }

    #[test]
    fn test_template_collection_list() {
        let meta = CollectionMetadata {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            tags: vec![],
        };
        let mut collection = TemplateCollection::new("test".to_string(), meta);

        collection.add_template("a".to_string(), "A".to_string());
        collection.add_template("b".to_string(), "B".to_string());

        let list = collection.list_templates();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_template_library_new() {
        let meta = LibraryMetadata {
            version: "1.0".to_string(),
            description: "Test library".to_string(),
            author: "Test".to_string(),
            created_at: "2025-01-01".to_string(),
        };
        let library = TemplateLibrary::new("test_lib".to_string(), meta);
        assert!(library.list_collections().is_empty());
    }

    #[test]
    fn test_template_library_add_and_get_collection() {
        let lib_meta = LibraryMetadata {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            created_at: "2025-01-01".to_string(),
        };
        let mut library = TemplateLibrary::new("lib".to_string(), lib_meta);

        let col_meta = CollectionMetadata {
            version: "1.0".to_string(),
            description: "Coll".to_string(),
            tags: vec![],
        };
        let collection = TemplateCollection::new("my_collection".to_string(), col_meta);

        library.add_collection(collection);

        let result = library.get_collection("my_collection");
        assert!(result.is_some());
    }

    #[test]
    fn test_template_library_get_nonexistent_collection() {
        let meta = LibraryMetadata {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            created_at: "2025-01-01".to_string(),
        };
        let library = TemplateLibrary::new("lib".to_string(), meta);

        let result = library.get_collection("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_template_library_list_collections() {
        let lib_meta = LibraryMetadata {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            created_at: "2025-01-01".to_string(),
        };
        let mut library = TemplateLibrary::new("lib".to_string(), lib_meta);

        let col_meta1 = CollectionMetadata {
            version: "1.0".to_string(),
            description: "A".to_string(),
            tags: vec![],
        };
        let col_meta2 = CollectionMetadata {
            version: "1.0".to_string(),
            description: "B".to_string(),
            tags: vec![],
        };

        library.add_collection(TemplateCollection::new("col1".to_string(), col_meta1));
        library.add_collection(TemplateCollection::new("col2".to_string(), col_meta2));

        let list = library.list_collections();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_template_library_save_stub() {
        let meta = LibraryMetadata {
            version: "1.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            created_at: "2025-01-01".to_string(),
        };
        let library = TemplateLibrary::new("lib".to_string(), meta);

        // Stub should succeed
        let result = library.save_to_directory(std::path::PathBuf::from("/tmp/test"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_library_metadata_serialization() {
        let meta = LibraryMetadata {
            version: "2.0.0".to_string(),
            description: "My library".to_string(),
            author: "Developer".to_string(),
            created_at: "2025-06-15".to_string(),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let restored: LibraryMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.version, "2.0.0");
        assert_eq!(restored.author, "Developer");
    }

    #[test]
    fn test_collection_metadata_serialization() {
        let meta = CollectionMetadata {
            version: "1.0".to_string(),
            description: "Test collection".to_string(),
            tags: vec!["ai".to_string(), "prompts".to_string()],
        };

        let json = serde_json::to_string(&meta).unwrap();
        let restored: CollectionMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tags.len(), 2);
        assert!(restored.tags.contains(&"ai".to_string()));
    }

    #[test]
    fn test_template_library_load_from_nonexistent_directory() {
        let path = std::path::PathBuf::from("/nonexistent/path/xyz123");
        let result = TemplateLibrary::load_from_directory(path);
        // Should succeed with empty default collection
        assert!(result.is_ok());
    }

    // ===== PromptLibrary helper tests =====

    #[test]
    fn test_prompt_library_template_count() {
        let mut lib = PromptLibrary::new();
        assert_eq!(lib.template_count(), 0);

        let template = crate::template::PromptTemplate::new("test", "Hello");
        lib.add_template("greeting", template);
        assert_eq!(lib.template_count(), 1);
    }

    #[test]
    fn test_prompt_library_has_template() {
        let mut lib = PromptLibrary::new();
        assert!(!lib.has_template("greeting"));

        let template = crate::template::PromptTemplate::new("test", "Hello");
        lib.add_template("greeting", template);
        assert!(lib.has_template("greeting"));
    }

    #[test]
    fn test_prompt_library_is_empty() {
        let mut lib = PromptLibrary::new();
        assert!(lib.is_empty());

        let template = crate::template::PromptTemplate::new("test", "Hello");
        lib.add_template("greeting", template);
        assert!(!lib.is_empty());
    }

    #[test]
    fn test_prompt_library_clear() {
        let mut lib = PromptLibrary::new();
        let template = crate::template::PromptTemplate::new("test", "Hello");
        lib.add_template("greeting", template);
        assert!(!lib.is_empty());

        lib.clear();
        assert!(lib.is_empty());
    }

    #[test]
    fn test_prompt_library_summary() {
        let lib = PromptLibrary::new();
        let summary = lib.summary();
        assert!(summary.contains("PromptLibrary"));
        assert!(summary.contains("0 templates"));
    }

    #[test]
    fn test_prompt_library_display() {
        let lib = PromptLibrary::new();
        let display = format!("{}", lib);
        assert!(display.contains("PromptLibrary"));
    }

    // ===== LibraryMetadata helper tests =====

    #[test]
    fn test_library_metadata_new() {
        let meta = LibraryMetadata::new("1.0.0", "Test library", "Author");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.description, "Test library");
        assert_eq!(meta.author, "Author");
    }

    #[test]
    fn test_library_metadata_default() {
        let meta = LibraryMetadata::default();
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.author, "AstraWeave");
    }

    #[test]
    fn test_library_metadata_has_author() {
        let meta = LibraryMetadata::new("1.0", "Desc", "Author");
        assert!(meta.has_author());

        let no_author = LibraryMetadata::new("1.0", "Desc", "");
        assert!(!no_author.has_author());
    }

    #[test]
    fn test_library_metadata_has_description() {
        let meta = LibraryMetadata::new("1.0", "Description", "Author");
        assert!(meta.has_description());

        let no_desc = LibraryMetadata::new("1.0", "", "Author");
        assert!(!no_desc.has_description());
    }

    #[test]
    fn test_library_metadata_summary() {
        let meta = LibraryMetadata::new("2.0.0", "Desc", "Developer");
        let summary = meta.summary();
        assert!(summary.contains("v2.0.0"));
        assert!(summary.contains("Developer"));
    }

    #[test]
    fn test_library_metadata_display() {
        let meta = LibraryMetadata::new("1.0", "Desc", "Author");
        let display = format!("{}", meta);
        assert!(display.contains("v1.0"));
    }

    // ===== CollectionMetadata helper tests =====

    #[test]
    fn test_collection_metadata_new() {
        let meta = CollectionMetadata::new("1.0.0", "Test collection");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.description, "Test collection");
        assert!(meta.tags.is_empty());
    }

    #[test]
    fn test_collection_metadata_with_tags() {
        let meta = CollectionMetadata::with_tags(
            "1.0",
            "Desc",
            vec!["ai".to_string(), "game".to_string()],
        );
        assert_eq!(meta.tags.len(), 2);
    }

    #[test]
    fn test_collection_metadata_has_tags() {
        let meta = CollectionMetadata::new("1.0", "Desc");
        assert!(!meta.has_tags());

        let with_tags = CollectionMetadata::with_tags("1.0", "Desc", vec!["ai".to_string()]);
        assert!(with_tags.has_tags());
    }

    #[test]
    fn test_collection_metadata_has_tag() {
        let meta = CollectionMetadata::with_tags(
            "1.0",
            "Desc",
            vec!["ai".to_string(), "game".to_string()],
        );
        assert!(meta.has_tag("ai"));
        assert!(meta.has_tag("game"));
        assert!(!meta.has_tag("missing"));
    }

    #[test]
    fn test_collection_metadata_tag_count() {
        let meta = CollectionMetadata::with_tags(
            "1.0",
            "Desc",
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
        assert_eq!(meta.tag_count(), 3);
    }

    #[test]
    fn test_collection_metadata_summary() {
        let meta = CollectionMetadata::with_tags("2.0", "Desc", vec!["ai".to_string()]);
        let summary = meta.summary();
        assert!(summary.contains("v2.0"));
        assert!(summary.contains("ai"));
    }

    #[test]
    fn test_collection_metadata_display() {
        let meta = CollectionMetadata::new("1.0", "Desc");
        let display = format!("{}", meta);
        assert!(display.contains("v1.0"));
    }

    #[test]
    fn test_collection_metadata_default() {
        let meta = CollectionMetadata::default();
        assert_eq!(meta.version, "1.0.0");
        assert!(meta.tags.is_empty());
    }

    // ===== TemplateLibrary helper tests =====

    #[test]
    fn test_template_library_collection_count() {
        let mut lib = TemplateLibrary::new("test".to_string(), LibraryMetadata::default());
        assert_eq!(lib.collection_count(), 0);

        lib.add_collection(TemplateCollection::new(
            "col1".to_string(),
            CollectionMetadata::default(),
        ));
        assert_eq!(lib.collection_count(), 1);
    }

    #[test]
    fn test_template_library_has_collection() {
        let mut lib = TemplateLibrary::new("test".to_string(), LibraryMetadata::default());
        assert!(!lib.has_collection("col1"));

        lib.add_collection(TemplateCollection::new(
            "col1".to_string(),
            CollectionMetadata::default(),
        ));
        assert!(lib.has_collection("col1"));
    }

    #[test]
    fn test_template_library_is_empty() {
        let lib = TemplateLibrary::new("test".to_string(), LibraryMetadata::default());
        assert!(lib.is_empty());
    }

    #[test]
    fn test_template_library_name() {
        let lib = TemplateLibrary::new("my_library".to_string(), LibraryMetadata::default());
        assert_eq!(lib.name(), "my_library");
    }

    #[test]
    fn test_template_library_metadata() {
        let lib = TemplateLibrary::new(
            "test".to_string(),
            LibraryMetadata::new("2.0", "Desc", "Author"),
        );
        assert_eq!(lib.metadata().version, "2.0");
    }

    #[test]
    fn test_template_library_total_template_count() {
        let mut lib = TemplateLibrary::new("test".to_string(), LibraryMetadata::default());

        let mut col1 = TemplateCollection::new("col1".to_string(), CollectionMetadata::default());
        col1.add_template("t1".to_string(), "Template 1".to_string());
        col1.add_template("t2".to_string(), "Template 2".to_string());
        lib.add_collection(col1);

        let mut col2 = TemplateCollection::new("col2".to_string(), CollectionMetadata::default());
        col2.add_template("t3".to_string(), "Template 3".to_string());
        lib.add_collection(col2);

        assert_eq!(lib.total_template_count(), 3);
    }

    #[test]
    fn test_template_library_summary() {
        let lib = TemplateLibrary::new("my_lib".to_string(), LibraryMetadata::default());
        let summary = lib.summary();
        assert!(summary.contains("TemplateLibrary"));
        assert!(summary.contains("my_lib"));
    }

    #[test]
    fn test_template_library_display() {
        let lib = TemplateLibrary::new("test".to_string(), LibraryMetadata::default());
        let display = format!("{}", lib);
        assert!(display.contains("TemplateLibrary"));
    }

    // ===== TemplateCollection helper tests =====

    #[test]
    fn test_template_collection_name() {
        let col =
            TemplateCollection::new("my_collection".to_string(), CollectionMetadata::default());
        assert_eq!(col.name(), "my_collection");
    }

    #[test]
    fn test_template_collection_metadata() {
        let col =
            TemplateCollection::new("test".to_string(), CollectionMetadata::new("2.0", "Desc"));
        assert_eq!(col.metadata().version, "2.0");
    }

    #[test]
    fn test_template_collection_template_count() {
        let mut col = TemplateCollection::new("test".to_string(), CollectionMetadata::default());
        assert_eq!(col.template_count(), 0);

        col.add_template("t1".to_string(), "Template".to_string());
        assert_eq!(col.template_count(), 1);
    }

    #[test]
    fn test_template_collection_has_template() {
        let mut col = TemplateCollection::new("test".to_string(), CollectionMetadata::default());
        assert!(!col.has_template("t1"));

        col.add_template("t1".to_string(), "Template".to_string());
        assert!(col.has_template("t1"));
    }

    #[test]
    fn test_template_collection_is_empty() {
        let col = TemplateCollection::new("test".to_string(), CollectionMetadata::default());
        assert!(col.is_empty());
    }

    #[test]
    fn test_template_collection_clear() {
        let mut col = TemplateCollection::new("test".to_string(), CollectionMetadata::default());
        col.add_template("t1".to_string(), "Template".to_string());
        assert!(!col.is_empty());

        col.clear();
        assert!(col.is_empty());
    }

    #[test]
    fn test_template_collection_summary() {
        let col = TemplateCollection::new("my_col".to_string(), CollectionMetadata::default());
        let summary = col.summary();
        assert!(summary.contains("TemplateCollection"));
        assert!(summary.contains("my_col"));
    }

    #[test]
    fn test_template_collection_display() {
        let col = TemplateCollection::new("test".to_string(), CollectionMetadata::default());
        let display = format!("{}", col);
        assert!(display.contains("TemplateCollection"));
    }

    // === Mutation Remediation Tests ===

    #[test]
    fn test_has_description_true_when_present() {
        // Remediation: kills "replace has_description -> bool with true/false" and "delete !"
        let meta = CollectionMetadata {
            description: "A real description".to_string(),
            ..Default::default()
        };
        assert!(
            meta.has_description(),
            "non-empty description → has_description should be true"
        );
    }

    #[test]
    fn test_has_description_false_when_empty() {
        let meta = CollectionMetadata {
            description: String::new(),
            ..Default::default()
        };
        assert!(
            !meta.has_description(),
            "empty description → has_description should be false"
        );
    }

    #[test]
    fn test_library_is_empty_true_when_no_collections() {
        // Remediation: kills "replace is_empty -> bool with true" mutation
        let lib = TemplateLibrary::new(
            "test".to_string(),
            LibraryMetadata {
                version: "1.0.0".to_string(),
                description: "test".to_string(),
                author: "test".to_string(),
                created_at: "2026-01-01".to_string(),
            },
        );
        assert!(
            lib.is_empty(),
            "new library with no collections should be empty"
        );
    }

    #[test]
    fn test_library_is_empty_false_after_adding_collection() {
        let mut lib = TemplateLibrary::new(
            "test".to_string(),
            LibraryMetadata {
                version: "1.0.0".to_string(),
                description: "test".to_string(),
                author: "test".to_string(),
                created_at: "2026-01-01".to_string(),
            },
        );
        lib.add_collection(TemplateCollection::new(
            "col1".to_string(),
            CollectionMetadata::default(),
        ));
        assert!(
            !lib.is_empty(),
            "library with a collection should not be empty"
        );
    }

    #[test]
    fn test_load_from_directory_nonexistent_still_has_default_collection() {
        // Remediation: kills "&& → ||" in load_from_directory (path.exists() && path.is_dir())
        let result = TemplateLibrary::load_from_directory(std::path::PathBuf::from(
            "/nonexistent_dir_12345",
        ));
        assert!(
            result.is_ok(),
            "loading from nonexistent dir should succeed with empty default"
        );
        let lib = result.unwrap();
        assert!(
            lib.has_collection("default"),
            "should have default collection"
        );
        assert_eq!(
            lib.total_template_count(),
            0,
            "no templates from nonexistent dir"
        );
    }

    #[test]
    fn test_load_from_directory_with_existing_file_not_dir_kills_and_or_mutant() {
        // Remediation: kills "&& → ||" in load_from_directory
        // A path that EXISTS but is NOT a directory:
        //   Normal (&&): exists=true && is_dir=false → false → skip block → Ok
        //   Mutant (||): exists=true || is_dir=false → true → read_dir(file) → Err
        let temp = std::env::temp_dir().join("astraweave_mutant_test_lib_340.txt");
        std::fs::write(&temp, "not a directory").unwrap();
        let result = TemplateLibrary::load_from_directory(temp.clone());
        let _ = std::fs::remove_file(&temp);
        assert!(
            result.is_ok(),
            "loading from file path (not dir) should succeed (skip non-dir)"
        );
    }
}
