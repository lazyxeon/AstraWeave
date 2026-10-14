use serde::{Deserialize, Serialize};

/// Minimal persona and companion types to satisfy cross-crate references in tests and examples.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Episode {
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Persona {
    pub name: String,
    pub likes: Vec<String>,
    pub dislikes: Vec<String>,
    // Additional fields expected by consumers
    pub tone: String,
    pub risk: String,
    pub humor: String,
    pub voice: String,
    pub backstory: String,
    pub goals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Fact {
    pub k: String,
    pub v: String,
    /// Optional type/metadata for the fact
    pub t: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Skill {
    pub name: String,
    pub level: u8,
    /// Additional notes about the skill
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionProfile {
    pub id: String,
    pub persona: Persona,
    pub episodes: Vec<Episode>,
    pub facts: Vec<Fact>,
    /// Player preferences (free-form JSON)
    pub player_prefs: serde_json::Value,
    /// Companion skills
    pub skills: Vec<Skill>,
    /// Lower-case hex SHA-256 of the canonical profile bytes. Empty until [`Self::sign`].
    #[serde(default)]
    pub signature: String,
}

impl CompanionProfile {
    pub fn new_default() -> Self {
        Self {
            id: "companion_default".to_string(),
            persona: Persona::default(),
            episodes: Vec::new(),
            facts: Vec::new(),
            player_prefs: serde_json::Value::Null,
            skills: Vec::new(),
            signature: String::new(),
        }
    }

    pub fn distill(&mut self) {
        // naive distill: turn episodes into facts
        for e in &self.episodes {
            self.facts.push(Fact {
                k: e.title.clone(),
                v: e.summary.clone(),
                t: "".to_string(),
            });
        }
    }

    pub fn load_from_file(_p: &str) -> anyhow::Result<Self> {
        Ok(CompanionProfile::new_default())
    }

    /// Replace `signature` with the SHA-256 of the canonical profile bytes.
    ///
    /// The digest does not include the previous signature. Facts, skills, and
    /// episodes are hashed in sorted order so `HashMap` or `Vec` order cannot
    /// change the bytes.
    pub fn sign(&mut self) {
        self.signature.clear();
        let digest = sha256(&canonical_profile_bytes(self));
        self.signature = hex_encode(&digest);
    }

    /// Save the profile to a file (placeholder)
    pub fn save_to_file(&self, _p: &str) -> anyhow::Result<()> {
        // In tests/examples we don't actually write to disk; return Ok
        Ok(())
    }

    /// True when `signature` matches a fresh digest of the canonical bytes.
    ///
    /// An empty signature does not verify.
    pub fn verify(&self) -> bool {
        if self.signature.is_empty() {
            return false;
        }
        let mut unsigned = self.clone();
        let expected = std::mem::take(&mut unsigned.signature);
        let digest = sha256(&canonical_profile_bytes(&unsigned));
        hex_encode(&digest) == expected
    }

    /// Check if profile has any episodes
    pub fn has_episodes(&self) -> bool {
        !self.episodes.is_empty()
    }

    /// Count the number of episodes
    pub fn episode_count(&self) -> usize {
        self.episodes.len()
    }

    /// Check if profile has any facts
    pub fn has_facts(&self) -> bool {
        !self.facts.is_empty()
    }

    /// Count the number of facts
    pub fn fact_count(&self) -> usize {
        self.facts.len()
    }

    /// Check if profile has any skills
    pub fn has_skills(&self) -> bool {
        !self.skills.is_empty()
    }

    /// Count the number of skills
    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }

    /// Add an episode to the profile
    pub fn add_episode(&mut self, episode: Episode) {
        self.episodes.push(episode);
    }

    /// Add a fact to the profile
    pub fn add_fact(&mut self, fact: Fact) {
        self.facts.push(fact);
    }

    /// Add a skill to the profile
    pub fn add_skill(&mut self, skill: Skill) {
        self.skills.push(skill);
    }

    /// Get a skill by name
    pub fn get_skill(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

    /// Get a fact by key
    pub fn get_fact(&self, key: &str) -> Option<&Fact> {
        self.facts.iter().find(|f| f.k == key)
    }
}

fn canonical_profile_bytes(profile: &CompanionProfile) -> Vec<u8> {
    let mut bytes = Vec::new();
    push_str(&mut bytes, &profile.id);
    push_str(&mut bytes, &profile.persona.name);
    push_str(&mut bytes, &profile.persona.tone);
    push_str(&mut bytes, &profile.persona.risk);
    push_str(&mut bytes, &profile.persona.humor);
    push_str(&mut bytes, &profile.persona.voice);
    push_str(&mut bytes, &profile.persona.backstory);
    let mut likes = profile.persona.likes.clone();
    likes.sort();
    for like in likes {
        push_str(&mut bytes, &like);
    }
    let mut dislikes = profile.persona.dislikes.clone();
    dislikes.sort();
    for dislike in dislikes {
        push_str(&mut bytes, &dislike);
    }
    let mut goals = profile.persona.goals.clone();
    goals.sort();
    for goal in goals {
        push_str(&mut bytes, &goal);
    }
    let mut episodes = profile.episodes.clone();
    episodes.sort_by(|a, b| (&a.title, &a.summary, &a.ts).cmp(&(&b.title, &b.summary, &b.ts)));
    for episode in episodes {
        push_str(&mut bytes, &episode.title);
        push_str(&mut bytes, &episode.summary);
        push_str(&mut bytes, &episode.ts);
        let mut tags = episode.tags.clone();
        tags.sort();
        for tag in tags {
            push_str(&mut bytes, &tag);
        }
    }
    let mut facts = profile.facts.clone();
    facts.sort_by(|a, b| (&a.k, &a.v, &a.t).cmp(&(&b.k, &b.v, &b.t)));
    for fact in facts {
        push_str(&mut bytes, &fact.k);
        push_str(&mut bytes, &fact.v);
        push_str(&mut bytes, &fact.t);
    }
    let mut skills = profile.skills.clone();
    skills.sort_by(|a, b| (&a.name, a.level, &a.notes).cmp(&(&b.name, b.level, &b.notes)));
    for skill in skills {
        push_str(&mut bytes, &skill.name);
        bytes.extend_from_slice(&skill.level.to_le_bytes());
        push_str(&mut bytes, &skill.notes);
    }
    let prefs = serde_json::to_string(&profile.player_prefs).unwrap_or_else(|_| "null".to_string());
    push_str(&mut bytes, &prefs);
    bytes
}

fn push_str(bytes: &mut Vec<u8>, value: &str) {
    let len = u32::try_from(value.len()).unwrap_or(u32::MAX);
    bytes.extend_from_slice(&len.to_le_bytes());
    let take = len as usize;
    bytes.extend_from_slice(&value.as_bytes()[..take.min(value.len())]);
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn sha256(message: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (message.len() as u64).wrapping_mul(8);
    let mut padded = message.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
    let mut out = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====================================================================
    // Episode Tests
    // ====================================================================

    #[test]
    fn test_episode_default() {
        let episode = Episode::default();
        assert!(episode.title.is_empty());
        assert!(episode.summary.is_empty());
        assert!(episode.tags.is_empty());
        assert!(episode.ts.is_empty());
    }

    #[test]
    fn test_episode_with_values() {
        let episode = Episode {
            title: "Battle of Dawn".to_string(),
            summary: "A fierce battle at dawn".to_string(),
            tags: vec!["combat".to_string(), "victory".to_string()],
            ts: "2025-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(episode.title, "Battle of Dawn");
        assert_eq!(episode.tags.len(), 2);
    }

    #[test]
    fn test_episode_clone() {
        let episode = Episode {
            title: "Test".to_string(),
            summary: "Summary".to_string(),
            tags: vec!["tag1".to_string()],
            ts: "now".to_string(),
        };
        let cloned = episode.clone();
        assert_eq!(episode.title, cloned.title);
        assert_eq!(episode.summary, cloned.summary);
    }

    #[test]
    fn test_episode_debug() {
        let episode = Episode::default();
        let debug_str = format!("{:?}", episode);
        assert!(debug_str.contains("Episode"));
    }

    // ====================================================================
    // Persona Tests
    // ====================================================================

    #[test]
    fn test_persona_default() {
        let persona = Persona::default();
        assert!(persona.name.is_empty());
        assert!(persona.likes.is_empty());
        assert!(persona.dislikes.is_empty());
        assert!(persona.tone.is_empty());
    }

    #[test]
    fn test_persona_with_values() {
        let persona = Persona {
            name: "Elara".to_string(),
            likes: vec!["music".to_string(), "adventure".to_string()],
            dislikes: vec!["cowardice".to_string()],
            tone: "friendly".to_string(),
            risk: "moderate".to_string(),
            humor: "witty".to_string(),
            voice: "melodic".to_string(),
            backstory: "Born in the mountains...".to_string(),
            goals: vec!["Find the artifact".to_string()],
        };
        assert_eq!(persona.name, "Elara");
        assert_eq!(persona.likes.len(), 2);
        assert_eq!(persona.dislikes.len(), 1);
    }

    #[test]
    fn test_persona_clone() {
        let persona = Persona {
            name: "Clone Test".to_string(),
            likes: vec!["testing".to_string()],
            ..Default::default()
        };
        let cloned = persona.clone();
        assert_eq!(persona.name, cloned.name);
    }

    // ====================================================================
    // Fact Tests
    // ====================================================================

    #[test]
    fn test_fact_default() {
        let fact = Fact::default();
        assert!(fact.k.is_empty());
        assert!(fact.v.is_empty());
        assert!(fact.t.is_empty());
    }

    #[test]
    fn test_fact_with_values() {
        let fact = Fact {
            k: "location".to_string(),
            v: "Enchanted Forest".to_string(),
            t: "place".to_string(),
        };
        assert_eq!(fact.k, "location");
        assert_eq!(fact.v, "Enchanted Forest");
        assert_eq!(fact.t, "place");
    }

    #[test]
    fn test_fact_clone() {
        let fact = Fact {
            k: "key".to_string(),
            v: "value".to_string(),
            t: "type".to_string(),
        };
        let cloned = fact.clone();
        assert_eq!(fact.k, cloned.k);
    }

    // ====================================================================
    // Skill Tests
    // ====================================================================

    #[test]
    fn test_skill_default() {
        let skill = Skill::default();
        assert!(skill.name.is_empty());
        assert_eq!(skill.level, 0);
        assert!(skill.notes.is_empty());
    }

    #[test]
    fn test_skill_with_values() {
        let skill = Skill {
            name: "Swordsmanship".to_string(),
            level: 75,
            notes: "Expert level fighter".to_string(),
        };
        assert_eq!(skill.name, "Swordsmanship");
        assert_eq!(skill.level, 75);
    }

    #[test]
    fn test_skill_clone() {
        let skill = Skill {
            name: "Archery".to_string(),
            level: 50,
            notes: "Intermediate".to_string(),
        };
        let cloned = skill.clone();
        assert_eq!(skill.name, cloned.name);
        assert_eq!(skill.level, cloned.level);
    }

    // ====================================================================
    // CompanionProfile Construction Tests
    // ====================================================================

    #[test]
    fn test_companion_profile_new_default() {
        let profile = CompanionProfile::new_default();
        assert_eq!(profile.id, "companion_default");
        assert!(profile.episodes.is_empty());
        assert!(profile.facts.is_empty());
        assert!(profile.skills.is_empty());
    }

    #[test]
    fn test_companion_profile_with_custom_id() {
        let mut profile = CompanionProfile::new_default();
        profile.id = "custom_companion".to_string();
        assert_eq!(profile.id, "custom_companion");
    }

    #[test]
    fn test_companion_profile_clone() {
        let profile = CompanionProfile::new_default();
        let cloned = profile.clone();
        assert_eq!(profile.id, cloned.id);
    }

    #[test]
    fn test_companion_profile_debug() {
        let profile = CompanionProfile::new_default();
        let debug_str = format!("{:?}", profile);
        assert!(debug_str.contains("CompanionProfile"));
    }

    // ====================================================================
    // CompanionProfile Episode Tests
    // ====================================================================

    #[test]
    fn test_companion_profile_has_episodes_empty() {
        let profile = CompanionProfile::new_default();
        assert!(!profile.has_episodes());
    }

    #[test]
    fn test_companion_profile_has_episodes_with_content() {
        let mut profile = CompanionProfile::new_default();
        profile.add_episode(Episode {
            title: "Test Episode".to_string(),
            ..Default::default()
        });
        assert!(profile.has_episodes());
    }

    #[test]
    fn test_companion_profile_episode_count() {
        let mut profile = CompanionProfile::new_default();
        assert_eq!(profile.episode_count(), 0);

        profile.add_episode(Episode::default());
        assert_eq!(profile.episode_count(), 1);

        profile.add_episode(Episode::default());
        assert_eq!(profile.episode_count(), 2);
    }

    #[test]
    fn test_companion_profile_add_episode() {
        let mut profile = CompanionProfile::new_default();
        let episode = Episode {
            title: "New Episode".to_string(),
            summary: "Something happened".to_string(),
            tags: vec!["important".to_string()],
            ts: "today".to_string(),
        };

        profile.add_episode(episode);
        assert_eq!(profile.episode_count(), 1);
        assert_eq!(profile.episodes[0].title, "New Episode");
    }

    // ====================================================================
    // CompanionProfile Fact Tests
    // ====================================================================

    #[test]
    fn test_companion_profile_has_facts_empty() {
        let profile = CompanionProfile::new_default();
        assert!(!profile.has_facts());
    }

    #[test]
    fn test_companion_profile_has_facts_with_content() {
        let mut profile = CompanionProfile::new_default();
        profile.add_fact(Fact {
            k: "key".to_string(),
            v: "value".to_string(),
            t: "".to_string(),
        });
        assert!(profile.has_facts());
    }

    #[test]
    fn test_companion_profile_fact_count() {
        let mut profile = CompanionProfile::new_default();
        assert_eq!(profile.fact_count(), 0);

        profile.add_fact(Fact::default());
        assert_eq!(profile.fact_count(), 1);
    }

    #[test]
    fn test_companion_profile_add_fact() {
        let mut profile = CompanionProfile::new_default();
        let fact = Fact {
            k: "player_name".to_string(),
            v: "Hero".to_string(),
            t: "meta".to_string(),
        };

        profile.add_fact(fact);
        assert_eq!(profile.fact_count(), 1);
        assert_eq!(profile.facts[0].k, "player_name");
    }

    #[test]
    fn test_companion_profile_get_fact() {
        let mut profile = CompanionProfile::new_default();
        profile.add_fact(Fact {
            k: "test_key".to_string(),
            v: "test_value".to_string(),
            t: "".to_string(),
        });

        let fact = profile.get_fact("test_key");
        assert!(fact.is_some());
        assert_eq!(fact.unwrap().v, "test_value");
    }

    #[test]
    fn test_companion_profile_get_fact_nonexistent() {
        let profile = CompanionProfile::new_default();
        assert!(profile.get_fact("missing").is_none());
    }

    // ====================================================================
    // CompanionProfile Skill Tests
    // ====================================================================

    #[test]
    fn test_companion_profile_has_skills_empty() {
        let profile = CompanionProfile::new_default();
        assert!(!profile.has_skills());
    }

    #[test]
    fn test_companion_profile_has_skills_with_content() {
        let mut profile = CompanionProfile::new_default();
        profile.add_skill(Skill {
            name: "Combat".to_string(),
            level: 50,
            notes: "".to_string(),
        });
        assert!(profile.has_skills());
    }

    #[test]
    fn test_companion_profile_skill_count() {
        let mut profile = CompanionProfile::new_default();
        assert_eq!(profile.skill_count(), 0);

        profile.add_skill(Skill::default());
        assert_eq!(profile.skill_count(), 1);
    }

    #[test]
    fn test_companion_profile_add_skill() {
        let mut profile = CompanionProfile::new_default();
        let skill = Skill {
            name: "Magic".to_string(),
            level: 80,
            notes: "Fire specialist".to_string(),
        };

        profile.add_skill(skill);
        assert_eq!(profile.skill_count(), 1);
        assert_eq!(profile.skills[0].name, "Magic");
    }

    #[test]
    fn test_companion_profile_get_skill() {
        let mut profile = CompanionProfile::new_default();
        profile.add_skill(Skill {
            name: "Stealth".to_string(),
            level: 65,
            notes: "Shadow walker".to_string(),
        });

        let skill = profile.get_skill("Stealth");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().level, 65);
    }

    #[test]
    fn test_companion_profile_get_skill_nonexistent() {
        let profile = CompanionProfile::new_default();
        assert!(profile.get_skill("NonExistent").is_none());
    }

    // ====================================================================
    // CompanionProfile Distill Tests
    // ====================================================================

    #[test]
    fn test_companion_profile_distill_empty() {
        let mut profile = CompanionProfile::new_default();
        profile.distill();
        assert!(!profile.has_facts()); // No episodes to distill
    }

    #[test]
    fn test_companion_profile_distill_with_episodes() {
        let mut profile = CompanionProfile::new_default();
        profile.add_episode(Episode {
            title: "First Battle".to_string(),
            summary: "Won against goblins".to_string(),
            tags: vec![],
            ts: "".to_string(),
        });
        profile.add_episode(Episode {
            title: "Quest Complete".to_string(),
            summary: "Found the treasure".to_string(),
            tags: vec![],
            ts: "".to_string(),
        });

        profile.distill();
        assert_eq!(profile.fact_count(), 2);
        assert!(profile.get_fact("First Battle").is_some());
        assert!(profile.get_fact("Quest Complete").is_some());
    }

    #[test]
    fn test_companion_profile_distill_preserves_episode_info() {
        let mut profile = CompanionProfile::new_default();
        profile.add_episode(Episode {
            title: "Title".to_string(),
            summary: "Summary text".to_string(),
            tags: vec![],
            ts: "".to_string(),
        });

        profile.distill();
        let fact = profile.get_fact("Title").unwrap();
        assert_eq!(fact.v, "Summary text");
    }

    // ====================================================================
    // CompanionProfile Load/Save/Verify Tests
    // ====================================================================

    #[test]
    fn test_companion_profile_load_from_file() {
        let result = CompanionProfile::load_from_file("any_path");
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.id, "companion_default");
    }

    #[test]
    fn test_companion_profile_save_to_file() {
        let profile = CompanionProfile::new_default();
        let result = profile.save_to_file("test_path");
        assert!(result.is_ok());
    }

    #[test]
    fn test_companion_profile_sign() {
        let mut profile = CompanionProfile::new_default();
        profile.sign(); // Should not panic
    }

    #[test]
    fn test_companion_profile_verify() {
        let mut profile = CompanionProfile::new_default();
        assert!(!profile.verify());
        profile.sign();
        assert!(profile.verify());
        profile.persona.name = "tampered".to_string();
        assert!(!profile.verify());
    }

    #[test]
    fn test_sha256_abc() {
        let digest = sha256(b"abc");
        assert_eq!(
            hex_encode(&digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_sign_ignores_fact_order() {
        let mut left = CompanionProfile::new_default();
        left.add_fact(Fact {
            k: "b".into(),
            v: "2".into(),
            t: "n".into(),
        });
        left.add_fact(Fact {
            k: "a".into(),
            v: "1".into(),
            t: "n".into(),
        });
        left.sign();
        let mut right = CompanionProfile::new_default();
        right.add_fact(Fact {
            k: "a".into(),
            v: "1".into(),
            t: "n".into(),
        });
        right.add_fact(Fact {
            k: "b".into(),
            v: "2".into(),
            t: "n".into(),
        });
        right.sign();
        assert_eq!(left.signature, right.signature);
    }

    // ====================================================================
    // Serialization Tests
    // ====================================================================

    #[test]
    fn test_episode_serialization() {
        let episode = Episode {
            title: "Serialize Me".to_string(),
            summary: "A test".to_string(),
            tags: vec!["test".to_string()],
            ts: "2025".to_string(),
        };

        let json = serde_json::to_string(&episode).unwrap();
        let deserialized: Episode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Serialize Me");
    }

    #[test]
    fn test_persona_serialization() {
        let persona = Persona {
            name: "Test".to_string(),
            likes: vec!["coding".to_string()],
            dislikes: vec![],
            tone: "calm".to_string(),
            risk: "low".to_string(),
            humor: "dry".to_string(),
            voice: "quiet".to_string(),
            backstory: "Story".to_string(),
            goals: vec!["finish".to_string()],
        };

        let json = serde_json::to_string(&persona).unwrap();
        let deserialized: Persona = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test");
    }

    #[test]
    fn test_fact_serialization() {
        let fact = Fact {
            k: "key".to_string(),
            v: "value".to_string(),
            t: "type".to_string(),
        };

        let json = serde_json::to_string(&fact).unwrap();
        let deserialized: Fact = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.k, "key");
    }

    #[test]
    fn test_skill_serialization() {
        let skill = Skill {
            name: "Swimming".to_string(),
            level: 42,
            notes: "Good form".to_string(),
        };

        let json = serde_json::to_string(&skill).unwrap();
        let deserialized: Skill = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Swimming");
        assert_eq!(deserialized.level, 42);
    }

    #[test]
    fn test_companion_profile_serialization() {
        let mut profile = CompanionProfile::new_default();
        profile.add_episode(Episode {
            title: "Test".to_string(),
            ..Default::default()
        });
        profile.add_fact(Fact {
            k: "k".to_string(),
            v: "v".to_string(),
            t: "".to_string(),
        });
        profile.add_skill(Skill {
            name: "Skill".to_string(),
            level: 10,
            notes: "".to_string(),
        });

        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: CompanionProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "companion_default");
        assert_eq!(deserialized.episode_count(), 1);
        assert_eq!(deserialized.fact_count(), 1);
        assert_eq!(deserialized.skill_count(), 1);
    }

    // ====================================================================
    // Edge Case Tests
    // ====================================================================

    #[test]
    fn test_profile_multiple_skills_same_name() {
        let mut profile = CompanionProfile::new_default();
        profile.add_skill(Skill {
            name: "Combat".to_string(),
            level: 10,
            notes: "First".to_string(),
        });
        profile.add_skill(Skill {
            name: "Combat".to_string(),
            level: 20,
            notes: "Second".to_string(),
        });

        // get_skill returns first match
        let skill = profile.get_skill("Combat");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().level, 10);
    }

    #[test]
    fn test_profile_unicode_content() {
        let mut profile = CompanionProfile::new_default();
        profile.persona.name = "日本語キャラクター".to_string();
        profile.add_episode(Episode {
            title: "冒険の始まり".to_string(),
            summary: "🎮 ゲーム開始!".to_string(),
            tags: vec!["日本語".to_string()],
            ts: "".to_string(),
        });

        assert_eq!(profile.persona.name, "日本語キャラクター");
        assert_eq!(profile.episodes[0].title, "冒険の始まり");
    }

    #[test]
    fn test_profile_empty_strings() {
        let mut profile = CompanionProfile::new_default();
        profile.add_fact(Fact {
            k: "".to_string(),
            v: "".to_string(),
            t: "".to_string(),
        });

        let fact = profile.get_fact("");
        assert!(fact.is_some());
    }

    #[test]
    fn test_profile_player_prefs_json() {
        let mut profile = CompanionProfile::new_default();
        profile.player_prefs =
            serde_json::json!({"difficulty": "hard", "volume": 75, "hints": false});

        if let serde_json::Value::Object(map) = &profile.player_prefs {
            assert!(map.contains_key("difficulty"));
            assert!(map.contains_key("volume"));
        } else {
            panic!("Expected object");
        }
    }
}
