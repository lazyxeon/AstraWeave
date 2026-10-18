#![forbid(unsafe_code)]
use anyhow::Result;
use astraweave_ecs::{App, Plugin, World};

/// Configuration for {{ProjectName}} plugin
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct {{ProjectName}}Config {
    // Add configuration fields here
}

impl Default for {{ProjectName}}Config {
    fn default() -> Self {
        Self {
            // Set defaults
        }
    }
}

/// Plugin for {{ProjectName}} functionality
pub struct {{ProjectName}}Plugin {
    config: {{ProjectName}}Config,
}

impl {{ProjectName}}Plugin {
    pub fn new(config: {{ProjectName}}Config) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self::new({{ProjectName}}Config::default())
    }
}

impl Plugin for {{ProjectName}}Plugin {
    fn build(&self, app: &mut App) {
        // Insert resources
        app.world.insert_resource(self.config.clone());

        // Add systems
        app.add_system_known_stage("{{stage}}", {{project_name}}_system);
    }
}

/// System for {{ProjectName}} logic
fn {{project_name}}_system(world: &mut World) {
    // Implement system logic here
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_ecs::App;

    #[test]
    fn test_plugin_build() {
        let mut app = App::new();
        let plugin = {{ProjectName}}Plugin::default();
        plugin.build(&mut app);

        // Add assertions
    }
}