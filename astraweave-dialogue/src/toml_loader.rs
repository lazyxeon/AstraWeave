//! TOML dialogue loader — parses Veilweaver dialogue TOML files into [`DialogueGraph`].
//!
//! The TOML format uses:
//! ```toml
//! id = "intro"
//! start = "n0"
//!
//! [[nodes]]
//! id = "n0"
//! line = { speaker = "Companion", text = "Hello." }
//! choices = [{ text = "Reply", go_to = "n1" }]
//!
//! [[nodes]]
//! id = "n1"
//! line = { speaker = "Companion", text = "Goodbye." }
//! end = true
//! ```
//!
//! This is translated into the engine's [`DialogueGraph`] / [`DialogueNode`] / [`DialogueResponse`].

use crate::{DialogueGraph, DialogueNode, DialogueResponse};
use anyhow::{Context, Result};
use serde::Deserialize;

// ── TOML schema types ──────────────────────────────────────────────────────

/// Top-level TOML dialogue file.
#[derive(Debug, Deserialize)]
struct TomlDialogueFile {
    /// Identifier for this dialogue tree (e.g. "intro").
    id: String,
    /// Node ID to start playback from.
    start: String,
    /// All nodes in the dialogue.
    nodes: Vec<TomlDialogueNode>,
}

/// A single node in the TOML format.
#[derive(Debug, Deserialize)]
struct TomlDialogueNode {
    /// Unique node ID (e.g. "n0", "storm_stabilize").
    id: String,
    /// The dialogue line.
    line: TomlLine,
    /// Player choices. Empty / absent on terminal nodes.
    #[serde(default)]
    choices: Vec<TomlChoice>,
    /// If `true`, this node ends the dialogue (no choices expected).
    #[serde(default)]
    end: bool,
}

/// Speaker + text pair.
#[derive(Debug, Deserialize)]
struct TomlLine {
    speaker: String,
    text: String,
}

/// A single player choice.
#[derive(Debug, Deserialize)]
struct TomlChoice {
    text: String,
    go_to: String,
}

// ── Public API ─────────────────────────────────────────────────────────────

/// Result of loading a TOML dialogue file — contains the graph plus metadata
/// that the engine needs (start node, dialogue ID).
#[derive(Debug, Clone)]
pub struct LoadedDialogue {
    /// Human-readable ID of this dialogue tree (from the TOML `id` field).
    pub dialogue_id: String,
    /// Which node the runner should begin on.
    pub start_node: String,
    /// The dialogue graph containing all nodes and responses.
    pub graph: DialogueGraph,
}

/// Parses a TOML string into a [`LoadedDialogue`].
///
/// # Errors
/// Returns an error if the TOML is malformed or if `start` references a
/// node that doesn't exist.
pub fn load_dialogue_from_toml(toml_str: &str) -> Result<LoadedDialogue> {
    let file: TomlDialogueFile =
        toml::from_str(toml_str).context("Failed to parse dialogue TOML")?;

    let mut nodes = Vec::with_capacity(file.nodes.len());

    for toml_node in &file.nodes {
        // Build the text with speaker prefix so the UI layer can split later.
        let text = format!("[{}] {}", toml_node.line.speaker, toml_node.line.text);

        let responses: Vec<DialogueResponse> = if toml_node.end {
            // Terminal node — no responses.
            Vec::new()
        } else {
            toml_node
                .choices
                .iter()
                .map(|c| DialogueResponse::with_next(&c.text, &c.go_to))
                .collect()
        };

        nodes.push(DialogueNode {
            id: toml_node.id.clone(),
            text,
            responses,
        });
    }

    let graph = DialogueGraph::with_nodes(nodes);

    // Validate start node exists.
    anyhow::ensure!(
        graph.has_node(&file.start),
        "start node '{}' not found in dialogue '{}'",
        file.start,
        file.id
    );

    // Validate graph integrity — all next_id references must resolve.
    graph
        .validate()
        .map_err(|e| anyhow::anyhow!("Dialogue graph validation failed: {}", e))?;

    Ok(LoadedDialogue {
        dialogue_id: file.id,
        start_node: file.start,
        graph,
    })
}

/// Convenience: load a dialogue from a file path on disk.
///
/// # Errors
/// Returns an error if the file cannot be read or parsed.
pub fn load_dialogue_from_file(path: &std::path::Path) -> Result<LoadedDialogue> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read dialogue file: {}", path.display()))?;
    load_dialogue_from_toml(&content)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_TOML: &str = r#"
id = "test"
start = "n0"

[[nodes]]
id = "n0"
line = { speaker = "NPC", text = "Hello." }
choices = [{ text = "Hi", go_to = "n1" }]

[[nodes]]
id = "n1"
line = { speaker = "NPC", text = "Goodbye." }
end = true
"#;

    #[test]
    fn load_minimal_dialogue() {
        let loaded = load_dialogue_from_toml(MINIMAL_TOML).unwrap();
        assert_eq!(loaded.dialogue_id, "test");
        assert_eq!(loaded.start_node, "n0");
        assert_eq!(loaded.graph.node_count(), 2);
        assert!(loaded.graph.is_valid());
    }

    #[test]
    fn terminal_nodes_have_no_responses() {
        let loaded = load_dialogue_from_toml(MINIMAL_TOML).unwrap();
        let n1 = loaded.graph.get_node("n1").unwrap();
        assert!(n1.is_terminal());
    }

    #[test]
    fn speaker_prefix_in_text() {
        let loaded = load_dialogue_from_toml(MINIMAL_TOML).unwrap();
        let n0 = loaded.graph.get_node("n0").unwrap();
        assert!(n0.text.starts_with("[NPC]"));
    }

    #[test]
    fn branching_dialogue() {
        let toml = r#"
id = "branch"
start = "root"

[[nodes]]
id = "root"
line = { speaker = "A", text = "Choose." }
choices = [
  { text = "Left", go_to = "left" },
  { text = "Right", go_to = "right" }
]

[[nodes]]
id = "left"
line = { speaker = "A", text = "You went left." }
end = true

[[nodes]]
id = "right"
line = { speaker = "A", text = "You went right." }
end = true
"#;
        let loaded = load_dialogue_from_toml(toml).unwrap();
        assert_eq!(loaded.graph.choice_count(), 1);
        assert_eq!(loaded.graph.terminal_count(), 2);
    }

    #[test]
    fn invalid_start_node_errors() {
        let toml = r#"
id = "bad"
start = "missing"

[[nodes]]
id = "n0"
line = { speaker = "A", text = "Solo." }
end = true
"#;
        let result = load_dialogue_from_toml(toml);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("missing"));
    }

    #[test]
    fn broken_reference_errors() {
        let toml = r#"
id = "broken"
start = "n0"

[[nodes]]
id = "n0"
line = { speaker = "A", text = "Broken ref." }
choices = [{ text = "Go", go_to = "does_not_exist" }]
"#;
        let result = load_dialogue_from_toml(toml);
        assert!(result.is_err());
    }

    #[test]
    fn player_facing_assets_do_not_say_veilweaver() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets");
        let mut files = vec![
            root.join("dialogue_intro.toml"),
            root.join("quests_main.toml"),
            root.join("echoes.toml"),
            root.join("voices.toml"),
        ];
        for dir in [root.join("npc"), root.join("cinematics")] {
            if !dir.is_dir() {
                continue;
            }
            let mut nested: Vec<_> = std::fs::read_dir(&dir)
                .unwrap_or_else(|err| panic!("reading {}: {err}", dir.display()))
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.is_file())
                .collect();
            nested.sort();
            files.extend(nested);
        }
        let mut hits = Vec::new();
        for path in files {
            if !path.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
            if text.to_ascii_lowercase().contains("veilweaver") {
                hits.push(path.display().to_string());
            }
        }
        assert!(
            hits.is_empty(),
            "player-facing assets must not say Veilweaver: {hits:?}"
        );
    }

    #[test]
    fn load_actual_intro_dialogue() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/dialogue_intro.toml");
        if path.exists() {
            let loaded = load_dialogue_from_file(&path).unwrap();
            assert_eq!(loaded.dialogue_id, "intro");
            assert_eq!(loaded.start_node, "n0");
            assert!(loaded.graph.node_count() >= 15);
            assert!(loaded.graph.is_valid());
            // Storm choice node must be present.
            assert!(loaded.graph.has_node("storm_stabilize"));
            assert!(loaded.graph.has_node("storm_redirect"));
        }
    }
}
