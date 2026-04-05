use anyhow::Result;
use astraweave_core::{Constraints, ToolRegistry, ToolSpec};
use astraweave_llm::parse_llm_plan;
use std::collections::BTreeMap;
use std::fs;

fn main() -> Result<()> {
    // Prefer the integration-assembled file, fall back to probe output
    let path_primary = "target/ollama_assistant_acc.txt";
    let path_fallback = "target/ollama_probe_assistant_acc.txt";
    let s = if std::path::Path::new(path_primary).exists() {
        fs::read_to_string(path_primary).expect("Failed to read target/ollama_assistant_acc.txt")
    } else {
        fs::read_to_string(path_fallback)
            .expect("Failed to read target/ollama_probe_assistant_acc.txt")
    };
    println!("Assembled assistant file content:\n---\n{}\n---", s);

    // Build a minimal ToolRegistry matching the example's registry shape to validate parsing
    let mut tools = vec![];

    tools.push(ToolSpec {
        name: "move_to".into(),
        args: [("x".into(), "i32".into()), ("y".into(), "i32".into())]
            .into_iter()
            .collect(),
    });

    tools.push(ToolSpec {
        name: "throw".into(),
        args: [
            ("item".into(), "enum[smoke,grenade,flashbang]".into()),
            ("x".into(), "i32".into()),
            ("y".into(), "i32".into()),
        ]
        .into_iter()
        .collect(),
    });

    tools.push(ToolSpec {
        name: "cover_fire".into(),
        args: [
            ("duration".into(), "f32".into()),
            ("target_id".into(), "u32".into()),
        ]
        .into_iter()
        .collect(),
    });

    tools.push(ToolSpec {
        name: "revive".into(),
        args: [("ally_id".into(), "u32".into())].into_iter().collect(),
    });

    let reg = ToolRegistry {
        tools,
        constraints: Constraints {
            enforce_cooldowns: true,
            enforce_los: true,
            enforce_stamina: true,
        },
    };

    // First try direct parsing
    match parse_llm_plan(&s, &reg) {
        Ok(plan) => {
            println!("Successfully parsed PlanIntent:\n{:#?}", plan);
            return Ok(());
        }
        Err(e) => {
            println!("Initial parse failed: {}. Attempting cleanup...", e);
        }
    }

    // Cleanup: remove optional BOM and code fences, then retry
    let mut cleaned = s.clone();
    // Remove UTF-8 BOM if present
    if cleaned.starts_with('\u{FEFF}') {
        cleaned = cleaned.trim_start_matches('\u{FEFF}').to_string();
    }

    // Strip triple-backtick fences (```json ... ``` or ``` ... ```)
    if let Some(start) = cleaned.find("```json") {
        if let Some(end_rel) = cleaned[start + 7..].find("```") {
            cleaned = cleaned[start + 7..start + 7 + end_rel].trim().to_string();
        }
    } else if let Some(start) = cleaned.find("```") {
        if let Some(end_rel) = cleaned[start + 3..].find("```") {
            cleaned = cleaned[start + 3..start + 3 + end_rel].trim().to_string();
        }
    }

    println!("Cleaned content to parse:\n---\n{}\n---", cleaned);

    match parse_llm_plan(&cleaned, &reg) {
        Ok(plan) => {
            println!("Successfully parsed PlanIntent after cleanup:\n{:#?}", plan);
            Ok(())
        }
        Err(e) => {
            println!("Failed to parse plan after cleanup: {}", e);
            Err(e.into())
        }
    }
}
