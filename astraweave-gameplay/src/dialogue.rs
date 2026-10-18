use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Cond {
    Eq { key: String, val: String },
    Ne { key: String, val: String },
    Has { key: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Line {
    pub speaker: String,
    pub text: String,
    #[serde(default)]
    pub set_vars: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Choice {
    pub text: String,
    pub go_to: String,
    #[serde(default)]
    pub require: Vec<Cond>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub line: Option<Line>,
    #[serde(default)]
    pub choices: Vec<Choice>,
    #[serde(default)]
    pub end: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dialogue {
    pub id: String,
    pub start: String,
    pub nodes: Vec<Node>,
}

pub struct DialogueState {
    pub idx: usize,
    pub map: HashMap<String, usize>,
    pub vars: HashMap<String, String>,
}

impl DialogueState {
    #[allow(clippy::expect_used)] // INVARIANT: d.start must reference a valid node ID in the dialogue
    pub fn new(d: &Dialogue) -> Self {
        let map: HashMap<String, usize> = d
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();
        let idx = *map.get(&d.start).expect("start");
        Self {
            idx,
            map,
            vars: HashMap::new(),
        }
    }
    pub fn current<'a>(&self, d: &'a Dialogue) -> &'a Node {
        &d.nodes[self.idx]
    }
    pub fn choose(&mut self, d: &Dialogue, choice_idx: usize) -> bool {
        let n = self.current(d);
        if let Some(c) = n.choices.get(choice_idx) {
            if !c.require.iter().all(|cond| eval(cond, &self.vars)) {
                return false;
            }
            if let Some(&ni) = self.map.get(&c.go_to) {
                // apply set_vars of next line when we move
                self.idx = ni;
                if let Some(l) = &d.nodes[ni].line {
                    for (k, v) in &l.set_vars {
                        self.vars.insert(k.clone(), v.clone());
                    }
                }
                return true;
            }
        }
        false
    }
}

fn eval(c: &Cond, vars: &HashMap<String, String>) -> bool {
    match c {
        Cond::Eq { key, val } => vars.get(key).map(|v| v == val).unwrap_or(false),
        Cond::Ne { key, val } => vars.get(key).map(|v| v != val).unwrap_or(true),
        Cond::Has { key } => vars.contains_key(key),
    }
}

/// “Compiler”: turn a simple banter script into Dialogue nodes.
/// Format:
///   [Speaker] line text
///   -> set var=value
///   ? key == value : goto node_id
pub fn compile_banter_to_nodes(id: &str, src: &str) -> Dialogue {
    let mut nodes = vec![];
    let mut i = 0usize;
    let mut _last_id = "n0".to_string();
    for line in src.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            // line
            let (speaker, text) = if let Some(end) = rest.find(']') {
                let spk = &rest[..end];
                let txt = rest[end + 1..].trim();
                (spk.to_string(), txt.to_string())
            } else {
                ("Unknown".into(), line.into())
            };
            let idn = format!("n{}", i);
            nodes.push(Node {
                id: idn.clone(),
                line: Some(Line {
                    speaker,
                    text,
                    set_vars: vec![],
                }),
                choices: vec![],
                end: false,
            });
            _last_id = idn;
            i += 1;
        } else if let Some(rest) = line.strip_prefix("->") {
            let kv = rest.trim();
            if let Some(eq) = kv.find('=') {
                let k = kv[..eq].trim().to_string();
                let v = kv[eq + 1..].trim().to_string();
                if let Some(n) = nodes.last_mut() {
                    if let Some(l) = n.line.as_mut() {
                        l.set_vars.push((k, v));
                    }
                }
            }
        } else if let Some(rest) = line.strip_prefix('?') {
            // condition goto
            // e.g., "? mood == happy : goto n2"
            let parts: Vec<_> = rest.split(':').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let cond = parts[0];
                let goto = parts[1]
                    .strip_prefix("goto")
                    .map(|s| s.trim())
                    .unwrap_or("n0")
                    .to_string();
                let mut conds = vec![];
                if cond.contains("==") {
                    let z: Vec<_> = cond.split("==").collect();
                    conds.push(Cond::Eq {
                        key: z[0].trim().into(),
                        val: z[1].trim().into(),
                    });
                } else if cond.contains("!=") {
                    let z: Vec<_> = cond.split("!=").collect();
                    conds.push(Cond::Ne {
                        key: z[0].trim().into(),
                        val: z[1].trim().into(),
                    });
                }
                if let Some(n) = nodes.last_mut() {
                    n.choices.push(Choice {
                        text: "Continue".into(),
                        go_to: goto,
                        require: conds,
                    });
                }
            }
        }
    }
    if let Some(n) = nodes.last_mut() {
        n.end = true;
    }
    Dialogue {
        id: id.into(),
        start: nodes.first().map(|n| n.id.clone()).unwrap_or("n0".into()),
        nodes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub use crate::test_support::TestValue;

    fn create_simple_dialogue() -> Dialogue {
        Dialogue {
            id: "test_dialogue".to_string(),
            start: "n0".to_string(),
            nodes: vec![
                Node {
                    id: "n0".to_string(),
                    line: Some(Line {
                        speaker: "Guard".to_string(),
                        text: "Halt! State your business.".to_string(),
                        set_vars: vec![],
                    }),
                    choices: vec![
                        Choice {
                            text: "I'm just passing through".to_string(),
                            go_to: "n1".to_string(),
                            require: vec![],
                        },
                        Choice {
                            text: "I have a quest".to_string(),
                            go_to: "n2".to_string(),
                            require: vec![Cond::Has {
                                key: "quest_token".to_string(),
                            }],
                        },
                    ],
                    end: false,
                },
                Node {
                    id: "n1".to_string(),
                    line: Some(Line {
                        speaker: "Guard".to_string(),
                        text: "Move along then.".to_string(),
                        set_vars: vec![("mood".to_string(), "neutral".to_string())],
                    }),
                    choices: vec![],
                    end: true,
                },
                Node {
                    id: "n2".to_string(),
                    line: Some(Line {
                        speaker: "Guard".to_string(),
                        text: "Ah, I see you have the token!".to_string(),
                        set_vars: vec![("mood".to_string(), "happy".to_string())],
                    }),
                    choices: vec![],
                    end: true,
                },
            ],
        }
    }

    #[test]
    fn test_dialogue_state_new() {
        let dialogue = create_simple_dialogue();
        let state = DialogueState::new(&dialogue);

        assert_eq!(state.idx, 0); // Should start at n0
        assert_eq!(state.map.len(), 3); // 3 nodes mapped
        assert!(state.vars.is_empty());
    }

    #[test]
    fn test_current_node() {
        let dialogue = create_simple_dialogue();
        let state = DialogueState::new(&dialogue);

        let node = state.current(&dialogue);
        assert_eq!(node.id, "n0");
        assert!(node.line.is_some());
        assert_eq!(node.line.as_ref().test_unwrap().speaker, "Guard");
    }

    #[test]
    fn test_choose_valid_choice_no_conditions() {
        let dialogue = create_simple_dialogue();
        let mut state = DialogueState::new(&dialogue);

        // Choose first option (no requirements)
        let success = state.choose(&dialogue, 0);
        assert!(success);
        assert_eq!(state.idx, 1); // Should move to n1

        // Verify set_vars applied
        assert_eq!(state.vars.get("mood"), Some(&"neutral".to_string()));
    }

    #[test]
    fn test_choose_fails_without_required_condition() {
        let dialogue = create_simple_dialogue();
        let mut state = DialogueState::new(&dialogue);

        // Choose second option (requires quest_token, which we don't have)
        let success = state.choose(&dialogue, 1);
        assert!(!success);
        assert_eq!(state.idx, 0); // Should stay at n0
        assert!(state.vars.is_empty());
    }

    #[test]
    fn test_choose_succeeds_with_required_condition() {
        let dialogue = create_simple_dialogue();
        let mut state = DialogueState::new(&dialogue);

        // Manually set the required variable
        state
            .vars
            .insert("quest_token".to_string(), "true".to_string());

        // Now choose second option
        let success = state.choose(&dialogue, 1);
        assert!(success);
        assert_eq!(state.idx, 2); // Should move to n2
        assert_eq!(state.vars.get("mood"), Some(&"happy".to_string()));
    }

    #[test]
    fn test_choose_invalid_index() {
        let dialogue = create_simple_dialogue();
        let mut state = DialogueState::new(&dialogue);

        // Choose invalid index
        let success = state.choose(&dialogue, 99);
        assert!(!success);
        assert_eq!(state.idx, 0); // Should stay at n0
    }

    #[test]
    fn test_eval_cond_eq_true() {
        let mut vars = HashMap::new();
        vars.insert("mood".to_string(), "happy".to_string());

        let cond = Cond::Eq {
            key: "mood".to_string(),
            val: "happy".to_string(),
        };
        assert!(eval(&cond, &vars));
    }

    #[test]
    fn test_eval_cond_eq_false() {
        let mut vars = HashMap::new();
        vars.insert("mood".to_string(), "sad".to_string());

        let cond = Cond::Eq {
            key: "mood".to_string(),
            val: "happy".to_string(),
        };
        assert!(!eval(&cond, &vars));
    }

    #[test]
    fn test_eval_cond_ne_true() {
        let mut vars = HashMap::new();
        vars.insert("mood".to_string(), "sad".to_string());

        let cond = Cond::Ne {
            key: "mood".to_string(),
            val: "happy".to_string(),
        };
        assert!(eval(&cond, &vars));
    }

    #[test]
    fn test_eval_cond_ne_false() {
        let mut vars = HashMap::new();
        vars.insert("mood".to_string(), "happy".to_string());

        let cond = Cond::Ne {
            key: "mood".to_string(),
            val: "happy".to_string(),
        };
        assert!(!eval(&cond, &vars));
    }

    #[test]
    fn test_eval_cond_has_true() {
        let mut vars = HashMap::new();
        vars.insert("quest_token".to_string(), "value".to_string());

        let cond = Cond::Has {
            key: "quest_token".to_string(),
        };
        assert!(eval(&cond, &vars));
    }

    #[test]
    fn test_eval_cond_has_false() {
        let vars = HashMap::new();

        let cond = Cond::Has {
            key: "quest_token".to_string(),
        };
        assert!(!eval(&cond, &vars));
    }

    #[test]
    fn test_compile_banter_simple() {
        let src = "[Guard] Hello there!\n[Player] Hi!";
        let dialogue = compile_banter_to_nodes("banter1", src);

        assert_eq!(dialogue.id, "banter1");
        assert_eq!(dialogue.start, "n0");
        assert_eq!(dialogue.nodes.len(), 2);
        assert_eq!(
            dialogue.nodes[0].line.as_ref().test_unwrap().speaker,
            "Guard"
        );
        assert_eq!(
            dialogue.nodes[1].line.as_ref().test_unwrap().speaker,
            "Player"
        );
    }

    #[test]
    fn test_compile_banter_with_set_var() {
        let src = "[Guard] Welcome!\n-> mood=happy";
        let dialogue = compile_banter_to_nodes("banter2", src);

        assert_eq!(dialogue.nodes.len(), 1);
        let line = dialogue.nodes[0].line.as_ref().test_unwrap();
        assert_eq!(line.set_vars.len(), 1);
        assert_eq!(line.set_vars[0], ("mood".to_string(), "happy".to_string()));
    }

    #[test]
    fn test_compile_banter_with_condition() {
        let src = "[Guard] How are you?\n? mood == happy : goto n1";
        let dialogue = compile_banter_to_nodes("banter3", src);

        assert_eq!(dialogue.nodes.len(), 1);
        assert_eq!(dialogue.nodes[0].choices.len(), 1);
        let choice = &dialogue.nodes[0].choices[0];
        assert_eq!(choice.go_to, "n1");
        assert_eq!(choice.require.len(), 1);
    }

    #[test]
    fn test_compile_banter_marks_last_node_as_end() {
        let src = "[Guard] Goodbye!";
        let dialogue = compile_banter_to_nodes("banter4", src);

        assert!(dialogue.nodes.last().test_unwrap().end);
    }

    // ===== Mutation-resistant compile_banter tests =====
    // Catches: `+ -> -/*` in text extraction (end + 1), `+= -> *=` in node ID counter

    #[test]
    fn test_compile_banter_exact_text_content() {
        // Tests that `rest[end + 1..]` correctly skips the `]` character
        // If mutated to `end - 1` or `end * 1`, text would include `]` or extra chars
        let src = "[Guard] Hello there!";
        let dialogue = compile_banter_to_nodes("txt_test", src);
        let line = dialogue.nodes[0].line.as_ref().test_unwrap();
        assert_eq!(
            line.text, "Hello there!",
            "Text must not include ] bracket; got: {}",
            line.text
        );
        assert!(!line.text.contains(']'), "Text must not contain ]");
    }

    #[test]
    fn test_compile_banter_sequential_node_ids() {
        // Tests that `i += 1` increments correctly (catches *= mutant where i stays 0)
        let src = "[Guard] Line one\n[Player] Line two\n[NPC] Line three";
        let dialogue = compile_banter_to_nodes("seq_test", src);
        assert_eq!(dialogue.nodes.len(), 3);
        assert_eq!(dialogue.nodes[0].id, "n0");
        assert_eq!(dialogue.nodes[1].id, "n1", "Second node must be n1, not n0");
        assert_eq!(dialogue.nodes[2].id, "n2", "Third node must be n2, not n0");
    }

    #[test]
    fn test_compile_banter_text_with_spaces_after_bracket() {
        // Ensures trimming works correctly after skipping `]`
        let src = "[A]   spaced text  ";
        let dialogue = compile_banter_to_nodes("space_test", src);
        let line = dialogue.nodes[0].line.as_ref().test_unwrap();
        assert_eq!(line.speaker, "A");
        assert_eq!(line.text, "spaced text");
    }
}
