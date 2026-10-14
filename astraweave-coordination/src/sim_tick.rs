//! Synchronous coordinator for the simulation tick.
//!
//! [`crate::AgentCoordinator`] routes through tokio tasks, `Uuid`, and the
//! wall clock. Those values must not enter the tick. This type applies the
//! same [`RoutingAction`] set in sorted agent and rule order. `Transform`
//! prefixes the message body with the action string.

use std::collections::BTreeSet;

use crate::{RoutingAction, RoutingRule};

/// One recorded message waiting for the tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimMessage {
    pub from: String,
    pub to: String,
    pub body: String,
}

/// A message the tick actually delivered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimDelivery {
    pub to: String,
    pub body: String,
}

/// Deterministic, single-threaded view of agent routing.
#[derive(Debug, Default)]
pub struct SimCoordinator {
    agents: BTreeSet<String>,
    rules: Vec<RoutingRule>,
    last_tick: u64,
}

impl SimCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_agent(&mut self, id: impl Into<String>) {
        self.agents.insert(id.into());
    }

    pub fn add_rule(&mut self, rule: RoutingRule) {
        self.rules.push(rule);
    }

    pub fn last_tick(&self) -> u64 {
        self.last_tick
    }

    pub fn agents(&self) -> impl Iterator<Item = &str> {
        self.agents.iter().map(String::as_str)
    }

    /// Route `messages` for this tick.
    ///
    /// Input order is ignored: messages are sorted by `(from, to, body)`.
    /// The first matching rule, ordered by descending priority then rule id,
    /// wins. A missing rule allows delivery.
    pub fn tick(&mut self, tick: u64, messages: &[SimMessage]) -> Vec<SimDelivery> {
        self.last_tick = tick;
        let mut ordered = messages.to_vec();
        ordered.sort_by(|a, b| (&a.from, &a.to, &a.body).cmp(&(&b.from, &b.to, &b.body)));
        let mut rules = self.rules.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));

        let mut deliveries = Vec::new();
        for message in ordered {
            let action = rules
                .iter()
                .find(|rule| rule_matches(rule, &message))
                .map(|rule| rule.action.clone())
                .unwrap_or(RoutingAction::Allow);
            match action {
                RoutingAction::Allow => self.deliver(&mut deliveries, &message.to, &message.body),
                RoutingAction::Block => {}
                RoutingAction::Redirect(target) => {
                    self.deliver(&mut deliveries, &target, &message.body);
                }
                RoutingAction::Broadcast(mut targets) => {
                    targets.sort();
                    for target in targets {
                        self.deliver(&mut deliveries, &target, &message.body);
                    }
                }
                RoutingAction::Transform(prefix) => {
                    let body = format!("{prefix}{}", message.body);
                    self.deliver(&mut deliveries, &message.to, &body);
                }
            }
        }
        deliveries
    }

    fn deliver(&self, out: &mut Vec<SimDelivery>, to: &str, body: &str) {
        if !self.agents.contains(to) {
            return;
        }
        out.push(SimDelivery {
            to: to.to_string(),
            body: body.to_string(),
        });
    }
}

fn rule_matches(rule: &RoutingRule, message: &SimMessage) -> bool {
    if let Some(pattern) = &rule.from_pattern {
        if pattern != &message.from {
            return false;
        }
    }
    if let Some(pattern) = &rule.to_pattern {
        if pattern != &message.to {
            return false;
        }
    }
    if let Some(kind) = &rule.message_type {
        if !message.body.starts_with(kind) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, from: &str, action: RoutingAction, priority: i32) -> RoutingRule {
        RoutingRule {
            id: id.to_string(),
            from_pattern: Some(from.to_string()),
            to_pattern: None,
            message_type: None,
            action,
            priority,
        }
    }

    #[test]
    fn transform_prefixes_and_order_is_sorted() {
        let mut sim = SimCoordinator::new();
        sim.register_agent("guard");
        sim.register_agent("merchant");
        sim.add_rule(rule(
            "hear",
            "merchant",
            RoutingAction::Transform("heard:".into()),
            1,
        ));
        let late = SimMessage {
            from: "merchant".into(),
            to: "guard".into(),
            body: "b".into(),
        };
        let early = SimMessage {
            from: "merchant".into(),
            to: "guard".into(),
            body: "a".into(),
        };
        let first = sim.tick(3, &[late.clone(), early.clone()]);
        let second = sim.tick(3, &[early, late]);
        assert_eq!(first, second);
        assert_eq!(first[0].body, "heard:a");
        assert_eq!(first[1].body, "heard:b");
        assert_eq!(sim.last_tick(), 3);
    }

    #[test]
    fn block_drops_and_unknown_recipient_is_not_delivered() {
        let mut sim = SimCoordinator::new();
        sim.register_agent("guard");
        sim.add_rule(rule("stop", "merchant", RoutingAction::Block, 5));
        let dropped = sim.tick(
            1,
            &[SimMessage {
                from: "merchant".into(),
                to: "guard".into(),
                body: "secret".into(),
            }],
        );
        assert!(dropped.is_empty());
    }
}
