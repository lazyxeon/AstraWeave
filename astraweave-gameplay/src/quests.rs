use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TaskKind {
    Gather { kind: String, count: u32 },
    Visit { marker: String },
    Defeat { enemy: String, count: u32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub kind: TaskKind,
    pub done: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub tasks: Vec<Task>,
    pub reward_text: String,
    #[serde(default)]
    pub completed: bool,
}

#[derive(Default, Debug)]
pub struct QuestLog {
    pub quests: HashMap<String, Quest>,
}

impl QuestLog {
    pub fn add(&mut self, q: Quest) {
        self.quests.insert(q.id.clone(), q);
    }
    pub fn is_done(&self, id: &str) -> bool {
        self.quests.get(id).map(|q| q.completed).unwrap_or(false)
    }

    pub fn progress_gather(&mut self, id: &str, kind: &str, n: u32) {
        if let Some(q) = self.quests.get_mut(id) {
            for t in q.tasks.iter_mut() {
                if let TaskKind::Gather { kind: tk, count } = &mut t.kind {
                    if tk == kind && !t.done {
                        if *count > n {
                            *count -= n;
                        } else {
                            *count = 0;
                            t.done = true;
                        }
                    }
                }
            }
            if q.tasks.iter().all(|t| t.done) {
                q.completed = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub use crate::test_support::{test_panic, TestValue};

    #[test]
    fn test_quest_log_add_quest() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: "q1".to_string(),
            title: "First Quest".to_string(),
            tasks: vec![],
            reward_text: "100 gold".to_string(),
            completed: false,
        };

        log.add(quest);
        assert_eq!(log.quests.len(), 1);
        assert!(log.quests.contains_key("q1"));
    }

    #[test]
    fn test_is_done_returns_false_for_incomplete() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: "q1".to_string(),
            title: "Quest".to_string(),
            tasks: vec![],
            reward_text: "reward".to_string(),
            completed: false,
        };
        log.add(quest);

        assert!(!log.is_done("q1"));
    }

    #[test]
    fn test_is_done_returns_true_for_completed() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: "q1".to_string(),
            title: "Quest".to_string(),
            tasks: vec![],
            reward_text: "reward".to_string(),
            completed: true,
        };
        log.add(quest);

        assert!(log.is_done("q1"));
    }

    #[test]
    fn test_is_done_returns_false_for_nonexistent() {
        let log = QuestLog::default();
        assert!(!log.is_done("nonexistent"));
    }

    #[test]
    fn test_progress_gather_partial_completion() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: "gather_wood".to_string(),
            title: "Gather Wood".to_string(),
            tasks: vec![Task {
                id: "t1".to_string(),
                kind: TaskKind::Gather {
                    kind: "wood".to_string(),
                    count: 10,
                },
                done: false,
            }],
            reward_text: "50 gold".to_string(),
            completed: false,
        };
        log.add(quest);

        // Gather 3 wood, should reduce count from 10 to 7
        log.progress_gather("gather_wood", "wood", 3);

        let q = log.quests.get("gather_wood").test_unwrap();
        if let TaskKind::Gather { count, .. } = &q.tasks[0].kind {
            assert_eq!(*count, 7);
            assert!(!q.tasks[0].done);
            assert!(!q.completed);
        } else {
            test_panic(format_args!("Expected Gather task"));
        }
    }

    #[test]
    fn test_progress_gather_complete_task() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: "gather_fiber".to_string(),
            title: "Gather Fiber".to_string(),
            tasks: vec![Task {
                id: "t1".to_string(),
                kind: TaskKind::Gather {
                    kind: "fiber".to_string(),
                    count: 5,
                },
                done: false,
            }],
            reward_text: "reward".to_string(),
            completed: false,
        };
        log.add(quest);

        // Gather 10 fiber (more than needed)
        log.progress_gather("gather_fiber", "fiber", 10);

        let q = log.quests.get("gather_fiber").test_unwrap();
        if let TaskKind::Gather { count, .. } = &q.tasks[0].kind {
            assert_eq!(*count, 0);
            assert!(q.tasks[0].done);
            assert!(q.completed); // Quest should auto-complete
        } else {
            test_panic(format_args!("Expected Gather task"));
        }
    }

    #[test]
    fn test_progress_gather_multiple_tasks_completes_quest() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: "multi_gather".to_string(),
            title: "Multi Gather".to_string(),
            tasks: vec![
                Task {
                    id: "t1".to_string(),
                    kind: TaskKind::Gather {
                        kind: "wood".to_string(),
                        count: 5,
                    },
                    done: false,
                },
                Task {
                    id: "t2".to_string(),
                    kind: TaskKind::Gather {
                        kind: "stone".to_string(),
                        count: 3,
                    },
                    done: false,
                },
            ],
            reward_text: "big reward".to_string(),
            completed: false,
        };
        log.add(quest);

        // Complete wood task
        log.progress_gather("multi_gather", "wood", 5);
        assert!(!log.quests.get("multi_gather").test_unwrap().completed);

        // Complete stone task
        log.progress_gather("multi_gather", "stone", 3);
        assert!(log.quests.get("multi_gather").test_unwrap().completed);
    }

    #[test]
    fn test_progress_gather_ignores_completed_tasks() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: "q1".to_string(),
            title: "Quest".to_string(),
            tasks: vec![Task {
                id: "t1".to_string(),
                kind: TaskKind::Gather {
                    kind: "wood".to_string(),
                    count: 0,
                },
                done: true,
            }],
            reward_text: "reward".to_string(),
            completed: true,
        };
        log.add(quest);

        // Try to progress already-done task
        log.progress_gather("q1", "wood", 5);

        // Count should stay at 0 (not go negative or change)
        let q = log.quests.get("q1").test_unwrap();
        if let TaskKind::Gather { count, .. } = &q.tasks[0].kind {
            assert_eq!(*count, 0);
        }
    }

    #[test]
    fn test_progress_gather_wrong_kind_ignored() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: "q1".to_string(),
            title: "Quest".to_string(),
            tasks: vec![Task {
                id: "t1".to_string(),
                kind: TaskKind::Gather {
                    kind: "wood".to_string(),
                    count: 10,
                },
                done: false,
            }],
            reward_text: "reward".to_string(),
            completed: false,
        };
        log.add(quest);

        // Try to progress with wrong resource kind
        log.progress_gather("q1", "stone", 5);

        let q = log.quests.get("q1").test_unwrap();
        if let TaskKind::Gather { count, .. } = &q.tasks[0].kind {
            assert_eq!(*count, 10); // Should remain unchanged
            assert!(!q.tasks[0].done);
        }
    }
}
