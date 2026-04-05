use super::{Action, ActionHistory, Goal, WorldState};
use std::fmt::Write;

/// Visualization format for plans and goals
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VisualizationFormat {
    /// ASCII tree with Unicode box drawing characters
    AsciiTree,
    /// ASCII timeline showing execution sequence
    AsciiTimeline,
    /// DOT format for GraphViz
    Dot,
    /// Simple text list
    Text,
    /// JSON for programmatic consumption
    Json,
}

/// Plan visualizer for rendering plans and goal hierarchies
pub struct PlanVisualizer {
    format: VisualizationFormat,
    show_costs: bool,
    show_risks: bool,
    show_state_changes: bool,
}

impl PlanVisualizer {
    /// Create a new visualizer with specified format
    pub fn new(format: VisualizationFormat) -> Self {
        Self {
            format,
            show_costs: true,
            show_risks: true,
            show_state_changes: false,
        }
    }

    /// Enable/disable cost display
    pub fn with_costs(mut self, show: bool) -> Self {
        self.show_costs = show;
        self
    }

    /// Enable/disable risk display
    pub fn with_risks(mut self, show: bool) -> Self {
        self.show_risks = show;
        self
    }

    /// Enable/disable state change display
    pub fn with_state_changes(mut self, show: bool) -> Self {
        self.show_state_changes = show;
        self
    }

    /// Visualize an action plan
    pub fn visualize_plan(
        &self,
        plan: &[String],
        actions: &[Box<dyn Action>],
        history: &ActionHistory,
        start_state: &WorldState,
    ) -> String {
        match self.format {
            VisualizationFormat::AsciiTree => self.render_plan_tree(plan, actions, history),
            VisualizationFormat::AsciiTimeline => {
                self.render_plan_timeline(plan, actions, history, start_state)
            }
            VisualizationFormat::Dot => self.render_plan_dot(plan, actions, history),
            VisualizationFormat::Text => self.render_plan_text(plan, actions, history),
            VisualizationFormat::Json => self.render_plan_json(plan, actions, history),
        }
    }

    /// Visualize a goal hierarchy
    pub fn visualize_goal_hierarchy(&self, goal: &Goal) -> String {
        match self.format {
            VisualizationFormat::AsciiTree => self.render_goal_tree(goal, 0, true),
            VisualizationFormat::Dot => self.render_goal_dot(goal),
            VisualizationFormat::Text => self.render_goal_text(goal, 0),
            _ => self.render_goal_tree(goal, 0, true),
        }
    }

    /// Render plan as ASCII tree
    fn render_plan_tree(
        &self,
        plan: &[String],
        actions: &[Box<dyn Action>],
        history: &ActionHistory,
    ) -> String {
        let mut output = String::new();

        // Calculate total metrics
        let (total_cost, total_risk) = self.calculate_plan_metrics(plan, actions, history);

        let _ = writeln!(
            &mut output,
            "Plan ({} actions, cost: {:.1}, risk: {:.2})",
            plan.len(),
            total_cost,
            total_risk
        );

        for (i, action_name) in plan.iter().enumerate() {
            let is_last = i == plan.len() - 1;
            let prefix = if is_last { "└─" } else { "├─" };

            if let Some(action) = actions.iter().find(|a| a.name() == action_name) {
                let cost = action.calculate_cost(&WorldState::new(), history);
                let risk = 1.0 - action.success_probability(&WorldState::new(), history);

                if self.show_costs && self.show_risks {
                    let _ = writeln!(
                        &mut output,
                        "{} {} (cost: {:.1}, risk: {:.2})",
                        prefix, action_name, cost, risk
                    );
                } else if self.show_costs {
                    let _ = writeln!(
                        &mut output,
                        "{} {} (cost: {:.1})",
                        prefix, action_name, cost
                    );
                } else if self.show_risks {
                    let _ = writeln!(
                        &mut output,
                        "{} {} (risk: {:.2})",
                        prefix, action_name, risk
                    );
                } else {
                    let _ = writeln!(&mut output, "{} {}", prefix, action_name);
                }
            } else {
                let _ = writeln!(&mut output, "{} {} (unknown)", prefix, action_name);
            }
        }

        output
    }

    /// Render plan as timeline
    fn render_plan_timeline(
        &self,
        plan: &[String],
        actions: &[Box<dyn Action>],
        history: &ActionHistory,
        start_state: &WorldState,
    ) -> String {
        let mut output = String::new();
        let mut current_time = 0.0;
        let mut current_state = start_state.clone();

        // Header
        let _ = writeln!(
            &mut output,
            "{:<6} | {:<20} | {:<30} | {}",
            "Time", "Action", "State Changes", "Success"
        );
        let _ = writeln!(&mut output, "{}", "-".repeat(80));

        for action_name in plan {
            if let Some(action) = actions.iter().find(|a| a.name() == action_name) {
                let success_prob = action.success_probability(&current_state, history);
                let duration = history
                    .get_action_stats(action_name)
                    .map(|s| s.avg_duration)
                    .unwrap_or(1.0);

                let state_changes = if self.show_state_changes {
                    self.format_state_changes(action.effects())
                } else {
                    String::from("...")
                };

                let success_icon = if success_prob > 0.8 {
                    "✓"
                } else if success_prob > 0.5 {
                    "~"
                } else {
                    "✗"
                };

                let _ = writeln!(
                    &mut output,
                    "{:<6.1} | {:<20} | {:<30} | {}",
                    current_time, action_name, state_changes, success_icon
                );

                current_state.apply_effects(action.effects());
                current_time += duration;
            }
        }

        output
    }

    /// Render plan as DOT (GraphViz)
    fn render_plan_dot(
        &self,
        plan: &[String],
        actions: &[Box<dyn Action>],
        history: &ActionHistory,
    ) -> String {
        let mut output = String::new();

        let _ = writeln!(&mut output, "digraph Plan {{");
        let _ = writeln!(&mut output, "  rankdir=LR;");
        let _ = writeln!(&mut output, "  node [shape=box];");

        let _ = writeln!(&mut output, "  start [label=\"Start\", shape=circle];");

        for (i, action_name) in plan.iter().enumerate() {
            let node_id = format!("action_{}", i);

            if let Some(action) = actions.iter().find(|a| a.name() == action_name) {
                let cost = action.calculate_cost(&WorldState::new(), history);
                let risk = 1.0 - action.success_probability(&WorldState::new(), history);

                let label = if self.show_costs && self.show_risks {
                    format!("{}\\ncost: {:.1}\\nrisk: {:.2}", action_name, cost, risk)
                } else {
                    action_name.clone()
                };

                let _ = writeln!(&mut output, "  {} [label=\"{}\"];", node_id, label);
            } else {
                let _ = writeln!(&mut output, "  {} [label=\"{}\"];", node_id, action_name);
            }

            if i == 0 {
                let _ = writeln!(&mut output, "  start -> {};", node_id);
            } else {
                let _ = writeln!(&mut output, "  action_{} -> {};", i - 1, node_id);
            }
        }

        if !plan.is_empty() {
            let _ = writeln!(&mut output, "  action_{} -> end;", plan.len() - 1);
        }

        let _ = writeln!(&mut output, "  end [label=\"End\", shape=doublecircle];");
        let _ = writeln!(&mut output, "}}");

        output
    }

    /// Render plan as simple text
    fn render_plan_text(
        &self,
        plan: &[String],
        actions: &[Box<dyn Action>],
        history: &ActionHistory,
    ) -> String {
        let mut output = String::new();

        for (i, action_name) in plan.iter().enumerate() {
            let _ = write!(&mut output, "{}. {}", i + 1, action_name);

            if let Some(action) = actions.iter().find(|a| a.name() == action_name) {
                if self.show_costs || self.show_risks {
                    let cost = action.calculate_cost(&WorldState::new(), history);
                    let risk = 1.0 - action.success_probability(&WorldState::new(), history);

                    let _ = write!(&mut output, " (");
                    if self.show_costs {
                        let _ = write!(&mut output, "cost: {:.1}", cost);
                    }
                    if self.show_costs && self.show_risks {
                        let _ = write!(&mut output, ", ");
                    }
                    if self.show_risks {
                        let _ = write!(&mut output, "risk: {:.2}", risk);
                    }
                    let _ = write!(&mut output, ")");
                }
            }

            let _ = writeln!(&mut output);
        }

        output
    }

    /// Render plan as JSON
    fn render_plan_json(
        &self,
        plan: &[String],
        actions: &[Box<dyn Action>],
        history: &ActionHistory,
    ) -> String {
        let mut output = String::new();

        let _ = writeln!(&mut output, "{{");
        let _ = writeln!(&mut output, "  \"actions\": [");

        for (i, action_name) in plan.iter().enumerate() {
            let comma = if i < plan.len() - 1 { "," } else { "" };

            if let Some(action) = actions.iter().find(|a| a.name() == action_name) {
                let cost = action.calculate_cost(&WorldState::new(), history);
                let risk = 1.0 - action.success_probability(&WorldState::new(), history);

                let _ = writeln!(&mut output, "    {{");
                let _ = writeln!(&mut output, "      \"name\": \"{}\",", action_name);
                let _ = writeln!(&mut output, "      \"cost\": {:.2},", cost);
                let _ = writeln!(&mut output, "      \"risk\": {:.2}", risk);
                let _ = writeln!(&mut output, "    }}{}", comma);
            } else {
                let _ = writeln!(&mut output, "    {{");
                let _ = writeln!(&mut output, "      \"name\": \"{}\"", action_name);
                let _ = writeln!(&mut output, "    }}{}", comma);
            }
        }

        let _ = writeln!(&mut output, "  ]");
        let _ = writeln!(&mut output, "}}");

        output
    }

    /// Render goal hierarchy as tree
    fn render_goal_tree(&self, goal: &Goal, depth: usize, is_last: bool) -> String {
        let mut output = String::new();

        let indent = if depth == 0 {
            String::new()
        } else {
            "  ".repeat(depth - 1) + if is_last { "  └─ " } else { "  ├─ " }
        };

        let strategy_str = match goal.decomposition_strategy {
            super::DecompositionStrategy::Sequential => "[SEQ]",
            super::DecompositionStrategy::Parallel => "[PAR]",
            super::DecompositionStrategy::AnyOf => "[ANY]",
            super::DecompositionStrategy::AllOf => "[ALL]",
        };

        let priority_str = format!("priority: {:.1}", goal.priority);
        let deadline_str = if let Some(deadline) = goal.deadline {
            format!(", deadline: {:.0}s", deadline)
        } else {
            String::new()
        };

        let _ = writeln!(
            &mut output,
            "{}{} {} ({}{})",
            indent, strategy_str, goal.name, priority_str, deadline_str
        );

        for (i, sub_goal) in goal.sub_goals.iter().enumerate() {
            let is_last_sub = i == goal.sub_goals.len() - 1;
            output.push_str(&self.render_goal_tree(sub_goal, depth + 1, is_last_sub));
        }

        output
    }

    /// Render goal hierarchy as DOT
    fn render_goal_dot(&self, goal: &Goal) -> String {
        let mut output = String::new();
        let mut node_counter = 0;

        let _ = writeln!(&mut output, "digraph GoalHierarchy {{");
        let _ = writeln!(&mut output, "  rankdir=TB;");
        let _ = writeln!(&mut output, "  node [shape=box];");

        self.render_goal_dot_recursive(goal, &mut output, &mut node_counter, None);

        let _ = writeln!(&mut output, "}}");

        output
    }

    fn render_goal_dot_recursive(
        &self,
        goal: &Goal,
        output: &mut String,
        node_counter: &mut usize,
        parent_id: Option<usize>,
    ) {
        let current_id = *node_counter;
        *node_counter += 1;

        let strategy_str = match goal.decomposition_strategy {
            super::DecompositionStrategy::Sequential => "SEQ",
            super::DecompositionStrategy::Parallel => "PAR",
            super::DecompositionStrategy::AnyOf => "ANY",
            super::DecompositionStrategy::AllOf => "ALL",
        };

        let _ = writeln!(
            output,
            "  node_{} [label=\"[{}] {}\\npriority: {:.1}\"];",
            current_id, strategy_str, goal.name, goal.priority
        );

        if let Some(parent) = parent_id {
            let _ = writeln!(output, "  node_{} -> node_{};", parent, current_id);
        }

        for sub_goal in &goal.sub_goals {
            self.render_goal_dot_recursive(sub_goal, output, node_counter, Some(current_id));
        }
    }

    /// Render goal as text
    fn render_goal_text(&self, goal: &Goal, depth: usize) -> String {
        let mut output = String::new();

        let indent = "  ".repeat(depth);
        let _ = writeln!(
            &mut output,
            "{}{} (priority: {:.1})",
            indent, goal.name, goal.priority
        );

        for sub_goal in &goal.sub_goals {
            output.push_str(&self.render_goal_text(sub_goal, depth + 1));
        }

        output
    }

    /// Calculate total plan metrics
    fn calculate_plan_metrics(
        &self,
        plan: &[String],
        actions: &[Box<dyn Action>],
        history: &ActionHistory,
    ) -> (f32, f32) {
        let mut total_cost = 0.0;
        let mut total_risk = 0.0;

        for action_name in plan {
            if let Some(action) = actions.iter().find(|a| a.name() == action_name) {
                total_cost += action.calculate_cost(&WorldState::new(), history);
                total_risk += 1.0 - action.success_probability(&WorldState::new(), history);
            }
        }

        (total_cost, total_risk)
    }

    /// Format state changes for display
    fn format_state_changes(
        &self,
        effects: &std::collections::BTreeMap<String, super::StateValue>,
    ) -> String {
        if effects.is_empty() {
            return String::from("(no changes)");
        }

        let changes: Vec<String> = effects
            .iter()
            .take(2) // Limit to 2 for space
            .map(|(k, v)| format!("{}={:?}", k, v))
            .collect();

        let mut result = changes.join(", ");
        if effects.len() > 2 {
            result.push_str(&format!(", +{} more", effects.len() - 2));
        }
        result
    }
}

impl Default for PlanVisualizer {
    fn default() -> Self {
        Self::new(VisualizationFormat::AsciiTree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goap::SimpleAction;
    use std::collections::BTreeMap;

    fn create_test_action(name: &str, cost: f32) -> Box<dyn Action> {
        Box::new(SimpleAction::new(
            name,
            BTreeMap::new(),
            BTreeMap::new(),
            cost,
        ))
    }

    #[test]
    fn test_visualize_simple_plan() {
        let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTree);
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("move", 1.0),
            create_test_action("attack", 2.0),
        ];
        let plan = vec!["move".to_string(), "attack".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);

        assert!(output.contains("Plan"));
        assert!(output.contains("move"));
        assert!(output.contains("attack"));
        assert!(output.contains("cost"));
    }

    #[test]
    fn test_visualize_goal_hierarchy() {
        let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTree);

        let sub_goal = Goal::new("sub", BTreeMap::new()).with_priority(5.0);
        let main_goal = Goal::new("main", BTreeMap::new())
            .with_priority(10.0)
            .with_sub_goals(vec![sub_goal]);

        let output = visualizer.visualize_goal_hierarchy(&main_goal);

        assert!(output.contains("main"));
        assert!(output.contains("sub"));
        assert!(output.contains("priority"));
    }

    #[test]
    fn test_text_format() {
        let visualizer = PlanVisualizer::new(VisualizationFormat::Text);
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("action1", 1.0),
            create_test_action("action2", 2.0),
        ];
        let plan = vec!["action1".to_string(), "action2".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);

        assert!(output.contains("1. action1"));
        assert!(output.contains("2. action2"));
    }

    #[test]
    fn test_dot_format() {
        let visualizer = PlanVisualizer::new(VisualizationFormat::Dot);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("action1", 1.0)];
        let plan = vec!["action1".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);

        assert!(output.contains("digraph Plan"));
        assert!(output.contains("action_0"));
        assert!(output.contains("start"));
        assert!(output.contains("end"));
    }

    #[test]
    fn test_timeline_format() {
        let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTimeline);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("action1", 1.0)];
        let plan = vec!["action1".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);

        assert!(output.contains("Time"));
        assert!(output.contains("Action"));
        assert!(output.contains("Success"));
    }

    #[test]
    fn test_json_format() {
        let visualizer = PlanVisualizer::new(VisualizationFormat::Json);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("action1", 1.0)];
        let plan = vec!["action1".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);

        assert!(output.contains("\"actions\""));
        assert!(output.contains("\"name\""));
        assert!(output.contains("\"cost\""));
    }

    #[test]
    fn test_with_options() {
        let visualizer = PlanVisualizer::new(VisualizationFormat::Text)
            .with_costs(false)
            .with_risks(false);

        let actions: Vec<Box<dyn Action>> = vec![create_test_action("action1", 1.0)];
        let plan = vec!["action1".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);

        assert!(!output.contains("cost"));
        assert!(!output.contains("risk"));
    }

    // ── Mutation-killing tests ──

    fn create_test_action_with_effects(
        name: &str,
        cost: f32,
        effects: BTreeMap<String, super::super::StateValue>,
    ) -> Box<dyn Action> {
        Box::new(SimpleAction::new(name, BTreeMap::new(), effects, cost))
    }

    #[test]
    fn test_calculate_plan_metrics_exact() {
        // Kills: total_cost += → -=, total_risk += → -=, 1.0 - prob → prob
        let visualizer = PlanVisualizer::new(VisualizationFormat::Text);
        let actions: Vec<Box<dyn Action>> =
            vec![create_test_action("a", 3.0), create_test_action("b", 5.0)];
        let plan = vec!["a".to_string(), "b".to_string()];
        let history = ActionHistory::new();

        let (cost, risk) = visualizer.calculate_plan_metrics(&plan, &actions, &history);
        assert_eq!(cost, 8.0); // 3+5
                               // Default prob=0.8, risk=0.2 each, total=0.4
        assert!((risk - 0.4).abs() < 1e-6);

        // Empty plan
        let (c2, r2) = visualizer.calculate_plan_metrics(&[], &actions, &history);
        assert_eq!(c2, 0.0);
        assert_eq!(r2, 0.0);
    }

    #[test]
    fn test_format_state_changes_branches() {
        use super::super::StateValue;
        let visualizer =
            PlanVisualizer::new(VisualizationFormat::AsciiTimeline).with_state_changes(true);

        // Empty effects → "(no changes)"
        let empty = BTreeMap::new();
        let result = visualizer.format_state_changes(&empty);
        assert_eq!(result, "(no changes)");

        // 1 effect → no "+N more"
        let mut one = BTreeMap::new();
        one.insert("health".to_string(), StateValue::Int(100));
        let r1 = visualizer.format_state_changes(&one);
        assert!(r1.contains("health"));
        assert!(!r1.contains("more"));

        // 2 effects → no "+N more"
        let mut two = BTreeMap::new();
        two.insert("a".to_string(), StateValue::Bool(true));
        two.insert("b".to_string(), StateValue::Bool(false));
        let r2 = visualizer.format_state_changes(&two);
        assert!(!r2.contains("more"));

        // 3 effects → "+1 more"
        let mut three = BTreeMap::new();
        three.insert("a".to_string(), StateValue::Bool(true));
        three.insert("b".to_string(), StateValue::Bool(false));
        three.insert("c".to_string(), StateValue::Int(5));
        let r3 = visualizer.format_state_changes(&three);
        assert!(r3.contains("+1 more"));

        // 4 effects → "+2 more"
        let mut four = BTreeMap::new();
        four.insert("a".to_string(), StateValue::Bool(true));
        four.insert("b".to_string(), StateValue::Bool(false));
        four.insert("c".to_string(), StateValue::Int(5));
        four.insert("d".to_string(), StateValue::Int(10));
        let r4 = visualizer.format_state_changes(&four);
        assert!(r4.contains("+2 more"));
    }

    #[test]
    fn test_timeline_success_icons() {
        // Kills: > 0.8 → >= 0.8, > 0.5 → >= 0.5, icon assignments
        let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTimeline);

        // No history → default prob 0.8, which is NOT > 0.8, so should be "~"
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("act", 1.0)];
        let plan = vec!["act".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);
        assert!(
            output.contains("~"),
            "0.8 probability should show ~ (not > 0.8)"
        );

        // Build history with 100% success → prob > 0.8 → "✓"
        let mut hist_good = ActionHistory::new();
        for _ in 0..10 {
            hist_good.record_success("act", 1.0);
        }
        let output2 = visualizer.visualize_plan(&plan, &actions, &hist_good, &start);
        assert!(output2.contains("✓"), "100% success should show ✓");

        // Build history with ~30% success → prob < 0.5 → "✗"
        let mut hist_bad = ActionHistory::new();
        hist_bad.record_success("act", 1.0);
        hist_bad.record_failure("act");
        hist_bad.record_failure("act");
        hist_bad.record_failure("act");
        let output3 = visualizer.visualize_plan(&plan, &actions, &hist_bad, &start);
        assert!(output3.contains("✗"), "25% success should show ✗");
    }

    #[test]
    fn test_timeline_with_state_changes() {
        use super::super::StateValue;
        let visualizer =
            PlanVisualizer::new(VisualizationFormat::AsciiTimeline).with_state_changes(true);

        let mut effects = BTreeMap::new();
        effects.insert("health".to_string(), StateValue::Int(100));
        let actions: Vec<Box<dyn Action>> =
            vec![create_test_action_with_effects("heal", 1.0, effects)];
        let plan = vec!["heal".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);
        assert!(output.contains("health"), "State changes should be shown");
    }

    #[test]
    fn test_timeline_without_state_changes() {
        let visualizer =
            PlanVisualizer::new(VisualizationFormat::AsciiTimeline).with_state_changes(false);

        let actions: Vec<Box<dyn Action>> = vec![create_test_action("act", 1.0)];
        let plan = vec!["act".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);
        assert!(
            output.contains("..."),
            "Without state_changes, should show ..."
        );
    }

    #[test]
    fn test_tree_unknown_action() {
        // Plan references an action not in the actions list
        let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTree);
        let actions: Vec<Box<dyn Action>> = vec![]; // No actions registered
        let plan = vec!["phantom".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);
        assert!(
            output.contains("unknown"),
            "Unknown action should say (unknown)"
        );
    }

    #[test]
    fn test_tree_prefix_last_vs_non_last() {
        // Kills: is_last toggling mutations for ├─ vs └─
        let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTree);
        let actions: Vec<Box<dyn Action>> =
            vec![create_test_action("a", 1.0), create_test_action("b", 1.0)];
        let plan = vec!["a".to_string(), "b".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);
        assert!(output.contains("├─"), "Non-last action should use ├─");
        assert!(output.contains("└─"), "Last action should use └─");
    }

    #[test]
    fn test_tree_cost_risk_flags() {
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("myact", 1.0)];
        let plan = vec!["myact".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        // costs=true, risks=false — action line has "cost:" but not "risk:"
        let v1 = PlanVisualizer::new(VisualizationFormat::AsciiTree)
            .with_costs(true)
            .with_risks(false);
        let o1 = v1.visualize_plan(&plan, &actions, &history, &start);
        // Find the action line (contains "myact" but NOT the header line starting with "Plan")
        let action_line = o1
            .lines()
            .find(|l| l.contains("myact") && !l.starts_with("Plan"))
            .unwrap();
        assert!(action_line.contains("cost:"));
        assert!(!action_line.contains("risk:"));

        // costs=false, risks=true
        let v2 = PlanVisualizer::new(VisualizationFormat::AsciiTree)
            .with_costs(false)
            .with_risks(true);
        let o2 = v2.visualize_plan(&plan, &actions, &history, &start);
        let action_line2 = o2
            .lines()
            .find(|l| l.contains("myact") && !l.starts_with("Plan"))
            .unwrap();
        assert!(action_line2.contains("risk:"));
        assert!(!action_line2.contains("cost:"));

        // costs=false, risks=false
        let v3 = PlanVisualizer::new(VisualizationFormat::AsciiTree)
            .with_costs(false)
            .with_risks(false);
        let o3 = v3.visualize_plan(&plan, &actions, &history, &start);
        let action_line3 = o3
            .lines()
            .find(|l| l.contains("myact") && !l.starts_with("Plan"))
            .unwrap();
        assert!(!action_line3.contains("cost:"));
        assert!(!action_line3.contains("risk:"));
    }

    #[test]
    fn test_dot_edge_connections() {
        // Kills: i==0 → i==1, i-1 → i+1, plan.len()-1 mutations
        let visualizer = PlanVisualizer::new(VisualizationFormat::Dot);
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("first", 1.0),
            create_test_action("second", 2.0),
        ];
        let plan = vec!["first".to_string(), "second".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);
        assert!(
            output.contains("start -> action_0"),
            "First action connects from start"
        );
        assert!(
            output.contains("action_0 -> action_1"),
            "Actions chain together"
        );
        assert!(
            output.contains("action_1 -> end"),
            "Last action connects to end"
        );
    }

    #[test]
    fn test_json_comma_logic() {
        // Kills: i < plan.len()-1 boundary, comma placement
        let visualizer = PlanVisualizer::new(VisualizationFormat::Json);
        let actions: Vec<Box<dyn Action>> =
            vec![create_test_action("a", 1.0), create_test_action("b", 2.0)];
        let plan = vec!["a".to_string(), "b".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);
        // The first action's closing brace should have comma, the last should not
        let lines: Vec<&str> = output.lines().collect();
        // Find lines with closing braces for action objects
        let closing_braces: Vec<&&str> = lines
            .iter()
            .filter(|l| l.trim().starts_with('}') || l.trim().starts_with("}"))
            .collect();
        // At least first closing brace has comma
        assert!(
            output.contains("},"),
            "First action should have trailing comma"
        );
    }

    #[test]
    fn test_json_unknown_action() {
        let visualizer = PlanVisualizer::new(VisualizationFormat::Json);
        let actions: Vec<Box<dyn Action>> = vec![]; // No actions
        let plan = vec!["unknown_action".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &history, &start);
        assert!(output.contains("unknown_action"));
        // Should NOT contain "cost" or "risk" for unknown action
        // Check that the unknown action block only has "name"
        assert!(output.contains("\"name\": \"unknown_action\""));
    }

    #[test]
    fn test_goal_dot_format() {
        let visualizer = PlanVisualizer::new(VisualizationFormat::Dot);
        let sub = Goal::new("child", BTreeMap::new()).with_priority(5.0);
        let goal = Goal::new("parent", BTreeMap::new())
            .with_priority(10.0)
            .with_sub_goals(vec![sub]);

        let output = visualizer.visualize_goal_hierarchy(&goal);
        assert!(output.contains("digraph GoalHierarchy"));
        assert!(output.contains("node_0"));
        assert!(output.contains("node_1"));
        assert!(
            output.contains("node_0 -> node_1"),
            "Parent should connect to child"
        );
        assert!(output.contains("parent"));
        assert!(output.contains("child"));
    }

    #[test]
    fn test_goal_tree_strategy_tags() {
        use super::super::DecompositionStrategy;

        let viz = PlanVisualizer::new(VisualizationFormat::AsciiTree);

        let goal_seq =
            Goal::new("g", BTreeMap::new()).with_strategy(DecompositionStrategy::Sequential);
        assert!(viz.visualize_goal_hierarchy(&goal_seq).contains("[SEQ]"));

        let goal_par =
            Goal::new("g", BTreeMap::new()).with_strategy(DecompositionStrategy::Parallel);
        assert!(viz.visualize_goal_hierarchy(&goal_par).contains("[PAR]"));

        let goal_any = Goal::new("g", BTreeMap::new()).with_strategy(DecompositionStrategy::AnyOf);
        assert!(viz.visualize_goal_hierarchy(&goal_any).contains("[ANY]"));

        let goal_all = Goal::new("g", BTreeMap::new()).with_strategy(DecompositionStrategy::AllOf);
        assert!(viz.visualize_goal_hierarchy(&goal_all).contains("[ALL]"));
    }

    #[test]
    fn test_goal_tree_deadline_display() {
        let viz = PlanVisualizer::new(VisualizationFormat::AsciiTree);

        let with_dl = Goal::new("urgent", BTreeMap::new()).with_deadline(30.0);
        let output = viz.visualize_goal_hierarchy(&with_dl);
        assert!(output.contains("deadline: 30s"));

        let without_dl = Goal::new("relaxed", BTreeMap::new());
        let output2 = viz.visualize_goal_hierarchy(&without_dl);
        assert!(!output2.contains("deadline"));
    }

    #[test]
    fn test_goal_tree_depth_indent() {
        let viz = PlanVisualizer::new(VisualizationFormat::AsciiTree);
        let child = Goal::new("child", BTreeMap::new());
        let parent = Goal::new("parent", BTreeMap::new()).with_sub_goals(vec![child]);

        let output = viz.visualize_goal_hierarchy(&parent);
        // Parent at depth 0 has no indent
        assert!(output.starts_with("["));
        // Child at depth 1 should have indent with └─
        assert!(output.contains("└─"));
    }

    #[test]
    fn test_goal_text_format() {
        let viz = PlanVisualizer::new(VisualizationFormat::Text);
        let child = Goal::new("child", BTreeMap::new()).with_priority(3.0);
        let parent = Goal::new("parent", BTreeMap::new())
            .with_priority(8.0)
            .with_sub_goals(vec![child]);

        let output = viz.visualize_goal_hierarchy(&parent);
        assert!(output.contains("parent"));
        assert!(output.contains("priority: 8.0"));
        assert!(output.contains("child"));
        assert!(output.contains("priority: 3.0"));
        // Child should be more indented than parent
        let parent_indent = output
            .lines()
            .find(|l| l.contains("parent"))
            .unwrap()
            .find('p')
            .unwrap();
        let child_indent = output
            .lines()
            .find(|l| l.contains("child"))
            .unwrap()
            .find('c')
            .unwrap();
        assert!(child_indent > parent_indent);
    }

    #[test]
    fn test_text_format_cost_only_risk_only() {
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("act", 2.5)];
        let plan = vec!["act".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        // cost only
        let v1 = PlanVisualizer::new(VisualizationFormat::Text)
            .with_costs(true)
            .with_risks(false);
        let o1 = v1.visualize_plan(&plan, &actions, &history, &start);
        assert!(o1.contains("cost:"));
        assert!(!o1.contains("risk:"));

        // risk only
        let v2 = PlanVisualizer::new(VisualizationFormat::Text)
            .with_costs(false)
            .with_risks(true);
        let o2 = v2.visualize_plan(&plan, &actions, &history, &start);
        assert!(!o2.contains("cost:"));
        assert!(o2.contains("risk:"));
    }

    #[test]
    fn test_visualize_goal_hierarchy_fallthrough_formats() {
        // AsciiTimeline and Json should fall through to tree format
        let viz_timeline = PlanVisualizer::new(VisualizationFormat::AsciiTimeline);
        let viz_json = PlanVisualizer::new(VisualizationFormat::Json);
        let goal = Goal::new("test", BTreeMap::new()).with_priority(5.0);

        let out_tl = viz_timeline.visualize_goal_hierarchy(&goal);
        let out_json = viz_json.visualize_goal_hierarchy(&goal);
        // Both should produce tree format (same output as AsciiTree)
        assert!(out_tl.contains("[SEQ]"));
        assert!(out_json.contains("[SEQ]"));
    }

    #[test]
    fn test_dot_cost_risk_labels() {
        // With costs+risks, DOT labels should include cost/risk info
        let viz = PlanVisualizer::new(VisualizationFormat::Dot)
            .with_costs(true)
            .with_risks(true);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("act", 3.5)];
        let plan = vec!["act".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);
        assert!(output.contains("cost:"));
        assert!(output.contains("risk:"));

        // Without costs+risks, DOT labels should be just action name
        let viz2 = PlanVisualizer::new(VisualizationFormat::Dot)
            .with_costs(false)
            .with_risks(false);
        let output2 = viz2.visualize_plan(&plan, &actions, &history, &start);
        assert!(!output2.contains("cost:"));
    }

    #[test]
    fn test_timeline_time_accumulation() {
        // Kills: current_time += duration mutations
        let visualizer = PlanVisualizer::new(VisualizationFormat::AsciiTimeline);
        let mut hist = ActionHistory::new();
        hist.record_success("a", 2.0); // avg_duration = 2.0
        hist.record_success("b", 3.0); // avg_duration = 3.0

        let actions: Vec<Box<dyn Action>> =
            vec![create_test_action("a", 1.0), create_test_action("b", 1.0)];
        let plan = vec!["a".to_string(), "b".to_string()];
        let start = WorldState::new();

        let output = visualizer.visualize_plan(&plan, &actions, &hist, &start);
        // First action at time 0.0, second at time 2.0
        assert!(output.contains("0.0"), "First action should be at time 0.0");
        assert!(
            output.contains("2.0"),
            "Second action should be at time 2.0 (after duration 2.0)"
        );
    }

    // ── Round 2 mutation-killing tests ──

    #[test]
    fn test_goal_text_format_no_strategy_tags() {
        // Kills: delete Text match arm → falls through to render_goal_tree which has [SEQ]
        let viz = PlanVisualizer::new(VisualizationFormat::Text);
        let goal = Goal::new("my_goal", BTreeMap::new()).with_priority(5.0);
        let output = viz.visualize_goal_hierarchy(&goal);
        // Text format should NOT contain strategy tags like [SEQ], [PAR], etc.
        assert!(
            !output.contains("[SEQ]"),
            "Text format should not include [SEQ] tag"
        );
        assert!(
            !output.contains("[PAR]"),
            "Text format should not include [PAR] tag"
        );
        assert!(output.contains("my_goal"));
    }

    #[test]
    fn test_tree_render_single_action_uses_last_prefix() {
        // Kills: i == plan.len() - 1 → i != plan.len() - 1 (line 107)
        // With 1 action, i=0 IS the last, so prefix should be "└─" not "├─"
        let viz = PlanVisualizer::new(VisualizationFormat::AsciiTree);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("only", 1.0)];
        let plan = vec!["only".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);
        // Single action must use └─ (last prefix), not ├─
        assert!(output.contains("└─"), "Single action should use └─");
        assert!(!output.contains("├─"), "Single action should NOT use ├─");
    }

    #[test]
    fn test_tree_cost_only_exact_line_content() {
        // Kills: 1.0 - prob → 1.0 + prob or / in render_plan_tree (line 112)
        // IMPORTANT: Must check the ACTION line, not just any occurrence.
        // calculate_plan_metrics also computes 1-prob for the header, masking the mutation.
        let viz = PlanVisualizer::new(VisualizationFormat::AsciiTree)
            .with_costs(true)
            .with_risks(true);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("act", 3.0)];
        let plan = vec!["act".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);
        // Find the action-specific line (contains └─), not the header line
        let action_line = output
            .lines()
            .find(|l| l.contains("└─") || l.contains("├─"))
            .expect("Should have an action line with tree prefix");
        // risk should be 0.20 (1.0 - 0.8), not 1.80 (1.0+0.8) or 1.25 (1.0/0.8)
        assert!(
            action_line.contains("risk: 0.20"),
            "Action line risk should be 0.20 (1.0-0.8), got: {:?}",
            action_line
        );
    }

    #[test]
    fn test_timeline_low_success_icon() {
        // Kills: > 0.5 → >= 0.5 in render_plan_timeline (line 179)
        // Build exactly 50% success → > 0.5 is false, >= 0.5 would be true
        let viz = PlanVisualizer::new(VisualizationFormat::AsciiTimeline);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("act", 1.0)];
        let plan = vec!["act".to_string()];
        let mut hist = ActionHistory::new();
        hist.record_success("act", 1.0);
        hist.record_failure("act");
        // success_rate = 0.5, exactly at boundary
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &hist, &start);
        // 0.5 is NOT > 0.5, so it should show ✗ (failure icon), not ~
        assert!(
            output.contains("✗"),
            "Exactly 0.5 probability should show ✗"
        );
    }

    #[test]
    fn test_timeline_duration_accumulation_exact() {
        // Kills: current_time += duration → -= or *=
        let viz = PlanVisualizer::new(VisualizationFormat::AsciiTimeline);
        let mut hist = ActionHistory::new();
        hist.record_success("a", 5.0);
        hist.record_success("b", 10.0);

        let actions: Vec<Box<dyn Action>> =
            vec![create_test_action("a", 1.0), create_test_action("b", 1.0)];
        let plan = vec!["a".to_string(), "b".to_string()];
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &hist, &start);
        // a at time 0.0 (duration 5.0), b at time 5.0 (duration 10.0)
        // Find the data line for action "b" (skip header/separator)
        let b_line = output
            .lines()
            .find(|l| l.contains("| b"))
            .expect("Should have a line for action b");
        // With +=: time is 5.0 (positive). With -=: time is -5.0. With *=: time is 0.0.
        // The time column is left-justified with width 6, so "5.0" starts at position 0.
        let time_str = b_line.split('|').next().unwrap().trim();
        let time_val: f32 = time_str.parse().expect("Time should be a valid float");
        assert!(
            (time_val - 5.0).abs() < 0.01,
            "Second action should start at time 5.0, got {} from line: {:?}",
            time_val,
            b_line
        );
    }

    #[test]
    fn test_dot_edge_index_arithmetic() {
        // Kills: i - 1 → i + 1 or i / 1 in render_plan_dot (line 219)
        // Also kills: 1.0 - prob → 1.0 + prob or / in DOT risk calc
        // Default viz has show_costs=true, show_risks=true → node labels include risk
        let viz = PlanVisualizer::new(VisualizationFormat::Dot);
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("a", 1.0),
            create_test_action("b", 1.0),
            create_test_action("c", 1.0),
        ];
        let plan = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);
        assert!(output.contains("start -> action_0"));
        assert!(output.contains("action_0 -> action_1"));
        assert!(output.contains("action_1 -> action_2"));
        assert!(output.contains("action_2 -> end"));
        assert!(
            !output.contains("action_3"),
            "Should not reference action_3 with only 3 actions"
        );
        // Risk value in DOT node labels: default prob=0.8, risk=1.0-0.8=0.20
        // With +: risk=1.80. With /: risk=1.25. Either would fail this assertion.
        assert!(
            output.contains("risk: 0.20"),
            "DOT node labels should contain risk: 0.20, got: {:?}",
            output
        );
    }

    #[test]
    fn test_dot_cost_risk_conditional_label() {
        // Kills: && → || in render_plan_dot (line 221)
        // With costs=true, risks=false: should NOT show cost/risk combined label
        let viz = PlanVisualizer::new(VisualizationFormat::Dot)
            .with_costs(true)
            .with_risks(false);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("act", 2.0)];
        let plan = vec!["act".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);
        // && requires BOTH true. With costs=true, risks=false, && is false → uses plain name
        // || would make it true → would show cost/risk labels
        assert!(
            !output.contains("cost:"),
            "DOT with costs=true but risks=false should not show combined label"
        );
    }

    #[test]
    fn test_text_index_arithmetic() {
        // Kills: i + 1 → i - 1 or i / 1 in render_plan_text (line 264) and
        // && → || in render_plan_text (line 270)
        let viz = PlanVisualizer::new(VisualizationFormat::Text)
            .with_costs(true)
            .with_risks(true);
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("alpha", 1.0),
            create_test_action("beta", 2.0),
            create_test_action("gamma", 3.0),
        ];
        let plan = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);
        // Check 1-based numbering: "1. alpha", "2. beta", "3. gamma"
        assert!(output.contains("1. alpha"));
        assert!(output.contains("2. beta"));
        assert!(output.contains("3. gamma"));
        // With i-1: would be "0. alpha", "-1. beta" etc.
        assert!(
            !output.contains("0. alpha"),
            "Should not use 0-based indexing"
        );
    }

    #[test]
    fn test_text_cost_and_risk_both_shown_with_comma() {
        // Kills: && → || in show_costs && show_risks comma condition (line 270)
        // Also kills: 1.0 - prob → 1.0 + prob or / in render_plan_text (line 264)
        let viz = PlanVisualizer::new(VisualizationFormat::Text)
            .with_costs(true)
            .with_risks(true);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("act", 2.0)];
        let plan = vec!["act".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);
        // Both cost and risk shown, with comma separator
        assert!(output.contains("cost:"));
        assert!(output.contains("risk:"));
        assert!(
            output.contains(", "),
            "Cost and risk should be separated by comma"
        );
        // Text format has no header with separate metrics — risk value is per-action only
        // risk should be 0.20 (1.0-0.8), not 1.80 or 1.25
        assert!(
            output.contains("risk: 0.20"),
            "Text risk should be 0.20 (1.0-0.8), got: {:?}",
            output
        );
    }

    #[test]
    fn test_text_cost_only_no_comma() {
        // Kills: && → || in show_costs && show_risks comma condition
        // With costs=true, risks=false: && is false (no comma), || is true (comma inserted)
        let viz = PlanVisualizer::new(VisualizationFormat::Text)
            .with_costs(true)
            .with_risks(false);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("x", 2.0)];
        let plan = vec!["x".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);
        let action_line = output.lines().find(|l| l.contains("1. x")).unwrap();
        assert!(action_line.contains("cost:"));
        assert!(!action_line.contains("risk:"));
        // With || mutation: comma is inserted before risk (which isn't shown)
        // producing "(cost: 2.0, )" — check no trailing comma before closing paren
        assert!(
            !action_line.contains(", )"),
            "Should not have trailing comma when only costs shown, got: {:?}",
            action_line
        );
    }

    #[test]
    fn test_json_comma_and_index_exact() {
        // Kills: i < plan.len()-1 mutations, and 1.0 - prob → 1.0 + prob or / in JSON
        let viz = PlanVisualizer::new(VisualizationFormat::Json);
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("first", 1.0),
            create_test_action("second", 2.0),
            create_test_action("third", 3.0),
        ];
        let plan = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);

        // First two action blocks end with "}," and last ends with "}"
        let commas = output.matches("},").count();
        assert!(
            commas >= 2,
            "First two actions should have trailing commas, found {}",
            commas
        );
        // JSON risk values: default prob=0.8, risk=1.0-0.8=0.20
        // With +: risk=1.80. With /: risk=1.25. Either would fail.
        assert!(
            output.contains("\"risk\": 0.20"),
            "JSON risk should be 0.20 (1.0-0.8), got: {:?}",
            output
        );
    }

    #[test]
    fn test_json_index_subtraction() {
        // Kills: plan.len() - 1 → plan.len() + 1 in JSON comma logic (line 299, 303)
        // With 1 action: plan.len()-1 = 0, i=0, 0 < 0 = false, no comma
        // With +1: plan.len()+1=2, 0 < 2 = true, would add comma
        let viz = PlanVisualizer::new(VisualizationFormat::Json);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("only", 1.0)];
        let plan = vec!["only".to_string()];
        let history = ActionHistory::new();
        let start = WorldState::new();

        let output = viz.visualize_plan(&plan, &actions, &history, &start);
        // Single action should NOT have trailing comma on its closing brace
        // Look for "}," which should not appear
        assert!(
            !output.contains("},"),
            "Single action JSON should not have trailing comma"
        );
    }

    #[test]
    fn test_goal_tree_depth_subtraction() {
        // Kills: depth - 1 → depth + 1 or depth / 1 in render_goal_tree (line 330)
        let viz = PlanVisualizer::new(VisualizationFormat::AsciiTree);
        let grandchild = Goal::new("grandchild", BTreeMap::new());
        let child = Goal::new("child", BTreeMap::new()).with_sub_goals(vec![grandchild]);
        let parent = Goal::new("parent", BTreeMap::new()).with_sub_goals(vec![child]);

        let output = viz.visualize_goal_hierarchy(&parent);
        // depth=0: no indent (empty string)
        // depth=1: "  ".repeat(1-1) + "  └─ " = "" + "  └─ " = "  └─ "
        // depth=2: "  ".repeat(2-1) + "  └─ " = "  " + "  └─ " = "    └─ "
        // With +1: depth=1 → "  ".repeat(2) + "  └─ " = "      └─ " (6 spaces)
        // With /1: depth=1 → "  ".repeat(1) + "  └─ " = "    └─ " (4 spaces)
        let lines: Vec<&str> = output.lines().collect();
        // Parent at depth 0: no indent
        assert!(
            lines[0].starts_with("["),
            "Parent line should start with strategy tag"
        );
        // Child at depth 1: exactly "  └─ " prefix (2 spaces before └)
        let child_line = lines
            .iter()
            .find(|l| l.contains("child") && !l.contains("grandchild"))
            .unwrap();
        assert!(
            child_line.starts_with("  └─") || child_line.starts_with("  ├─"),
            "Child at depth 1 should start with exactly 2 spaces before tree char. Got: {:?}",
            child_line
        );
        // Verify grandchild at depth 2: exactly "    └─ " prefix (4 spaces before └)
        let gc_line = lines.iter().find(|l| l.contains("grandchild")).unwrap();
        assert!(
            gc_line.starts_with("    └─") || gc_line.starts_with("    ├─"),
            "Grandchild at depth 2 should start with exactly 4 spaces before tree char. Got: {:?}",
            gc_line
        );
    }

    #[test]
    fn test_calculate_plan_metrics_unknown_action() {
        // Kills: == → != in calculate_plan_metrics (line 438)
        // When action is found (name matches), cost & risk are added.
        // With !=, it would add metrics for NON-matching actions instead.
        let viz = PlanVisualizer::new(VisualizationFormat::AsciiTree);
        let actions: Vec<Box<dyn Action>> = vec![create_test_action("known", 5.0)];
        let history = ActionHistory::new();

        // Plan with the known action
        let plan_known = vec!["known".to_string()];
        let (cost_k, _) = viz.calculate_plan_metrics(&plan_known, &actions, &history);
        assert_eq!(cost_k, 5.0);

        // Plan with an unknown action — should contribute 0 cost
        let plan_unknown = vec!["phantom".to_string()];
        let (cost_u, risk_u) = viz.calculate_plan_metrics(&plan_unknown, &actions, &history);
        assert_eq!(cost_u, 0.0);
        assert_eq!(risk_u, 0.0);
    }
}
