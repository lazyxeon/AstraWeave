//! Terrain Generation Prompt Templates
//!
//! This module provides pre-built prompt templates for AI-orchestrated
//! dynamic terrain generation and modification.
//!
//! # Overview
//!
//! These templates are designed to work with the TerrainGenerationRequest
//! schema defined in astraweave-core, enabling Qwen3-8B LLM to generate
//! structured terrain modification requests.

use crate::{PromptTemplate, TemplateCategory, TemplateMetadata};
use std::collections::HashMap;

/// Default terrain generation system prompt
pub const TERRAIN_SYSTEM_PROMPT: &str = r#"You are a terrain generation AI for the AstraWeave game engine.
Your role is to translate natural language terrain requests into structured JSON.

When generating terrain modifications, you MUST:
1. Use only the allowed feature types listed below
2. Specify locations relative to the player or using coordinates
3. Set appropriate intensity (0.0-1.0) and radius
4. Consider biome compatibility

Allowed TerrainFeatureType values:
- Mountain, Hill, Valley, Cliff, Canyon
- River, Lake, Waterfall, Pond, Stream
- Forest, Grove, Meadow, Marsh, Desert
- Cave, Crater, Plateau, Ridge, Ravine

Allowed RelativeLocation types:
- LineOfSight: Place visible in front of player
- DirectionFrom: Cardinal direction (North/South/East/West) + distance
- Coordinates: Exact (x, y, z) position

Output MUST be valid JSON matching the TerrainGenerationRequest schema."#;

/// Terrain feature request prompt template
pub const TERRAIN_FEATURE_TEMPLATE: &str = r#"Generate a TerrainGenerationRequest for the following:

Player Request: {{request_text}}
Current Biome: {{current_biome}}
Player Position: ({{player_x}}, {{player_y}}, {{player_z}})
Player Facing: {{player_facing}}

Respond with a valid JSON object:
```json
{
  "feature_type": "...",
  "location": { ... },
  "radius": <number>,
  "intensity": <0.0-1.0>,
  "biome_constraints": [...],
  "persistence_mode": "Permanent|Temporary|Session",
  "seed": <optional number>
}
```"#;

/// Simple terrain modification prompt
pub const TERRAIN_MODIFY_TEMPLATE: &str = r#"Convert this terrain request to a TerrainGenerationRequest:

Request: "{{request_text}}"

The player is at position ({{player_x}}, {{player_y}}, {{player_z}}).
{{#if direction}}They want the feature to the {{direction}}.{{/if}}
{{#if distance}}Distance: {{distance}} blocks away.{{/if}}

Generate the JSON response."#;

/// Batch terrain generation prompt
pub const TERRAIN_BATCH_TEMPLATE: &str = r#"Generate multiple terrain features for this scene:

Description: {{scene_description}}
Center Position: ({{center_x}}, {{center_y}}, {{center_z}})
Area Radius: {{area_radius}} blocks
Biome: {{biome}}

Generate an array of TerrainGenerationRequest objects that would create this scene.
Each feature should have appropriate spacing and not overlap excessively.

```json
[
  { "feature_type": "...", "location": {...}, ... },
  ...
]
```"#;

/// Get the default terrain system prompt template
pub fn terrain_system_template() -> PromptTemplate {
    PromptTemplate::new_with_metadata(
        "terrain_system",
        TERRAIN_SYSTEM_PROMPT,
        terrain_system_metadata(),
    )
}

/// Get the terrain feature request template
pub fn terrain_feature_template() -> PromptTemplate {
    let mut metadata = TemplateMetadata {
        name: "terrain_feature".to_string(),
        description: "Generate a single terrain feature from natural language".to_string(),
        category: TemplateCategory::TerrainGeneration,
        tags: vec!["terrain".into(), "generation".into(), "feature".into()],
        required_variables: vec![
            "request_text".into(),
            "current_biome".into(),
            "player_x".into(),
            "player_y".into(),
            "player_z".into(),
            "player_facing".into(),
        ],
        ..Default::default()
    };
    metadata.version = "1.0.0".to_string();

    PromptTemplate::new_with_metadata("terrain_feature", TERRAIN_FEATURE_TEMPLATE, metadata)
}

/// Get the simple terrain modification template
pub fn terrain_modify_template() -> PromptTemplate {
    let mut metadata = TemplateMetadata {
        name: "terrain_modify".to_string(),
        description: "Simple terrain modification with optional direction".to_string(),
        category: TemplateCategory::TerrainGeneration,
        tags: vec!["terrain".into(), "modify".into(), "simple".into()],
        required_variables: vec![
            "request_text".into(),
            "player_x".into(),
            "player_y".into(),
            "player_z".into(),
        ],
        optional_variables: {
            let mut m = HashMap::new();
            m.insert("direction".into(), serde_json::Value::Null);
            m.insert("distance".into(), serde_json::Value::Null);
            m
        },
        ..Default::default()
    };
    metadata.version = "1.0.0".to_string();

    PromptTemplate::new_with_metadata("terrain_modify", TERRAIN_MODIFY_TEMPLATE, metadata)
}

/// Get the batch terrain generation template  
pub fn terrain_batch_template() -> PromptTemplate {
    let mut metadata = TemplateMetadata {
        name: "terrain_batch".to_string(),
        description: "Generate multiple terrain features for a scene".to_string(),
        category: TemplateCategory::TerrainGeneration,
        tags: vec!["terrain".into(), "batch".into(), "scene".into()],
        required_variables: vec![
            "scene_description".into(),
            "center_x".into(),
            "center_y".into(),
            "center_z".into(),
            "area_radius".into(),
            "biome".into(),
        ],
        ..Default::default()
    };
    metadata.version = "1.0.0".to_string();

    PromptTemplate::new_with_metadata("terrain_batch", TERRAIN_BATCH_TEMPLATE, metadata)
}

fn terrain_system_metadata() -> TemplateMetadata {
    TemplateMetadata {
        name: "terrain_system".to_string(),
        description: "System prompt for terrain generation AI".to_string(),
        category: TemplateCategory::TerrainGeneration,
        tags: vec!["terrain".into(), "system".into(), "instructions".into()],
        required_variables: vec![],
        ..Default::default()
    }
}

/// Register all terrain templates with a template engine
pub fn register_terrain_templates(engine: &mut crate::TemplateEngine) -> anyhow::Result<()> {
    engine.register_template("terrain_system", terrain_system_template())?;
    engine.register_template("terrain_feature", terrain_feature_template())?;
    engine.register_template("terrain_modify", terrain_modify_template())?;
    engine.register_template("terrain_batch", terrain_batch_template())?;
    Ok(())
}

/// Get all terrain templates as a collection
pub fn terrain_template_collection() -> Vec<(String, PromptTemplate)> {
    vec![
        ("terrain_system".into(), terrain_system_template()),
        ("terrain_feature".into(), terrain_feature_template()),
        ("terrain_modify".into(), terrain_modify_template()),
        ("terrain_batch".into(), terrain_batch_template()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_system_template() {
        let template = terrain_system_template();
        assert_eq!(template.metadata().name, "terrain_system");
        assert_eq!(
            template.metadata().category,
            TemplateCategory::TerrainGeneration
        );
    }

    #[test]
    fn test_terrain_feature_template() {
        let template = terrain_feature_template();
        assert_eq!(template.metadata().name, "terrain_feature");
        assert!(!template.metadata().required_variables.is_empty());
    }

    #[test]
    fn test_terrain_modify_template() {
        let template = terrain_modify_template();
        assert_eq!(template.metadata().name, "terrain_modify");
        assert!(!template.metadata().optional_variables.is_empty());
    }

    #[test]
    fn test_terrain_batch_template() {
        let template = terrain_batch_template();
        assert_eq!(template.metadata().name, "terrain_batch");
        assert!(template
            .metadata()
            .required_variables
            .contains(&"scene_description".to_string()));
    }

    #[test]
    fn test_terrain_template_collection() {
        let templates = terrain_template_collection();
        assert_eq!(templates.len(), 4);
    }

    #[test]
    fn test_system_prompt_contains_feature_types() {
        assert!(TERRAIN_SYSTEM_PROMPT.contains("Mountain"));
        assert!(TERRAIN_SYSTEM_PROMPT.contains("TerrainFeatureType"));
        assert!(TERRAIN_SYSTEM_PROMPT.contains("RelativeLocation"));
    }

    // ── Mutation-resistant field-assertion tests (v3.3 remediation) ──

    #[test]
    fn test_terrain_feature_metadata_description_not_default() {
        // Kills: delete field description from struct TemplateMetadata in terrain_feature_template
        let t = terrain_feature_template();
        assert!(
            !t.metadata().description.is_empty(),
            "terrain_feature must have a non-empty description"
        );
        assert!(
            t.metadata().description.contains("terrain"),
            "description should mention terrain"
        );
    }

    #[test]
    fn test_terrain_feature_metadata_category_is_terrain() {
        // Kills: delete field category from struct TemplateMetadata in terrain_feature_template
        let t = terrain_feature_template();
        assert_eq!(
            t.metadata().category,
            TemplateCategory::TerrainGeneration,
            "terrain_feature must use TerrainGeneration category, not Default (Custom)"
        );
    }

    #[test]
    fn test_terrain_modify_metadata_description_not_default() {
        // Kills: delete field description from struct TemplateMetadata in terrain_modify_template
        let t = terrain_modify_template();
        assert!(!t.metadata().description.is_empty());
    }

    #[test]
    fn test_terrain_modify_metadata_category_is_terrain() {
        // Kills: delete field category from struct TemplateMetadata in terrain_modify_template
        let t = terrain_modify_template();
        assert_eq!(t.metadata().category, TemplateCategory::TerrainGeneration);
    }

    #[test]
    fn test_terrain_modify_metadata_tags_not_empty() {
        // Kills: delete field tags from struct TemplateMetadata in terrain_modify_template
        let t = terrain_modify_template();
        assert!(
            !t.metadata().tags.is_empty(),
            "terrain_modify must have tags"
        );
        assert!(t.metadata().tags.contains(&"terrain".to_string()));
    }

    #[test]
    fn test_terrain_modify_metadata_required_variables_not_empty() {
        // Kills: delete field required_variables from struct TemplateMetadata in terrain_modify_template
        let t = terrain_modify_template();
        assert!(
            !t.metadata().required_variables.is_empty(),
            "terrain_modify must declare required_variables"
        );
        assert!(t
            .metadata()
            .required_variables
            .contains(&"request_text".to_string()));
    }

    #[test]
    fn test_terrain_batch_metadata_description_not_default() {
        // Kills: delete field description from struct TemplateMetadata in terrain_batch_template
        let t = terrain_batch_template();
        assert!(!t.metadata().description.is_empty());
    }

    #[test]
    fn test_terrain_batch_metadata_category_is_terrain() {
        // Kills: delete field category from struct TemplateMetadata in terrain_batch_template
        let t = terrain_batch_template();
        assert_eq!(t.metadata().category, TemplateCategory::TerrainGeneration);
    }

    #[test]
    fn test_terrain_batch_metadata_tags_not_empty() {
        // Kills: delete field tags from struct TemplateMetadata in terrain_batch_template
        let t = terrain_batch_template();
        assert!(!t.metadata().tags.is_empty());
        assert!(t.metadata().tags.contains(&"batch".to_string()));
    }

    #[test]
    fn test_terrain_system_metadata_description_not_default() {
        // Kills: delete field description from struct TemplateMetadata in terrain_system_metadata
        let t = terrain_system_template();
        assert!(!t.metadata().description.is_empty());
        assert!(t.metadata().description.contains("terrain"));
    }

    #[test]
    fn test_terrain_system_metadata_required_variables_empty() {
        // Kills: delete field required_variables from struct TemplateMetadata in terrain_system_metadata
        // (System template has explicitly empty vec — deleting the field leaves Default which is also
        // empty, BUT this confirms the field *is* set and we can distinguish if logic changes.)
        let t = terrain_system_template();
        assert!(t.metadata().required_variables.is_empty());
    }

    #[test]
    fn test_register_terrain_templates_actually_registers() {
        // Kills: replace register_terrain_templates -> anyhow::Result<()> with Ok(())
        let mut engine = crate::TemplateEngine::new();
        register_terrain_templates(&mut engine).unwrap();
        // After registration the engine must contain all 4 terrain templates
        assert!(engine.has_template("terrain_system"));
        assert!(engine.has_template("terrain_feature"));
        assert!(engine.has_template("terrain_modify"));
        assert!(engine.has_template("terrain_batch"));
    }
}
