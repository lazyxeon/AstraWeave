//! Deterministic events resource for ECS systems
use crate::IVec2;
use astraweave_ecs as ecs;
use std::collections::VecDeque;

pub struct Events<T> {
    q: VecDeque<T>,
    /// Maximum number of events to retain. Oldest events are dropped when exceeded.
    /// None means unbounded (default).
    max_size: Option<usize>,
}

impl<T> Events<T> {
    /// Create an event queue with a maximum size limit.
    /// When the limit is reached, the oldest event is dropped before inserting new ones.
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            q: VecDeque::with_capacity(max_size.min(1024)),
            max_size: Some(max_size),
        }
    }

    pub fn writer(&mut self) -> EventWriter<'_, T> {
        EventWriter {
            q: &mut self.q,
            max_size: self.max_size,
        }
    }
    pub fn reader(&mut self) -> EventReader<'_, T> {
        EventReader { q: &mut self.q }
    }
    pub fn clear(&mut self) {
        self.q.clear();
    }
    pub fn len(&self) -> usize {
        self.q.len()
    }
    pub fn is_empty(&self) -> bool {
        self.q.is_empty()
    }
}

impl<T> Default for Events<T> {
    fn default() -> Self {
        Self {
            q: VecDeque::new(),
            max_size: None,
        }
    }
}

pub struct EventWriter<'a, T> {
    q: &'a mut VecDeque<T>,
    max_size: Option<usize>,
}
impl<'a, T> EventWriter<'a, T> {
    pub fn send(&mut self, ev: T) {
        // Enforce max_size: drop oldest event if at capacity
        if let Some(max) = self.max_size {
            if self.q.len() >= max {
                self.q.pop_front();
            }
        }
        self.q.push_back(ev);
    }
}

pub struct EventReader<'a, T> {
    q: &'a mut VecDeque<T>,
}
impl<'a, T> EventReader<'a, T> {
    pub fn drain(&mut self) -> std::collections::vec_deque::Drain<'_, T> {
        self.q.drain(..)
    }
}

// Concrete event types used by core + AI plugin
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovedEvent {
    pub entity: ecs::Entity,
    pub from: IVec2,
    pub to: IVec2,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiPlannedEvent {
    pub entity: ecs::Entity,
    pub target: IVec2,
}

#[derive(Clone, Debug)]
pub struct AiPlanningFailedEvent {
    pub entity: ecs::Entity,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct ToolValidationFailedEvent {
    pub entity: ecs::Entity,
    pub tool_verb: String,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct HealthChangedEvent {
    pub entity: ecs::Entity,
    pub old_hp: i32,
    pub new_hp: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_default() {
        let events: Events<i32> = Events::default();
        assert!(events.is_empty());
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_events_send_and_len() {
        let mut events: Events<i32> = Events::default();
        {
            let mut writer = events.writer();
            writer.send(42);
            writer.send(100);
        }
        assert_eq!(events.len(), 2);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_events_drain() {
        let mut events: Events<i32> = Events::default();
        {
            let mut writer = events.writer();
            writer.send(1);
            writer.send(2);
            writer.send(3);
        }

        let drained: Vec<_> = {
            let mut reader = events.reader();
            reader.drain().collect()
        };

        assert_eq!(drained, vec![1, 2, 3]);
        assert!(events.is_empty());
    }

    #[test]
    fn test_events_clear() {
        let mut events: Events<i32> = Events::default();
        {
            let mut writer = events.writer();
            writer.send(1);
            writer.send(2);
        }
        events.clear();

        assert!(events.is_empty());
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_moved_event_creation() {
        let entity = unsafe { ecs::Entity::from_raw(1u64) };
        let event = MovedEvent {
            entity,
            from: IVec2 { x: 0, y: 0 },
            to: IVec2 { x: 5, y: 5 },
        };

        assert_eq!(event.from, IVec2 { x: 0, y: 0 });
        assert_eq!(event.to, IVec2 { x: 5, y: 5 });
    }

    #[test]
    fn test_moved_event_equality() {
        let entity = unsafe { ecs::Entity::from_raw(1u64) };
        let event1 = MovedEvent {
            entity,
            from: IVec2 { x: 0, y: 0 },
            to: IVec2 { x: 5, y: 5 },
        };
        let event2 = MovedEvent {
            entity,
            from: IVec2 { x: 0, y: 0 },
            to: IVec2 { x: 5, y: 5 },
        };

        assert_eq!(event1, event2);
    }

    #[test]
    fn test_ai_planned_event() {
        let entity = unsafe { ecs::Entity::from_raw(1u64) };
        let event = AiPlannedEvent {
            entity,
            target: IVec2 { x: 10, y: 20 },
        };

        assert_eq!(event.target, IVec2 { x: 10, y: 20 });
    }

    #[test]
    fn test_ai_planning_failed_event() {
        let entity = unsafe { ecs::Entity::from_raw(1u64) };
        let event = AiPlanningFailedEvent {
            entity,
            reason: "no path".into(),
        };

        assert_eq!(event.reason, "no path");
    }

    #[test]
    fn test_tool_validation_failed_event() {
        let entity = unsafe { ecs::Entity::from_raw(1u64) };
        let event = ToolValidationFailedEvent {
            entity,
            tool_verb: "MoveTo".into(),
            reason: "blocked".into(),
        };

        assert_eq!(event.tool_verb, "MoveTo");
        assert_eq!(event.reason, "blocked");
    }

    #[test]
    fn test_health_changed_event() {
        let entity = unsafe { ecs::Entity::from_raw(1u64) };
        let event = HealthChangedEvent {
            entity,
            old_hp: 100,
            new_hp: 75,
        };

        assert_eq!(event.old_hp, 100);
        assert_eq!(event.new_hp, 75);
    }
}
