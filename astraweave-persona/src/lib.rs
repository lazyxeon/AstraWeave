#![forbid(unsafe_code)]
//! # AstraWeave Persona
//!
//! AI personality system for NPCs with LLM-enhanced behavior.
//!
//! ## Features
//! - Load persona packs from zip files
//! - Define skills, facts, and personality traits
//! - LLM integration for dynamic responses
//!
//! ## Quick Start
//! ```ignore
//! let persona = load_persona_zip("npc_merchant.zip")?;
//! println!("Loaded: {}", persona.name);
//! ```

use anyhow::{anyhow, Result};
// persona types live in the `persona` module of `astraweave_memory`
use astraweave_memory::persona::{CompanionProfile, Fact, Skill};
use serde::Deserialize;

// Phase 2 LLM Integration
pub mod llm_persona;
pub use llm_persona::*;

#[derive(Deserialize)]
struct Manifest {
    tone: String,
    risk: String,
    humor: String,
    voice: String,
    #[serde(default)]
    prefs_json: Option<String>,
    #[serde(default)]
    skills: Option<Vec<SkillEntry>>,
    #[serde(default)]
    facts: Option<Vec<FactEntry>>,
}
#[derive(Deserialize)]
struct SkillEntry {
    name: String,
    level: u8,
    notes: String,
}
#[derive(Deserialize)]
struct FactEntry {
    k: String,
    v: String,
    t: String,
}

const MAX_PERSONA_ZIP_ENTRIES: usize = 10_000;
const MAX_PERSONA_UNCOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_PERSONA_MANIFEST_BYTES: u64 = 1024 * 1024;

pub fn load_persona_zip(path: &str) -> Result<CompanionProfile> {
    let file = std::fs::File::open(path)?;
    let mut zip = zip::ZipArchive::new(file)?;
    if zip.len() > MAX_PERSONA_ZIP_ENTRIES {
        return Err(anyhow!(
            "zip entry count exceeds cap ({} > {MAX_PERSONA_ZIP_ENTRIES})",
            zip.len()
        ));
    }
    let mut total = 0u64;
    for index in 0..zip.len() {
        let member = zip.by_index(index)?;
        total = total.saturating_add(member.size());
        if total > MAX_PERSONA_UNCOMPRESSED_BYTES {
            return Err(anyhow!(
                "zip uncompressed size exceeds cap ({total} > {MAX_PERSONA_UNCOMPRESSED_BYTES})"
            ));
        }
    }
    let mut manifest_txt = String::new();
    {
        let mf = zip
            .by_name("persona_manifest.toml")
            .map_err(|_| anyhow!("persona_manifest.toml missing"))?;
        if mf.size() > MAX_PERSONA_MANIFEST_BYTES {
            return Err(anyhow!(
                "persona manifest exceeds cap ({} > {MAX_PERSONA_MANIFEST_BYTES})",
                mf.size()
            ));
        }
        use std::io::Read;
        mf.take(MAX_PERSONA_MANIFEST_BYTES)
            .read_to_string(&mut manifest_txt)?;
    }
    let m: Manifest = toml::from_str(&manifest_txt)?;
    let mut p = CompanionProfile::new_default();
    // Populate persona fields
    p.persona.name = m.voice.clone();
    p.persona.tone = m.tone.clone();
    p.persona.risk = m.risk.clone();
    p.persona.humor = m.humor.clone();
    p.persona.voice = m.voice.clone();
    p.persona.backstory = "Loaded from manifest".to_string();

    // Player preferences (optional JSON)
    if let Some(js) = m.prefs_json {
        p.player_prefs = serde_json::from_str(&js).unwrap_or(serde_json::Value::Null);
    }

    // Skills
    if let Some(sk) = m.skills {
        p.skills = sk
            .into_iter()
            .map(|s| Skill {
                name: s.name,
                level: s.level,
                notes: s.notes,
            })
            .collect();
    }

    // Facts
    if let Some(fs) = m.facts {
        p.facts = fs
            .into_iter()
            .map(|f| Fact {
                k: f.k,
                v: f.v,
                t: f.t,
            })
            .collect();
    }

    // Sign the profile (no-op placeholder)
    p.sign();
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_deserialization_basic() {
        let toml_str = r#"
tone = "friendly"
risk = "low"
humor = "high"
voice = "TestBot"
"#;
        let manifest: Manifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.tone, "friendly");
        assert_eq!(manifest.risk, "low");
        assert_eq!(manifest.humor, "high");
        assert_eq!(manifest.voice, "TestBot");
        assert!(manifest.prefs_json.is_none());
        assert!(manifest.skills.is_none());
        assert!(manifest.facts.is_none());
    }

    #[test]
    fn test_manifest_deserialization_with_skills() {
        let toml_str = r#"
tone = "serious"
risk = "medium"
humor = "low"
voice = "Warrior"

[[skills]]
name = "Combat"
level = 5
notes = "Expert"
"#;
        let manifest: Manifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.skills.is_some());
        let skills = manifest.skills.unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "Combat");
        assert_eq!(skills[0].level, 5);
        assert_eq!(skills[0].notes, "Expert");
    }

    #[test]
    fn test_manifest_deserialization_with_facts() {
        let toml_str = r#"
tone = "curious"
risk = "high"
humor = "medium"
voice = "Explorer"

[[facts]]
k = "origin"
v = "Mars"
t = "background"
"#;
        let manifest: Manifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.facts.is_some());
        let facts = manifest.facts.unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].k, "origin");
        assert_eq!(facts[0].v, "Mars");
        assert_eq!(facts[0].t, "background");
    }

    #[test]
    fn test_manifest_deserialization_with_prefs() {
        let toml_str = r#"
tone = "helpful"
risk = "low"
humor = "low"
voice = "Assistant"
prefs_json = '{"theme": "dark"}'
"#;
        let manifest: Manifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.prefs_json.is_some());
        assert_eq!(manifest.prefs_json.unwrap(), r#"{"theme": "dark"}"#);
    }

    #[test]
    fn test_manifest_missing_required_field() {
        let toml_str = r#"
tone = "friendly"
risk = "low"
"#;
        let result: Result<Manifest, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_skill_entry_all_fields() {
        let toml_str = r#"
[[skills]]
name = "Hacking"
level = 8
notes = "Elite hacker"
"#;
        #[derive(Deserialize)]
        struct TestSkills {
            skills: Vec<SkillEntry>,
        }

        let parsed: TestSkills = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.skills[0].name, "Hacking");
        assert_eq!(parsed.skills[0].level, 8);
        assert_eq!(parsed.skills[0].notes, "Elite hacker");
    }

    #[test]
    fn test_fact_entry_all_fields() {
        let toml_str = r#"
[[facts]]
k = "age"
v = "25"
t = "personal"
"#;
        #[derive(Deserialize)]
        struct TestFacts {
            facts: Vec<FactEntry>,
        }

        let parsed: TestFacts = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.facts[0].k, "age");
        assert_eq!(parsed.facts[0].v, "25");
        assert_eq!(parsed.facts[0].t, "personal");
    }

    #[test]
    fn test_manifest_multiple_skills() {
        let toml_str = r#"
tone = "test"
risk = "test"
humor = "test"
voice = "Test"

[[skills]]
name = "Skill1"
level = 1
notes = "Note1"

[[skills]]
name = "Skill2"
level = 2
notes = "Note2"

[[skills]]
name = "Skill3"
level = 3
notes = "Note3"
"#;
        let manifest: Manifest = toml::from_str(toml_str).unwrap();
        let skills = manifest.skills.unwrap();
        assert_eq!(skills.len(), 3);
        assert_eq!(skills[2].name, "Skill3");
    }

    #[test]
    fn test_manifest_multiple_facts() {
        let toml_str = r#"
tone = "test"
risk = "test"
humor = "test"
voice = "Test"

[[facts]]
k = "fact1"
v = "value1"
t = "type1"

[[facts]]
k = "fact2"
v = "value2"
t = "type2"
"#;
        let manifest: Manifest = toml::from_str(toml_str).unwrap();
        let facts = manifest.facts.unwrap();
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[1].k, "fact2");
    }

    #[test]
    fn test_invalid_json_in_prefs() {
        let toml_str = r#"
tone = "test"
risk = "test"
humor = "test"
voice = "Test"
prefs_json = 'invalid json {'
"#;
        // TOML should parse fine, but the JSON will be invalid
        let manifest: Manifest = toml::from_str(toml_str).unwrap();
        assert!(manifest.prefs_json.is_some());

        // Test that invalid JSON is handled gracefully in load_persona_zip
        // (it returns Value::Null on error)
    }

    #[test]
    fn persona_zip_with_too_many_entries_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("persona.zip");
        let file = std::fs::File::create(&path).expect("zip");
        let mut zip = zip::ZipWriter::new(file);
        use std::io::Write;
        let opts = zip::write::SimpleFileOptions::default();
        for index in 0..(MAX_PERSONA_ZIP_ENTRIES + 1) {
            zip.start_file(format!("f{index}.txt"), opts)
                .expect("start");
            zip.write_all(b"x").expect("write");
        }
        zip.finish().expect("finish");
        let err = load_persona_zip(path.to_str().expect("utf8")).expect_err("over-cap zip");
        let msg = format!("{err:#}");
        assert!(msg.contains("exceeds cap"), "{msg}");
    }
}
