#![forbid(unsafe_code)]
//! Behavior Graphs (BT/HTN) and GOAP for AstraWeave AI

#[cfg(test)]
mod mutation_tests;
#[cfg(test)]
mod test_support;

pub mod ecs;
pub mod goap;
pub mod goap_cache; // Week 3 Action 9: GOAP plan caching with LRU eviction
pub mod interner;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Node types for behavior trees and HTN
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BehaviorNode {
    Sequence(Vec<BehaviorNode>),
    Selector(Vec<BehaviorNode>),
    Action(String),
    Condition(String),
    Decorator(DecoratorType, Box<BehaviorNode>),
    Parallel(Vec<BehaviorNode>, usize), // children, success threshold
}

impl BehaviorNode {
    /// Creates a new action node.
    #[must_use]
    pub fn action(name: impl Into<String>) -> Self {
        Self::Action(name.into())
    }

    /// Creates a new condition node.
    #[must_use]
    pub fn condition(name: impl Into<String>) -> Self {
        Self::Condition(name.into())
    }

    /// Creates a new sequence node.
    #[must_use]
    pub fn sequence(children: Vec<BehaviorNode>) -> Self {
        Self::Sequence(children)
    }

    /// Creates a new selector node.
    #[must_use]
    pub fn selector(children: Vec<BehaviorNode>) -> Self {
        Self::Selector(children)
    }

    /// Creates a new parallel node.
    #[must_use]
    pub fn parallel(children: Vec<BehaviorNode>, threshold: usize) -> Self {
        Self::Parallel(children, threshold)
    }

    /// Creates a new decorator node.
    #[must_use]
    pub fn decorator(decorator_type: DecoratorType, child: BehaviorNode) -> Self {
        Self::Decorator(decorator_type, Box::new(child))
    }

    /// Returns true if this is an action node.
    #[must_use]
    pub fn is_action(&self) -> bool {
        matches!(self, Self::Action(_))
    }

    /// Returns true if this is a condition node.
    #[must_use]
    pub fn is_condition(&self) -> bool {
        matches!(self, Self::Condition(_))
    }

    /// Returns true if this is a sequence node.
    #[must_use]
    pub fn is_sequence(&self) -> bool {
        matches!(self, Self::Sequence(_))
    }

    /// Returns true if this is a selector node.
    #[must_use]
    pub fn is_selector(&self) -> bool {
        matches!(self, Self::Selector(_))
    }

    /// Returns true if this is a parallel node.
    #[must_use]
    pub fn is_parallel(&self) -> bool {
        matches!(self, Self::Parallel(_, _))
    }

    /// Returns true if this is a decorator node.
    #[must_use]
    pub fn is_decorator(&self) -> bool {
        matches!(self, Self::Decorator(_, _))
    }

    /// Returns true if this is a leaf node (action or condition).
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Action(_) | Self::Condition(_))
    }

    /// Returns true if this is a composite node (sequence, selector, or parallel).
    #[must_use]
    pub fn is_composite(&self) -> bool {
        matches!(
            self,
            Self::Sequence(_) | Self::Selector(_) | Self::Parallel(_, _)
        )
    }

    /// Returns the number of direct children.
    #[must_use]
    pub fn child_count(&self) -> usize {
        match self {
            Self::Sequence(children) | Self::Selector(children) | Self::Parallel(children, _) => {
                children.len()
            }
            Self::Decorator(_, _) => 1,
            Self::Action(_) | Self::Condition(_) => 0,
        }
    }

    /// Returns the name if this is an action or condition node.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Action(name) | Self::Condition(name) => Some(name),
            _ => None,
        }
    }

    /// Returns the node type as a string.
    #[must_use]
    pub fn node_type(&self) -> &'static str {
        match self {
            Self::Sequence(_) => "Sequence",
            Self::Selector(_) => "Selector",
            Self::Action(_) => "Action",
            Self::Condition(_) => "Condition",
            Self::Decorator(_, _) => "Decorator",
            Self::Parallel(_, _) => "Parallel",
        }
    }

    /// Returns the total node count including all descendants.
    #[must_use]
    pub fn total_node_count(&self) -> usize {
        match self {
            Self::Sequence(children) | Self::Selector(children) | Self::Parallel(children, _) => {
                1 + children.iter().map(|c| c.total_node_count()).sum::<usize>()
            }
            Self::Decorator(_, child) => 1 + child.total_node_count(),
            Self::Action(_) | Self::Condition(_) => 1,
        }
    }

    /// Returns the maximum depth of the tree.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        match self {
            Self::Sequence(children) | Self::Selector(children) | Self::Parallel(children, _) => {
                1 + children.iter().map(|c| c.max_depth()).max().unwrap_or(0)
            }
            Self::Decorator(_, child) => 1 + child.max_depth(),
            Self::Action(_) | Self::Condition(_) => 1,
        }
    }

    /// Returns a brief summary of the node.
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            Self::Action(name) => format!("Action({})", name),
            Self::Condition(name) => format!("Condition({})", name),
            Self::Sequence(children) => format!("Sequence[{}]", children.len()),
            Self::Selector(children) => format!("Selector[{}]", children.len()),
            Self::Parallel(children, threshold) => {
                format!("Parallel[{}/{}]", threshold, children.len())
            }
            Self::Decorator(dec_type, _) => format!("Decorator({})", dec_type),
        }
    }

    pub fn tick(&self, context: &BehaviorContext) -> BehaviorStatus {
        match self {
            BehaviorNode::Action(name) => context.evaluate_action(name),
            BehaviorNode::Condition(name) => context.evaluate_condition(name),
            BehaviorNode::Sequence(children) => {
                for child in children {
                    match child.tick(context) {
                        BehaviorStatus::Running => return BehaviorStatus::Running,
                        BehaviorStatus::Failure => return BehaviorStatus::Failure,
                        BehaviorStatus::Success => continue,
                    }
                }
                BehaviorStatus::Success
            }
            BehaviorNode::Selector(children) => {
                for child in children {
                    match child.tick(context) {
                        BehaviorStatus::Running => return BehaviorStatus::Running,
                        BehaviorStatus::Success => return BehaviorStatus::Success,
                        BehaviorStatus::Failure => continue,
                    }
                }
                BehaviorStatus::Failure
            }
            BehaviorNode::Decorator(decorator, child) => match decorator {
                DecoratorType::Inverter => match child.tick(context) {
                    BehaviorStatus::Success => BehaviorStatus::Failure,
                    BehaviorStatus::Failure => BehaviorStatus::Success,
                    r => r,
                },
                DecoratorType::Succeeder => {
                    child.tick(context);
                    BehaviorStatus::Success
                }
                DecoratorType::Failer => {
                    child.tick(context);
                    BehaviorStatus::Failure
                }
                DecoratorType::Repeat(max) => {
                    for _ in 0..*max {
                        match child.tick(context) {
                            BehaviorStatus::Running => return BehaviorStatus::Running,
                            BehaviorStatus::Success => continue,
                            BehaviorStatus::Failure => return BehaviorStatus::Failure,
                        }
                    }
                    BehaviorStatus::Success
                }
                DecoratorType::Retry(max) => {
                    for _ in 0..*max {
                        match child.tick(context) {
                            BehaviorStatus::Running => return BehaviorStatus::Running,
                            BehaviorStatus::Success => return BehaviorStatus::Success,
                            BehaviorStatus::Failure => continue,
                        }
                    }
                    BehaviorStatus::Failure
                }
            },
            BehaviorNode::Parallel(children, threshold) => {
                // Ensure the threshold is within sensible bounds
                if *threshold == 0 {
                    return BehaviorStatus::Success;
                }
                if *threshold > children.len() {
                    return BehaviorStatus::Failure;
                }

                let mut success_count = 0;
                let mut running_count = 0;
                for child in children {
                    match child.tick(context) {
                        BehaviorStatus::Success => success_count += 1,
                        BehaviorStatus::Running => running_count += 1,
                        BehaviorStatus::Failure => {} // do nothing
                    }
                }
                if success_count >= *threshold {
                    BehaviorStatus::Success
                } else if running_count > 0 {
                    BehaviorStatus::Running
                } else {
                    BehaviorStatus::Failure
                }
            }
        }
    }
}

impl fmt::Display for BehaviorNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecoratorType {
    Inverter,
    Succeeder,
    Failer,
    Repeat(u32), // max repeats
    Retry(u32),  // max retries
}

impl DecoratorType {
    /// Returns the decorator name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Inverter => "Inverter",
            Self::Succeeder => "Succeeder",
            Self::Failer => "Failer",
            Self::Repeat(_) => "Repeat",
            Self::Retry(_) => "Retry",
        }
    }

    /// Returns true if this is an inverter.
    #[must_use]
    pub fn is_inverter(&self) -> bool {
        matches!(self, Self::Inverter)
    }

    /// Returns true if this forces a specific result.
    #[must_use]
    pub fn forces_result(&self) -> bool {
        matches!(self, Self::Succeeder | Self::Failer)
    }

    /// Returns true if this is a looping decorator.
    #[must_use]
    pub fn is_looping(&self) -> bool {
        matches!(self, Self::Repeat(_) | Self::Retry(_))
    }

    /// Returns the iteration count if this is a looping decorator.
    #[must_use]
    pub fn iteration_count(&self) -> Option<u32> {
        match self {
            Self::Repeat(n) | Self::Retry(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns all decorator types (with default counts for looping decorators).
    #[must_use]
    pub fn all() -> Vec<DecoratorType> {
        vec![
            Self::Inverter,
            Self::Succeeder,
            Self::Failer,
            Self::Repeat(1),
            Self::Retry(1),
        ]
    }
}

impl fmt::Display for DecoratorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inverter => write!(f, "Inverter"),
            Self::Succeeder => write!(f, "Succeeder"),
            Self::Failer => write!(f, "Failer"),
            Self::Repeat(n) => write!(f, "Repeat({})", n),
            Self::Retry(n) => write!(f, "Retry({})", n),
        }
    }
}

/// Behavior graph structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorGraph {
    pub root: BehaviorNode,
}

impl BehaviorGraph {
    pub fn new(root: BehaviorNode) -> Self {
        Self { root }
    }

    pub fn tick(&self, context: &BehaviorContext) -> BehaviorStatus {
        self.root.tick(context)
    }

    pub fn current_node_name(&self) -> Option<String> {
        // For now, return a placeholder. In a more complex implementation,
        // this would track the currently executing node.
        Some("root".to_string())
    }

    /// Returns the total number of nodes in the graph.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.root.total_node_count()
    }

    /// Returns the maximum depth of the graph.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.root.max_depth()
    }

    /// Returns the root node type.
    #[must_use]
    pub fn root_type(&self) -> &'static str {
        self.root.node_type()
    }

    /// Returns true if the root is a leaf node.
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        self.root.is_leaf()
    }

    /// Returns true if the root is a composite node.
    #[must_use]
    pub fn is_composite(&self) -> bool {
        self.root.is_composite()
    }

    /// Returns a brief summary of the graph.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "BehaviorGraph: root={}, nodes={}, depth={}",
            self.root.node_type(),
            self.node_count(),
            self.max_depth()
        )
    }
}

impl fmt::Display for BehaviorGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== BehaviorContext Tests =====

    #[test]
    fn test_behavior_context_default() {
        let ctx = BehaviorContext::default();
        assert_eq!(ctx.actions.len(), 0);
        assert_eq!(ctx.conditions.len(), 0);
    }

    #[test]
    fn test_register_action() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("test_action", || BehaviorStatus::Success);
        assert_eq!(ctx.actions.len(), 1);
        assert!(ctx.actions.contains_key("test_action"));
    }

    #[test]
    fn test_register_condition() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("test_condition", || true);
        assert_eq!(ctx.conditions.len(), 1);
        assert!(ctx.conditions.contains_key("test_condition"));
    }

    #[test]
    fn test_evaluate_action_success() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("succeed", || BehaviorStatus::Success);
        let node = BehaviorNode::Action("succeed".to_string());
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_evaluate_action_failure() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("fail", || BehaviorStatus::Failure);
        let node = BehaviorNode::Action("fail".to_string());
        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    #[test]
    fn test_evaluate_action_running() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("running", || BehaviorStatus::Running);
        let node = BehaviorNode::Action("running".to_string());
        assert_eq!(node.tick(&ctx), BehaviorStatus::Running);
    }

    #[test]
    fn test_evaluate_condition_true() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("is_true", || true);
        let node = BehaviorNode::Condition("is_true".to_string());
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_evaluate_condition_false() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("is_false", || false);
        let node = BehaviorNode::Condition("is_false".to_string());
        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    // ===== Sequence Node Tests =====

    #[test]
    fn test_sequence_all_success() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Success);
        ctx.register_action("a3", || BehaviorStatus::Success);

        let seq = BehaviorNode::Sequence(vec![
            BehaviorNode::Action("a1".to_string()),
            BehaviorNode::Action("a2".to_string()),
            BehaviorNode::Action("a3".to_string()),
        ]);

        assert_eq!(seq.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_sequence_early_failure() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Failure);
        ctx.register_action("a3", || BehaviorStatus::Success);

        let seq = BehaviorNode::Sequence(vec![
            BehaviorNode::Action("a1".to_string()),
            BehaviorNode::Action("a2".to_string()),
            BehaviorNode::Action("a3".to_string()),
        ]);

        assert_eq!(seq.tick(&ctx), BehaviorStatus::Failure);
    }

    #[test]
    fn test_sequence_running_short_circuit() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Running);
        ctx.register_action("a3", || BehaviorStatus::Success);

        let seq = BehaviorNode::Sequence(vec![
            BehaviorNode::Action("a1".to_string()),
            BehaviorNode::Action("a2".to_string()),
            BehaviorNode::Action("a3".to_string()),
        ]);

        assert_eq!(seq.tick(&ctx), BehaviorStatus::Running);
    }

    #[test]
    fn test_sequence_empty() {
        let ctx = BehaviorContext::new();
        let seq = BehaviorNode::Sequence(vec![]);
        assert_eq!(seq.tick(&ctx), BehaviorStatus::Success);
    }

    // ===== Selector Node Tests =====

    #[test]
    fn test_selector_all_failure() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Failure);
        ctx.register_action("a2", || BehaviorStatus::Failure);
        ctx.register_action("a3", || BehaviorStatus::Failure);

        let sel = BehaviorNode::Selector(vec![
            BehaviorNode::Action("a1".to_string()),
            BehaviorNode::Action("a2".to_string()),
            BehaviorNode::Action("a3".to_string()),
        ]);

        assert_eq!(sel.tick(&ctx), BehaviorStatus::Failure);
    }

    #[test]
    fn test_selector_early_success() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Failure);
        ctx.register_action("a2", || BehaviorStatus::Success);
        ctx.register_action("a3", || BehaviorStatus::Failure);

        let sel = BehaviorNode::Selector(vec![
            BehaviorNode::Action("a1".to_string()),
            BehaviorNode::Action("a2".to_string()),
            BehaviorNode::Action("a3".to_string()),
        ]);

        assert_eq!(sel.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_selector_running_short_circuit() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Failure);
        ctx.register_action("a2", || BehaviorStatus::Running);
        ctx.register_action("a3", || BehaviorStatus::Success);

        let sel = BehaviorNode::Selector(vec![
            BehaviorNode::Action("a1".to_string()),
            BehaviorNode::Action("a2".to_string()),
            BehaviorNode::Action("a3".to_string()),
        ]);

        assert_eq!(sel.tick(&ctx), BehaviorStatus::Running);
    }

    #[test]
    fn test_selector_empty() {
        let ctx = BehaviorContext::new();
        let sel = BehaviorNode::Selector(vec![]);
        assert_eq!(sel.tick(&ctx), BehaviorStatus::Failure);
    }

    // ===== Decorator Tests =====

    #[test]
    fn test_inverter_success_to_failure() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("succeed", || BehaviorStatus::Success);
        let node = BehaviorNode::Decorator(
            DecoratorType::Inverter,
            Box::new(BehaviorNode::Action("succeed".to_string())),
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    #[test]
    fn test_inverter_failure_to_success() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("fail", || BehaviorStatus::Failure);
        let node = BehaviorNode::Decorator(
            DecoratorType::Inverter,
            Box::new(BehaviorNode::Action("fail".to_string())),
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_inverter_running_unchanged() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("running", || BehaviorStatus::Running);
        let node = BehaviorNode::Decorator(
            DecoratorType::Inverter,
            Box::new(BehaviorNode::Action("running".to_string())),
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Running);
    }

    #[test]
    fn test_succeeder_forces_success() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("fail", || BehaviorStatus::Failure);
        let node = BehaviorNode::Decorator(
            DecoratorType::Succeeder,
            Box::new(BehaviorNode::Action("fail".to_string())),
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_failer_forces_failure() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("succeed", || BehaviorStatus::Success);
        let node = BehaviorNode::Decorator(
            DecoratorType::Failer,
            Box::new(BehaviorNode::Action("succeed".to_string())),
        );
        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    #[test]
    fn test_repeat_success() {
        use crate::test_support::TestValue;
        use std::sync::{Arc, Mutex};
        let count = Arc::new(Mutex::new(0));
        let count_clone = count.clone();
        let mut ctx = BehaviorContext::new();
        ctx.register_action("increment", move || {
            *count_clone.lock().test_unwrap() += 1;
            BehaviorStatus::Success
        });

        let node = BehaviorNode::Decorator(
            DecoratorType::Repeat(5),
            Box::new(BehaviorNode::Action("increment".to_string())),
        );

        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
        assert_eq!(*count.lock().test_unwrap(), 5);
    }

    #[test]
    fn test_repeat_early_failure() {
        use crate::test_support::TestValue;
        use std::sync::{Arc, Mutex};
        let count = Arc::new(Mutex::new(0));
        let count_clone = count.clone();
        let mut ctx = BehaviorContext::new();
        ctx.register_action("fail_at_3", move || {
            *count_clone.lock().test_unwrap() += 1;
            if *count_clone.lock().test_unwrap() < 3 {
                BehaviorStatus::Success
            } else {
                BehaviorStatus::Failure
            }
        });

        let node = BehaviorNode::Decorator(
            DecoratorType::Repeat(5),
            Box::new(BehaviorNode::Action("fail_at_3".to_string())),
        );

        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
        assert_eq!(*count.lock().test_unwrap(), 3);
    }

    #[test]
    fn test_retry_eventual_success() {
        use crate::test_support::TestValue;
        use std::sync::{Arc, Mutex};
        let attempt = Arc::new(Mutex::new(0));
        let attempt_clone = attempt.clone();
        let mut ctx = BehaviorContext::new();
        ctx.register_action("succeed_at_3", move || {
            *attempt_clone.lock().test_unwrap() += 1;
            if *attempt_clone.lock().test_unwrap() >= 3 {
                BehaviorStatus::Success
            } else {
                BehaviorStatus::Failure
            }
        });

        let node = BehaviorNode::Decorator(
            DecoratorType::Retry(5),
            Box::new(BehaviorNode::Action("succeed_at_3".to_string())),
        );

        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
        assert_eq!(*attempt.lock().test_unwrap(), 3);
    }

    #[test]
    fn test_retry_exhausted() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("always_fail", || BehaviorStatus::Failure);

        let node = BehaviorNode::Decorator(
            DecoratorType::Retry(3),
            Box::new(BehaviorNode::Action("always_fail".to_string())),
        );

        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    // ===== Parallel Node Tests =====

    #[test]
    fn test_parallel_threshold_met() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Success);
        ctx.register_action("a3", || BehaviorStatus::Failure);

        let node = BehaviorNode::Parallel(
            vec![
                BehaviorNode::Action("a1".to_string()),
                BehaviorNode::Action("a2".to_string()),
                BehaviorNode::Action("a3".to_string()),
            ],
            2, // threshold: 2 successes needed
        );

        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_parallel_threshold_not_met() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Failure);
        ctx.register_action("a3", || BehaviorStatus::Failure);

        let node = BehaviorNode::Parallel(
            vec![
                BehaviorNode::Action("a1".to_string()),
                BehaviorNode::Action("a2".to_string()),
                BehaviorNode::Action("a3".to_string()),
            ],
            2, // threshold: 2 successes needed
        );

        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    #[test]
    fn test_parallel_running() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Running);
        ctx.register_action("a3", || BehaviorStatus::Failure);

        let node = BehaviorNode::Parallel(
            vec![
                BehaviorNode::Action("a1".to_string()),
                BehaviorNode::Action("a2".to_string()),
                BehaviorNode::Action("a3".to_string()),
            ],
            2, // threshold: 2 successes needed
        );

        assert_eq!(node.tick(&ctx), BehaviorStatus::Running);
    }

    #[test]
    fn test_parallel_zero_threshold() {
        let ctx = BehaviorContext::new();
        let node = BehaviorNode::Parallel(vec![], 0);
        assert_eq!(node.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_parallel_threshold_exceeds_children() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);

        let node = BehaviorNode::Parallel(
            vec![BehaviorNode::Action("a1".to_string())],
            5, // threshold > children count
        );

        assert_eq!(node.tick(&ctx), BehaviorStatus::Failure);
    }

    // ===== BehaviorGraph Tests =====

    #[test]
    fn test_behavior_graph_creation() {
        let root = BehaviorNode::Action("test".to_string());
        let graph = BehaviorGraph::new(root);
        assert!(matches!(graph.root, BehaviorNode::Action(_)));
    }

    #[test]
    fn test_behavior_graph_tick() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("succeed", || BehaviorStatus::Success);

        let root = BehaviorNode::Action("succeed".to_string());
        let graph = BehaviorGraph::new(root);

        assert_eq!(graph.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_behavior_graph_current_node_name() {
        let root = BehaviorNode::Action("test".to_string());
        let graph = BehaviorGraph::new(root);

        assert_eq!(graph.current_node_name(), Some("root".to_string()));
    }

    // ===== Complex Integration Tests =====

    #[test]
    fn test_nested_sequence_selector() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Failure);
        ctx.register_action("a2", || BehaviorStatus::Success);
        ctx.register_action("a3", || BehaviorStatus::Success);
        ctx.register_action("a4", || BehaviorStatus::Success);

        // Selector [ Sequence [a1, a2], Sequence [a3, a4] ]
        let root = BehaviorNode::Selector(vec![
            BehaviorNode::Sequence(vec![
                BehaviorNode::Action("a1".to_string()),
                BehaviorNode::Action("a2".to_string()),
            ]),
            BehaviorNode::Sequence(vec![
                BehaviorNode::Action("a3".to_string()),
                BehaviorNode::Action("a4".to_string()),
            ]),
        ]);

        assert_eq!(root.tick(&ctx), BehaviorStatus::Success);
    }

    #[test]
    fn test_nested_decorator_sequence() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("fail", || BehaviorStatus::Failure);
        ctx.register_action("succeed", || BehaviorStatus::Success);

        // Sequence [ Inverter(fail), succeed ]
        let root = BehaviorNode::Sequence(vec![
            BehaviorNode::Decorator(
                DecoratorType::Inverter,
                Box::new(BehaviorNode::Action("fail".to_string())),
            ),
            BehaviorNode::Action("succeed".to_string()),
        ]);

        assert_eq!(root.tick(&ctx), BehaviorStatus::Success);
    }

    // ===== BehaviorNode Factory Method Tests =====

    #[test]
    fn test_behavior_node_action_factory() {
        let node = BehaviorNode::action("attack");
        assert!(node.is_action());
        assert_eq!(node.name(), Some("attack"));
    }

    #[test]
    fn test_behavior_node_condition_factory() {
        let node = BehaviorNode::condition("is_healthy");
        assert!(node.is_condition());
        assert_eq!(node.name(), Some("is_healthy"));
    }

    #[test]
    fn test_behavior_node_sequence_factory() {
        let node =
            BehaviorNode::sequence(vec![BehaviorNode::action("a1"), BehaviorNode::action("a2")]);
        assert!(node.is_sequence());
        assert_eq!(node.child_count(), 2);
    }

    #[test]
    fn test_behavior_node_selector_factory() {
        let node = BehaviorNode::selector(vec![
            BehaviorNode::action("a1"),
            BehaviorNode::condition("c1"),
        ]);
        assert!(node.is_selector());
        assert_eq!(node.child_count(), 2);
    }

    #[test]
    fn test_behavior_node_parallel_factory() {
        let node = BehaviorNode::parallel(
            vec![
                BehaviorNode::action("a1"),
                BehaviorNode::action("a2"),
                BehaviorNode::action("a3"),
            ],
            2,
        );
        assert!(node.is_parallel());
        assert_eq!(node.child_count(), 3);
    }

    #[test]
    fn test_behavior_node_decorator_factory() {
        let node = BehaviorNode::decorator(DecoratorType::Inverter, BehaviorNode::action("test"));
        assert!(node.is_decorator());
        assert_eq!(node.child_count(), 1);
    }

    // ===== BehaviorNode Query Method Tests =====

    #[test]
    fn test_behavior_node_is_leaf() {
        assert!(BehaviorNode::action("test").is_leaf());
        assert!(BehaviorNode::condition("test").is_leaf());
        assert!(!BehaviorNode::sequence(vec![]).is_leaf());
        assert!(!BehaviorNode::selector(vec![]).is_leaf());
        assert!(!BehaviorNode::parallel(vec![], 1).is_leaf());
    }

    #[test]
    fn test_behavior_node_is_composite() {
        assert!(!BehaviorNode::action("test").is_composite());
        assert!(!BehaviorNode::condition("test").is_composite());
        assert!(BehaviorNode::sequence(vec![]).is_composite());
        assert!(BehaviorNode::selector(vec![]).is_composite());
        assert!(BehaviorNode::parallel(vec![], 1).is_composite());
    }

    #[test]
    fn test_behavior_node_name_returns_none_for_composites() {
        assert!(BehaviorNode::sequence(vec![]).name().is_none());
        assert!(BehaviorNode::selector(vec![]).name().is_none());
        assert!(BehaviorNode::parallel(vec![], 1).name().is_none());
    }

    #[test]
    fn test_behavior_node_node_type() {
        assert_eq!(BehaviorNode::action("test").node_type(), "Action");
        assert_eq!(BehaviorNode::condition("test").node_type(), "Condition");
        assert_eq!(BehaviorNode::sequence(vec![]).node_type(), "Sequence");
        assert_eq!(BehaviorNode::selector(vec![]).node_type(), "Selector");
        assert_eq!(BehaviorNode::parallel(vec![], 1).node_type(), "Parallel");
        assert_eq!(
            BehaviorNode::decorator(DecoratorType::Inverter, BehaviorNode::action("test"))
                .node_type(),
            "Decorator"
        );
    }

    #[test]
    fn test_behavior_node_total_node_count() {
        // Single node
        assert_eq!(BehaviorNode::action("test").total_node_count(), 1);

        // Sequence with 3 actions
        let seq = BehaviorNode::sequence(vec![
            BehaviorNode::action("a1"),
            BehaviorNode::action("a2"),
            BehaviorNode::action("a3"),
        ]);
        assert_eq!(seq.total_node_count(), 4);

        // Nested structure
        let nested = BehaviorNode::selector(vec![
            BehaviorNode::sequence(vec![BehaviorNode::action("a1"), BehaviorNode::action("a2")]),
            BehaviorNode::action("a3"),
        ]);
        assert_eq!(nested.total_node_count(), 5);
    }

    #[test]
    fn test_behavior_node_max_depth() {
        // Single node
        assert_eq!(BehaviorNode::action("test").max_depth(), 1);

        // Sequence with actions
        let seq =
            BehaviorNode::sequence(vec![BehaviorNode::action("a1"), BehaviorNode::action("a2")]);
        assert_eq!(seq.max_depth(), 2);

        // Nested structure
        let nested = BehaviorNode::selector(vec![
            BehaviorNode::sequence(vec![BehaviorNode::action("a1")]),
            BehaviorNode::action("a2"),
        ]);
        assert_eq!(nested.max_depth(), 3);
    }

    #[test]
    fn test_behavior_node_summary() {
        assert_eq!(BehaviorNode::action("attack").summary(), "Action(attack)");
        assert_eq!(
            BehaviorNode::condition("ready").summary(),
            "Condition(ready)"
        );
        assert_eq!(
            BehaviorNode::sequence(vec![BehaviorNode::action("a1")]).summary(),
            "Sequence[1]"
        );
        assert_eq!(
            BehaviorNode::selector(vec![BehaviorNode::action("a1"), BehaviorNode::action("a2")])
                .summary(),
            "Selector[2]"
        );
        assert_eq!(
            BehaviorNode::parallel(vec![BehaviorNode::action("a1")], 1).summary(),
            "Parallel[1/1]"
        );
    }

    #[test]
    fn test_behavior_node_display() {
        let node = BehaviorNode::action("test");
        assert_eq!(format!("{}", node), "Action(test)");
    }

    // ===== DecoratorType Tests =====

    #[test]
    fn test_decorator_type_name() {
        assert_eq!(DecoratorType::Inverter.name(), "Inverter");
        assert_eq!(DecoratorType::Succeeder.name(), "Succeeder");
        assert_eq!(DecoratorType::Failer.name(), "Failer");
        assert_eq!(DecoratorType::Repeat(5).name(), "Repeat");
        assert_eq!(DecoratorType::Retry(3).name(), "Retry");
    }

    #[test]
    fn test_decorator_type_is_inverter() {
        assert!(DecoratorType::Inverter.is_inverter());
        assert!(!DecoratorType::Succeeder.is_inverter());
        assert!(!DecoratorType::Failer.is_inverter());
    }

    #[test]
    fn test_decorator_type_forces_result() {
        assert!(DecoratorType::Succeeder.forces_result());
        assert!(DecoratorType::Failer.forces_result());
        assert!(!DecoratorType::Inverter.forces_result());
        assert!(!DecoratorType::Repeat(3).forces_result());
    }

    #[test]
    fn test_decorator_type_is_looping() {
        assert!(DecoratorType::Repeat(3).is_looping());
        assert!(DecoratorType::Retry(5).is_looping());
        assert!(!DecoratorType::Inverter.is_looping());
        assert!(!DecoratorType::Succeeder.is_looping());
    }

    #[test]
    fn test_decorator_type_iteration_count() {
        assert_eq!(DecoratorType::Repeat(5).iteration_count(), Some(5));
        assert_eq!(DecoratorType::Retry(3).iteration_count(), Some(3));
        assert_eq!(DecoratorType::Inverter.iteration_count(), None);
        assert_eq!(DecoratorType::Succeeder.iteration_count(), None);
    }

    #[test]
    fn test_decorator_type_all() {
        let all = DecoratorType::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_decorator_type_display() {
        assert_eq!(format!("{}", DecoratorType::Inverter), "Inverter");
        assert_eq!(format!("{}", DecoratorType::Succeeder), "Succeeder");
        assert_eq!(format!("{}", DecoratorType::Failer), "Failer");
        assert_eq!(format!("{}", DecoratorType::Repeat(3)), "Repeat(3)");
        assert_eq!(format!("{}", DecoratorType::Retry(5)), "Retry(5)");
    }

    #[test]
    fn test_decorator_type_equality() {
        assert_eq!(DecoratorType::Inverter, DecoratorType::Inverter);
        assert_ne!(DecoratorType::Inverter, DecoratorType::Succeeder);
        assert_eq!(DecoratorType::Repeat(3), DecoratorType::Repeat(3));
        assert_ne!(DecoratorType::Repeat(3), DecoratorType::Repeat(5));
    }

    // ===== BehaviorGraph Tests =====

    #[test]
    fn test_behavior_graph_node_count() {
        let graph = BehaviorGraph::new(BehaviorNode::sequence(vec![
            BehaviorNode::action("a1"),
            BehaviorNode::action("a2"),
        ]));
        assert_eq!(graph.node_count(), 3);
    }

    #[test]
    fn test_behavior_graph_max_depth() {
        let graph = BehaviorGraph::new(BehaviorNode::selector(vec![BehaviorNode::sequence(vec![
            BehaviorNode::action("a1"),
        ])]));
        assert_eq!(graph.max_depth(), 3);
    }

    #[test]
    fn test_behavior_graph_root_type() {
        let graph = BehaviorGraph::new(BehaviorNode::sequence(vec![]));
        assert_eq!(graph.root_type(), "Sequence");
    }

    #[test]
    fn test_behavior_graph_is_leaf() {
        let leaf_graph = BehaviorGraph::new(BehaviorNode::action("test"));
        assert!(leaf_graph.is_leaf());

        let composite_graph = BehaviorGraph::new(BehaviorNode::sequence(vec![]));
        assert!(!composite_graph.is_leaf());
    }

    #[test]
    fn test_behavior_graph_is_composite() {
        let leaf_graph = BehaviorGraph::new(BehaviorNode::action("test"));
        assert!(!leaf_graph.is_composite());

        let composite_graph = BehaviorGraph::new(BehaviorNode::sequence(vec![]));
        assert!(composite_graph.is_composite());
    }

    #[test]
    fn test_behavior_graph_summary() {
        let graph = BehaviorGraph::new(BehaviorNode::sequence(vec![BehaviorNode::action("a1")]));
        let summary = graph.summary();
        assert!(summary.contains("Sequence"));
        assert!(summary.contains("nodes=2"));
        assert!(summary.contains("depth=2"));
    }

    #[test]
    fn test_behavior_graph_display() {
        let graph = BehaviorGraph::new(BehaviorNode::action("test"));
        let display = format!("{}", graph);
        assert!(display.contains("BehaviorGraph"));
    }

    // ===== BehaviorStatus Tests =====

    #[test]
    fn test_behavior_status_is_success() {
        assert!(BehaviorStatus::Success.is_success());
        assert!(!BehaviorStatus::Failure.is_success());
        assert!(!BehaviorStatus::Running.is_success());
    }

    #[test]
    fn test_behavior_status_is_failure() {
        assert!(!BehaviorStatus::Success.is_failure());
        assert!(BehaviorStatus::Failure.is_failure());
        assert!(!BehaviorStatus::Running.is_failure());
    }

    #[test]
    fn test_behavior_status_is_running() {
        assert!(!BehaviorStatus::Success.is_running());
        assert!(!BehaviorStatus::Failure.is_running());
        assert!(BehaviorStatus::Running.is_running());
    }

    #[test]
    fn test_behavior_status_is_terminal() {
        assert!(BehaviorStatus::Success.is_terminal());
        assert!(BehaviorStatus::Failure.is_terminal());
        assert!(!BehaviorStatus::Running.is_terminal());
    }

    #[test]
    fn test_behavior_status_name() {
        assert_eq!(BehaviorStatus::Success.name(), "Success");
        assert_eq!(BehaviorStatus::Failure.name(), "Failure");
        assert_eq!(BehaviorStatus::Running.name(), "Running");
    }

    #[test]
    fn test_behavior_status_invert() {
        assert_eq!(BehaviorStatus::Success.invert(), BehaviorStatus::Failure);
        assert_eq!(BehaviorStatus::Failure.invert(), BehaviorStatus::Success);
        assert_eq!(BehaviorStatus::Running.invert(), BehaviorStatus::Running);
    }

    #[test]
    fn test_behavior_status_to_success_if_running() {
        assert_eq!(
            BehaviorStatus::Running.to_success_if_running(),
            BehaviorStatus::Success
        );
        assert_eq!(
            BehaviorStatus::Success.to_success_if_running(),
            BehaviorStatus::Success
        );
        assert_eq!(
            BehaviorStatus::Failure.to_success_if_running(),
            BehaviorStatus::Failure
        );
    }

    #[test]
    fn test_behavior_status_to_failure_if_running() {
        assert_eq!(
            BehaviorStatus::Running.to_failure_if_running(),
            BehaviorStatus::Failure
        );
        assert_eq!(
            BehaviorStatus::Success.to_failure_if_running(),
            BehaviorStatus::Success
        );
        assert_eq!(
            BehaviorStatus::Failure.to_failure_if_running(),
            BehaviorStatus::Failure
        );
    }

    #[test]
    fn test_behavior_status_all() {
        let all = BehaviorStatus::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&BehaviorStatus::Success));
        assert!(all.contains(&BehaviorStatus::Failure));
        assert!(all.contains(&BehaviorStatus::Running));
    }

    #[test]
    fn test_behavior_status_display() {
        assert_eq!(format!("{}", BehaviorStatus::Success), "Success");
        assert_eq!(format!("{}", BehaviorStatus::Failure), "Failure");
        assert_eq!(format!("{}", BehaviorStatus::Running), "Running");
    }

    // ===== BehaviorContext Helper Tests =====

    #[test]
    fn test_behavior_context_action_count() {
        let mut ctx = BehaviorContext::new();
        assert_eq!(ctx.action_count(), 0);

        ctx.register_action("a1", || BehaviorStatus::Success);
        assert_eq!(ctx.action_count(), 1);

        ctx.register_action("a2", || BehaviorStatus::Failure);
        assert_eq!(ctx.action_count(), 2);
    }

    #[test]
    fn test_behavior_context_condition_count() {
        let mut ctx = BehaviorContext::new();
        assert_eq!(ctx.condition_count(), 0);

        ctx.register_condition("c1", || true);
        assert_eq!(ctx.condition_count(), 1);

        ctx.register_condition("c2", || false);
        assert_eq!(ctx.condition_count(), 2);
    }

    #[test]
    fn test_behavior_context_total_count() {
        let mut ctx = BehaviorContext::new();
        assert_eq!(ctx.total_count(), 0);

        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_condition("c1", || true);
        assert_eq!(ctx.total_count(), 2);
    }

    #[test]
    fn test_behavior_context_is_empty() {
        let mut ctx = BehaviorContext::new();
        assert!(ctx.is_empty());

        ctx.register_action("a1", || BehaviorStatus::Success);
        assert!(!ctx.is_empty());
    }

    #[test]
    fn test_behavior_context_has_action() {
        let mut ctx = BehaviorContext::new();
        assert!(!ctx.has_action("test"));

        ctx.register_action("test", || BehaviorStatus::Success);
        assert!(ctx.has_action("test"));
    }

    #[test]
    fn test_behavior_context_has_condition() {
        let mut ctx = BehaviorContext::new();
        assert!(!ctx.has_condition("test"));

        ctx.register_condition("test", || true);
        assert!(ctx.has_condition("test"));
    }

    #[test]
    fn test_behavior_context_action_names() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Failure);

        let names = ctx.action_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a1"));
        assert!(names.contains(&"a2"));
    }

    #[test]
    fn test_behavior_context_condition_names() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("c1", || true);
        ctx.register_condition("c2", || false);

        let names = ctx.condition_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"c1"));
        assert!(names.contains(&"c2"));
    }

    #[test]
    fn test_behavior_context_remove_action() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("test", || BehaviorStatus::Success);
        assert!(ctx.has_action("test"));

        assert!(ctx.remove_action("test"));
        assert!(!ctx.has_action("test"));

        assert!(!ctx.remove_action("nonexistent"));
    }

    #[test]
    fn test_behavior_context_remove_condition() {
        let mut ctx = BehaviorContext::new();
        ctx.register_condition("test", || true);
        assert!(ctx.has_condition("test"));

        assert!(ctx.remove_condition("test"));
        assert!(!ctx.has_condition("test"));

        assert!(!ctx.remove_condition("nonexistent"));
    }

    #[test]
    fn test_behavior_context_clear_actions() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_action("a2", || BehaviorStatus::Failure);
        ctx.register_condition("c1", || true);

        ctx.clear_actions();
        assert_eq!(ctx.action_count(), 0);
        assert_eq!(ctx.condition_count(), 1);
    }

    #[test]
    fn test_behavior_context_clear_conditions() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_condition("c1", || true);
        ctx.register_condition("c2", || false);

        ctx.clear_conditions();
        assert_eq!(ctx.action_count(), 1);
        assert_eq!(ctx.condition_count(), 0);
    }

    #[test]
    fn test_behavior_context_clear_all() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_condition("c1", || true);

        ctx.clear();
        assert!(ctx.is_empty());
        assert_eq!(ctx.action_count(), 0);
        assert_eq!(ctx.condition_count(), 0);
    }

    #[test]
    fn test_behavior_context_summary() {
        let mut ctx = BehaviorContext::new();
        ctx.register_action("a1", || BehaviorStatus::Success);
        ctx.register_condition("c1", || true);

        let summary = ctx.summary();
        assert!(summary.contains("1 actions"));
        assert!(summary.contains("1 conditions"));
    }

    #[test]
    fn test_behavior_context_display() {
        let ctx = BehaviorContext::new();
        let display = format!("{}", ctx);
        assert!(display.contains("BehaviorContext"));
    }

    // ═══════════════════════════════════════════════════════════════
    // MUTATION REMEDIATION TESTS — targets lib.rs missed mutants
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutation_is_parallel_only_for_parallel_variant() {
        // Targets: lib.rs:92 replace is_parallel -> bool with true
        assert!(!BehaviorNode::action("a").is_parallel());
        assert!(!BehaviorNode::condition("c").is_parallel());
        assert!(!BehaviorNode::sequence(vec![]).is_parallel());
        assert!(!BehaviorNode::selector(vec![]).is_parallel());
        assert!(BehaviorNode::parallel(vec![], 1).is_parallel());
    }

    #[test]
    fn mutation_total_node_count_addition_check() {
        // Targets: lib.rs:157 replace + with -/* in total_node_count
        // Single leaf
        assert_eq!(BehaviorNode::action("a").total_node_count(), 1);

        // Sequence with 2 children → 1 (seq) + 2 (leaves) = 3
        let seq =
            BehaviorNode::sequence(vec![BehaviorNode::action("a"), BehaviorNode::action("b")]);
        assert_eq!(seq.total_node_count(), 3);

        // If + was replaced with -, 1 - 2 would underflow or be negative
        // If + was replaced with *, 1 * 2 = 2 which != 3
        // Deeper: Selector > [Sequence > [Action, Action], Action] = 1 + (1 + 1 + 1) + 1 = 5
        let deep = BehaviorNode::selector(vec![
            BehaviorNode::sequence(vec![BehaviorNode::action("a"), BehaviorNode::action("b")]),
            BehaviorNode::action("c"),
        ]);
        assert_eq!(deep.total_node_count(), 5);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BehaviorStatus {
    Success,
    Failure,
    Running,
}

impl BehaviorStatus {
    /// Returns true if this is a success status.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Returns true if this is a failure status.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure)
    }

    /// Returns true if this is a running status.
    #[must_use]
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Returns true if this is a terminal status (success or failure).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Success | Self::Failure)
    }

    /// Returns the status name as a string.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::Failure => "Failure",
            Self::Running => "Running",
        }
    }

    /// Inverts the status (Success <-> Failure, Running unchanged).
    #[must_use]
    pub fn invert(&self) -> Self {
        match self {
            Self::Success => Self::Failure,
            Self::Failure => Self::Success,
            Self::Running => Self::Running,
        }
    }

    /// Converts to success if currently running.
    #[must_use]
    pub fn to_success_if_running(&self) -> Self {
        match self {
            Self::Running => Self::Success,
            other => *other,
        }
    }

    /// Converts to failure if currently running.
    #[must_use]
    pub fn to_failure_if_running(&self) -> Self {
        match self {
            Self::Running => Self::Failure,
            other => *other,
        }
    }

    /// Returns all status variants.
    #[must_use]
    pub fn all() -> &'static [BehaviorStatus] {
        &[Self::Success, Self::Failure, Self::Running]
    }
}

impl fmt::Display for BehaviorStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Context for behavior evaluation
pub struct BehaviorContext {
    pub actions: HashMap<String, Box<dyn Fn() -> BehaviorStatus + Send + Sync>>,
    pub conditions: HashMap<String, Box<dyn Fn() -> bool + Send + Sync>>,
}

impl Default for BehaviorContext {
    fn default() -> Self {
        Self::new()
    }
}

impl BehaviorContext {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
            conditions: HashMap::new(),
        }
    }

    pub fn register_action<F>(&mut self, name: &str, f: F)
    where
        F: Fn() -> BehaviorStatus + Send + Sync + 'static,
    {
        self.actions.insert(name.to_string(), Box::new(f));
    }

    pub fn register_condition<F>(&mut self, name: &str, f: F)
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.conditions.insert(name.to_string(), Box::new(f));
    }

    /// Returns the number of registered actions.
    #[must_use]
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    /// Returns the number of registered conditions.
    #[must_use]
    pub fn condition_count(&self) -> usize {
        self.conditions.len()
    }

    /// Returns the total number of registered handlers.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.actions.len() + self.conditions.len()
    }

    /// Returns true if the context has no registered actions or conditions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty() && self.conditions.is_empty()
    }

    /// Returns true if an action with the given name is registered.
    #[must_use]
    pub fn has_action(&self, name: &str) -> bool {
        self.actions.contains_key(name)
    }

    /// Returns true if a condition with the given name is registered.
    #[must_use]
    pub fn has_condition(&self, name: &str) -> bool {
        self.conditions.contains_key(name)
    }

    /// Returns all registered action names.
    #[must_use]
    pub fn action_names(&self) -> Vec<&str> {
        self.actions.keys().map(|s| s.as_str()).collect()
    }

    /// Returns all registered condition names.
    #[must_use]
    pub fn condition_names(&self) -> Vec<&str> {
        self.conditions.keys().map(|s| s.as_str()).collect()
    }

    /// Removes an action by name. Returns true if an action was removed.
    pub fn remove_action(&mut self, name: &str) -> bool {
        self.actions.remove(name).is_some()
    }

    /// Removes a condition by name. Returns true if a condition was removed.
    pub fn remove_condition(&mut self, name: &str) -> bool {
        self.conditions.remove(name).is_some()
    }

    /// Clears all registered actions.
    pub fn clear_actions(&mut self) {
        self.actions.clear();
    }

    /// Clears all registered conditions.
    pub fn clear_conditions(&mut self) {
        self.conditions.clear();
    }

    /// Clears all registered actions and conditions.
    pub fn clear(&mut self) {
        self.actions.clear();
        self.conditions.clear();
    }

    /// Returns a brief summary of the context.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "BehaviorContext: {} actions, {} conditions",
            self.action_count(),
            self.condition_count()
        )
    }

    fn evaluate_action(&self, name: &str) -> BehaviorStatus {
        if let Some(action) = self.actions.get(name) {
            action()
        } else {
            debug_assert!(false, "Action '{}' not registered in BehaviorContext", name);
            BehaviorStatus::Failure
        }
    }

    fn evaluate_condition(&self, name: &str) -> BehaviorStatus {
        if let Some(condition) = self.conditions.get(name) {
            if condition() {
                BehaviorStatus::Success
            } else {
                BehaviorStatus::Failure
            }
        } else {
            debug_assert!(
                false,
                "Condition '{}' not registered in BehaviorContext",
                name
            );
            BehaviorStatus::Failure
        }
    }
}

impl fmt::Display for BehaviorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}
