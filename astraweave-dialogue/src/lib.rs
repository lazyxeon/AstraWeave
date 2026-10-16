#![forbid(unsafe_code)]
//! Dialogue system: branching dialogue graphs, validation, and execution
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod llm_dialogue;
pub mod runner;
pub mod toml_loader;

/// A node in a dialogue graph representing a single piece of dialogue.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DialogueNode {
    /// Unique identifier for this node
    pub id: String,
    /// The dialogue text displayed to the player
    pub text: String,
    /// Available responses/choices from this node
    pub responses: Vec<DialogueResponse>,
}

impl DialogueNode {
    /// Creates a new dialogue node with the given id and text.
    #[must_use]
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            responses: Vec::new(),
        }
    }

    /// Creates a dialogue node with responses.
    #[must_use]
    pub fn with_responses(mut self, responses: Vec<DialogueResponse>) -> Self {
        self.responses = responses;
        self
    }

    /// Adds a single response to this node.
    #[must_use]
    pub fn with_response(mut self, response: DialogueResponse) -> Self {
        self.responses.push(response);
        self
    }

    /// Returns true if this node has any responses.
    #[must_use]
    pub fn has_responses(&self) -> bool {
        !self.responses.is_empty()
    }

    /// Returns the number of responses available from this node.
    #[must_use]
    pub fn response_count(&self) -> usize {
        self.responses.len()
    }

    /// Returns true if this is a terminal node (no responses).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.responses.is_empty()
    }

    /// Returns true if this is a choice node (multiple responses).
    #[must_use]
    pub fn is_choice(&self) -> bool {
        self.responses.len() > 1
    }

    /// Returns true if this is a linear node (exactly one response).
    #[must_use]
    pub fn is_linear(&self) -> bool {
        self.responses.len() == 1
    }

    /// Returns the first response if available.
    #[must_use]
    pub fn first_response(&self) -> Option<&DialogueResponse> {
        self.responses.first()
    }

    /// Returns the last response if available.
    #[must_use]
    pub fn last_response(&self) -> Option<&DialogueResponse> {
        self.responses.last()
    }

    /// Returns a response by index.
    #[must_use]
    pub fn get_response(&self, index: usize) -> Option<&DialogueResponse> {
        self.responses.get(index)
    }

    /// Returns the text truncated to the given length.
    #[must_use]
    pub fn truncated_text(&self, max_len: usize) -> String {
        if self.text.len() <= max_len {
            self.text.clone()
        } else {
            format!("{}...", &self.text[..max_len.saturating_sub(3)])
        }
    }

    /// Returns a brief summary of this node.
    #[must_use]
    pub fn summary(&self) -> String {
        let text_preview = self.truncated_text(30);
        if self.is_terminal() {
            format!("[{}] \"{}\" (end)", self.id, text_preview)
        } else if self.is_choice() {
            format!(
                "[{}] \"{}\" ({} choices)",
                self.id,
                text_preview,
                self.response_count()
            )
        } else {
            format!("[{}] \"{}\" (continue)", self.id, text_preview)
        }
    }

    /// Returns true if the node's id matches the given string.
    #[must_use]
    pub fn has_id(&self, id: &str) -> bool {
        self.id == id
    }

    /// Returns true if the node's text contains the given substring (case-insensitive).
    #[must_use]
    pub fn text_contains(&self, substr: &str) -> bool {
        self.text.to_lowercase().contains(&substr.to_lowercase())
    }

    /// Returns all next node IDs that responses in this node can lead to.
    #[must_use]
    pub fn next_node_ids(&self) -> Vec<&str> {
        self.responses
            .iter()
            .filter_map(|r| r.next_id.as_deref())
            .collect()
    }

    /// Returns true if any response leads to the given node ID.
    #[must_use]
    pub fn leads_to(&self, node_id: &str) -> bool {
        self.responses.iter().any(|r| r.has_next_id(node_id))
    }
}

impl fmt::Display for DialogueNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_terminal() {
            write!(
                f,
                "DialogueNode[{}]: \"{}\" (terminal)",
                self.id,
                self.truncated_text(40)
            )
        } else {
            write!(
                f,
                "DialogueNode[{}]: \"{}\" ({} responses)",
                self.id,
                self.truncated_text(40),
                self.response_count()
            )
        }
    }
}

/// A response option in a dialogue node.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DialogueResponse {
    /// The text of the response/choice
    pub text: String,
    /// The ID of the next node, or None if this ends the dialogue
    pub next_id: Option<String>,
}

impl DialogueResponse {
    /// Creates a new dialogue response with the given text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            next_id: None,
        }
    }

    /// Creates a response that leads to another node.
    #[must_use]
    pub fn with_next(text: impl Into<String>, next_id: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            next_id: Some(next_id.into()),
        }
    }

    /// Sets the next node ID.
    #[must_use]
    pub fn next(mut self, next_id: impl Into<String>) -> Self {
        self.next_id = Some(next_id.into());
        self
    }

    /// Returns true if this response leads to another node.
    #[must_use]
    pub fn has_next(&self) -> bool {
        self.next_id.is_some()
    }

    /// Returns true if this response ends the dialogue.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.next_id.is_none()
    }

    /// Returns true if this response leads to the specified node.
    #[must_use]
    pub fn has_next_id(&self, id: &str) -> bool {
        self.next_id.as_deref() == Some(id)
    }

    /// Returns the next node ID if present.
    #[must_use]
    pub fn next_node_id(&self) -> Option<&str> {
        self.next_id.as_deref()
    }

    /// Returns the text truncated to the given length.
    #[must_use]
    pub fn truncated_text(&self, max_len: usize) -> String {
        if self.text.len() <= max_len {
            self.text.clone()
        } else {
            format!("{}...", &self.text[..max_len.saturating_sub(3)])
        }
    }

    /// Returns a brief summary of this response.
    #[must_use]
    pub fn summary(&self) -> String {
        let text_preview = self.truncated_text(30);
        match &self.next_id {
            Some(next) => format!("\"{}\" -> {}", text_preview, next),
            None => format!("\"{}\" (end)", text_preview),
        }
    }

    /// Returns true if the response text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Returns the length of the response text.
    #[must_use]
    pub fn text_len(&self) -> usize {
        self.text.len()
    }

    /// Returns true if the text contains the given substring (case-insensitive).
    #[must_use]
    pub fn text_contains(&self, substr: &str) -> bool {
        self.text.to_lowercase().contains(&substr.to_lowercase())
    }
}

impl fmt::Display for DialogueResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.next_id {
            Some(next) => write!(f, "Response: \"{}\" -> {}", self.truncated_text(40), next),
            None => write!(f, "Response: \"{}\" (end)", self.truncated_text(40)),
        }
    }
}

/// A graph structure representing a complete dialogue tree.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DialogueGraph {
    /// All nodes in this dialogue graph
    pub nodes: Vec<DialogueNode>,
}

impl DialogueGraph {
    /// Creates a new empty dialogue graph.
    #[must_use]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Creates a dialogue graph with the given nodes.
    #[must_use]
    pub fn with_nodes(nodes: Vec<DialogueNode>) -> Self {
        Self { nodes }
    }

    /// Adds a node to the graph.
    pub fn add_node(&mut self, node: DialogueNode) {
        self.nodes.push(node);
    }

    /// Builder method to add a node.
    #[must_use]
    pub fn with_node(mut self, node: DialogueNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Validates the dialogue graph for structural integrity.
    pub fn validate(&self) -> Result<(), String> {
        let ids: std::collections::HashSet<_> = self.nodes.iter().map(|n| &n.id).collect();
        for node in &self.nodes {
            for resp in &node.responses {
                if let Some(ref next) = resp.next_id {
                    if !ids.contains(next) {
                        return Err(format!("Invalid next_id: {}", next));
                    }
                }
            }
        }
        Ok(())
    }

    /// Gets a node by its ID.
    #[must_use]
    pub fn get_node(&self, id: &str) -> Option<&DialogueNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Gets a mutable reference to a node by its ID.
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut DialogueNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Returns the number of nodes in the graph.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the graph has no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns true if a node with the given ID exists.
    #[must_use]
    pub fn has_node(&self, id: &str) -> bool {
        self.nodes.iter().any(|n| n.id == id)
    }

    /// Returns all terminal nodes (nodes with no responses).
    #[must_use]
    pub fn terminal_nodes(&self) -> Vec<&DialogueNode> {
        self.nodes.iter().filter(|n| n.is_terminal()).collect()
    }

    /// Returns the count of terminal nodes.
    #[must_use]
    pub fn terminal_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_terminal()).count()
    }

    /// Returns all choice nodes (nodes with multiple responses).
    #[must_use]
    pub fn choice_nodes(&self) -> Vec<&DialogueNode> {
        self.nodes.iter().filter(|n| n.is_choice()).collect()
    }

    /// Returns the count of choice nodes.
    #[must_use]
    pub fn choice_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.is_choice()).count()
    }

    /// Returns all linear nodes (nodes with exactly one response).
    #[must_use]
    pub fn linear_nodes(&self) -> Vec<&DialogueNode> {
        self.nodes.iter().filter(|n| n.is_linear()).collect()
    }

    /// Returns the total number of responses across all nodes.
    #[must_use]
    pub fn total_response_count(&self) -> usize {
        self.nodes.iter().map(|n| n.response_count()).sum()
    }

    /// Returns the first node in the graph.
    #[must_use]
    pub fn first_node(&self) -> Option<&DialogueNode> {
        self.nodes.first()
    }

    /// Returns nodes that are not referenced by any response.
    #[must_use]
    pub fn root_nodes(&self) -> Vec<&DialogueNode> {
        let referenced: std::collections::HashSet<&str> = self
            .nodes
            .iter()
            .flat_map(|n| n.responses.iter().filter_map(|r| r.next_id.as_deref()))
            .collect();
        self.nodes
            .iter()
            .filter(|n| !referenced.contains(n.id.as_str()))
            .collect()
    }

    /// Returns all unique node IDs.
    #[must_use]
    pub fn node_ids(&self) -> Vec<&str> {
        self.nodes.iter().map(|n| n.id.as_str()).collect()
    }

    /// Finds nodes whose text contains the given substring (case-insensitive).
    #[must_use]
    pub fn find_nodes_by_text(&self, substr: &str) -> Vec<&DialogueNode> {
        self.nodes
            .iter()
            .filter(|n| n.text_contains(substr))
            .collect()
    }

    /// Returns true if the graph is valid (no broken references).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Returns validation errors if any.
    #[must_use]
    pub fn validation_errors(&self) -> Vec<String> {
        let ids: std::collections::HashSet<_> = self.nodes.iter().map(|n| &n.id).collect();
        let mut errors = Vec::new();
        for node in &self.nodes {
            for resp in &node.responses {
                if let Some(ref next) = resp.next_id {
                    if !ids.contains(next) {
                        errors.push(format!("Node '{}' has invalid next_id: {}", node.id, next));
                    }
                }
            }
        }
        errors
    }

    /// Returns a brief summary of the graph.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "DialogueGraph: {} nodes, {} responses, {} terminals, {} choices",
            self.node_count(),
            self.total_response_count(),
            self.terminal_count(),
            self.choice_count()
        )
    }

    /// Returns the maximum depth of the dialogue tree (approximate).
    #[must_use]
    pub fn max_depth(&self) -> usize {
        if self.is_empty() {
            return 0;
        }

        let mut max = 0;
        let mut visited = std::collections::HashSet::new();

        for node in &self.nodes {
            let depth = self.calculate_depth(&node.id, &mut visited);
            max = max.max(depth);
            visited.clear();
        }
        max
    }

    fn calculate_depth(
        &self,
        node_id: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> usize {
        if visited.contains(node_id) {
            return 0; // Cycle detected
        }
        visited.insert(node_id.to_string());

        if let Some(node) = self.get_node(node_id) {
            if node.is_terminal() {
                return 1;
            }
            let max_child = node
                .responses
                .iter()
                .filter_map(|r| r.next_id.as_ref())
                .map(|next| self.calculate_depth(next, visited))
                .max()
                .unwrap_or(0);
            1 + max_child
        } else {
            0
        }
    }
}

impl fmt::Display for DialogueGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== DialogueNode Tests ====================

    #[test]
    fn test_dialogue_node_default() {
        let node = DialogueNode::default();
        assert!(node.id.is_empty());
        assert!(node.text.is_empty());
        assert!(node.responses.is_empty());
    }

    #[test]
    fn test_dialogue_node_new() {
        let node = DialogueNode::new("test_id", "Hello there!");
        assert_eq!(node.id, "test_id");
        assert_eq!(node.text, "Hello there!");
        assert!(node.responses.is_empty());
    }

    #[test]
    fn test_dialogue_node_with_responses() {
        let responses = vec![
            DialogueResponse::new("Option 1"),
            DialogueResponse::new("Option 2"),
        ];
        let node = DialogueNode::new("test", "Choose").with_responses(responses);
        assert_eq!(node.response_count(), 2);
    }

    #[test]
    fn test_dialogue_node_with_response() {
        let node = DialogueNode::new("test", "Hello")
            .with_response(DialogueResponse::new("Response 1"))
            .with_response(DialogueResponse::new("Response 2"));
        assert_eq!(node.response_count(), 2);
    }

    #[test]
    fn test_dialogue_node_has_responses() {
        let empty = DialogueNode::new("test", "Hello");
        assert!(!empty.has_responses());

        let with_response = empty.with_response(DialogueResponse::new("Yes"));
        assert!(with_response.has_responses());
    }

    #[test]
    fn test_dialogue_node_response_count() {
        let node = DialogueNode::new("test", "Choose")
            .with_response(DialogueResponse::new("A"))
            .with_response(DialogueResponse::new("B"))
            .with_response(DialogueResponse::new("C"));
        assert_eq!(node.response_count(), 3);
    }

    #[test]
    fn test_dialogue_node_is_terminal() {
        let terminal = DialogueNode::new("end", "Goodbye");
        assert!(terminal.is_terminal());

        let non_terminal = terminal.with_response(DialogueResponse::new("Wait"));
        assert!(!non_terminal.is_terminal());
    }

    #[test]
    fn test_dialogue_node_is_choice() {
        let single =
            DialogueNode::new("test", "Hello").with_response(DialogueResponse::new("Continue"));
        assert!(!single.is_choice());

        let choice = DialogueNode::new("test", "Choose")
            .with_response(DialogueResponse::new("A"))
            .with_response(DialogueResponse::new("B"));
        assert!(choice.is_choice());
    }

    #[test]
    fn test_dialogue_node_is_linear() {
        let linear =
            DialogueNode::new("test", "Hello").with_response(DialogueResponse::new("Continue"));
        assert!(linear.is_linear());

        let empty = DialogueNode::new("test", "End");
        assert!(!empty.is_linear());

        let choice = DialogueNode::new("test", "Choose")
            .with_response(DialogueResponse::new("A"))
            .with_response(DialogueResponse::new("B"));
        assert!(!choice.is_linear());
    }

    #[test]
    fn test_dialogue_node_first_response() {
        let node = DialogueNode::new("test", "Hello")
            .with_response(DialogueResponse::new("First"))
            .with_response(DialogueResponse::new("Second"));
        assert_eq!(node.first_response().unwrap().text, "First");

        let empty = DialogueNode::new("test", "Hello");
        assert!(empty.first_response().is_none());
    }

    #[test]
    fn test_dialogue_node_last_response() {
        let node = DialogueNode::new("test", "Hello")
            .with_response(DialogueResponse::new("First"))
            .with_response(DialogueResponse::new("Last"));
        assert_eq!(node.last_response().unwrap().text, "Last");
    }

    #[test]
    fn test_dialogue_node_get_response() {
        let node = DialogueNode::new("test", "Choose")
            .with_response(DialogueResponse::new("Zero"))
            .with_response(DialogueResponse::new("One"))
            .with_response(DialogueResponse::new("Two"));
        assert_eq!(node.get_response(0).unwrap().text, "Zero");
        assert_eq!(node.get_response(1).unwrap().text, "One");
        assert_eq!(node.get_response(2).unwrap().text, "Two");
        assert!(node.get_response(3).is_none());
    }

    #[test]
    fn test_dialogue_node_truncated_text() {
        let node = DialogueNode::new("test", "This is a very long dialogue text");
        assert_eq!(node.truncated_text(10), "This is...");
        assert_eq!(
            node.truncated_text(100),
            "This is a very long dialogue text"
        );
    }

    #[test]
    fn test_dialogue_node_summary_terminal() {
        let node = DialogueNode::new("end", "Goodbye!");
        let summary = node.summary();
        assert!(summary.contains("[end]"));
        assert!(summary.contains("(end)"));
    }

    #[test]
    fn test_dialogue_node_summary_choice() {
        let node = DialogueNode::new("choice", "Choose your path")
            .with_response(DialogueResponse::new("A"))
            .with_response(DialogueResponse::new("B"))
            .with_response(DialogueResponse::new("C"));
        let summary = node.summary();
        assert!(summary.contains("3 choices"));
    }

    #[test]
    fn test_dialogue_node_summary_linear() {
        let node =
            DialogueNode::new("linear", "Hello").with_response(DialogueResponse::new("Continue"));
        let summary = node.summary();
        assert!(summary.contains("(continue)"));
    }

    #[test]
    fn test_dialogue_node_has_id() {
        let node = DialogueNode::new("my_id", "Text");
        assert!(node.has_id("my_id"));
        assert!(!node.has_id("other_id"));
    }

    #[test]
    fn test_dialogue_node_text_contains() {
        let node = DialogueNode::new("test", "Hello World, how are you?");
        assert!(node.text_contains("world"));
        assert!(node.text_contains("HELLO"));
        assert!(!node.text_contains("goodbye"));
    }

    #[test]
    fn test_dialogue_node_next_node_ids() {
        let node = DialogueNode::new("test", "Choose")
            .with_response(DialogueResponse::with_next("A", "path_a"))
            .with_response(DialogueResponse::with_next("B", "path_b"))
            .with_response(DialogueResponse::new("End"));
        let next_ids = node.next_node_ids();
        assert_eq!(next_ids.len(), 2);
        assert!(next_ids.contains(&"path_a"));
        assert!(next_ids.contains(&"path_b"));
    }

    #[test]
    fn test_dialogue_node_leads_to() {
        let node = DialogueNode::new("test", "Hello")
            .with_response(DialogueResponse::with_next("Continue", "next_node"));
        assert!(node.leads_to("next_node"));
        assert!(!node.leads_to("other_node"));
    }

    #[test]
    fn test_dialogue_node_display() {
        let terminal = DialogueNode::new("end", "Goodbye!");
        let display = format!("{}", terminal);
        assert!(display.contains("DialogueNode"));
        assert!(display.contains("terminal"));

        let with_responses =
            DialogueNode::new("start", "Hello").with_response(DialogueResponse::new("Hi"));
        let display = format!("{}", with_responses);
        assert!(display.contains("1 responses"));
    }

    // ==================== DialogueResponse Tests ====================

    #[test]
    fn test_dialogue_response_default() {
        let resp = DialogueResponse::default();
        assert!(resp.text.is_empty());
        assert!(resp.next_id.is_none());
    }

    #[test]
    fn test_dialogue_response_new() {
        let resp = DialogueResponse::new("Click me");
        assert_eq!(resp.text, "Click me");
        assert!(resp.next_id.is_none());
    }

    #[test]
    fn test_dialogue_response_with_next() {
        let resp = DialogueResponse::with_next("Continue", "next_node");
        assert_eq!(resp.text, "Continue");
        assert_eq!(resp.next_id, Some("next_node".to_string()));
    }

    #[test]
    fn test_dialogue_response_next_builder() {
        let resp = DialogueResponse::new("Go").next("destination");
        assert_eq!(resp.next_id, Some("destination".to_string()));
    }

    #[test]
    fn test_dialogue_response_has_next() {
        let with_next = DialogueResponse::with_next("Go", "target");
        assert!(with_next.has_next());

        let without_next = DialogueResponse::new("End");
        assert!(!without_next.has_next());
    }

    #[test]
    fn test_dialogue_response_is_terminal() {
        let terminal = DialogueResponse::new("Goodbye");
        assert!(terminal.is_terminal());

        let continuing = DialogueResponse::with_next("Continue", "next");
        assert!(!continuing.is_terminal());
    }

    #[test]
    fn test_dialogue_response_has_next_id() {
        let resp = DialogueResponse::with_next("Go", "target_node");
        assert!(resp.has_next_id("target_node"));
        assert!(!resp.has_next_id("other_node"));
    }

    #[test]
    fn test_dialogue_response_next_node_id() {
        let resp = DialogueResponse::with_next("Go", "target");
        assert_eq!(resp.next_node_id(), Some("target"));

        let terminal = DialogueResponse::new("End");
        assert!(terminal.next_node_id().is_none());
    }

    #[test]
    fn test_dialogue_response_truncated_text() {
        let resp = DialogueResponse::new("This is a very long response text");
        assert_eq!(resp.truncated_text(10), "This is...");
        assert_eq!(
            resp.truncated_text(100),
            "This is a very long response text"
        );
    }

    #[test]
    fn test_dialogue_response_summary() {
        let continuing = DialogueResponse::with_next("Continue the story", "next");
        let summary = continuing.summary();
        assert!(summary.contains("-> next"));

        let terminal = DialogueResponse::new("The End");
        let summary = terminal.summary();
        assert!(summary.contains("(end)"));
    }

    #[test]
    fn test_dialogue_response_is_empty() {
        let empty = DialogueResponse::new("");
        assert!(empty.is_empty());

        let with_text = DialogueResponse::new("Hello");
        assert!(!with_text.is_empty());
    }

    #[test]
    fn test_dialogue_response_text_len() {
        let resp = DialogueResponse::new("Hello");
        assert_eq!(resp.text_len(), 5);
    }

    #[test]
    fn test_dialogue_response_text_contains() {
        let resp = DialogueResponse::new("Accept the quest");
        assert!(resp.text_contains("quest"));
        assert!(resp.text_contains("ACCEPT"));
        assert!(!resp.text_contains("decline"));
    }

    #[test]
    fn test_dialogue_response_display() {
        let continuing = DialogueResponse::with_next("Go forward", "next");
        let display = format!("{}", continuing);
        assert!(display.contains("Response"));
        assert!(display.contains("-> next"));

        let terminal = DialogueResponse::new("Goodbye");
        let display = format!("{}", terminal);
        assert!(display.contains("(end)"));
    }

    // ==================== DialogueGraph Tests ====================

    #[test]
    fn test_dialogue_graph_default() {
        let graph = DialogueGraph::default();
        assert!(graph.nodes.is_empty());
    }

    #[test]
    fn test_dialogue_graph_new() {
        let graph = DialogueGraph::new();
        assert!(graph.is_empty());
    }

    #[test]
    fn test_dialogue_graph_with_nodes() {
        let nodes = vec![
            DialogueNode::new("1", "First"),
            DialogueNode::new("2", "Second"),
        ];
        let graph = DialogueGraph::with_nodes(nodes);
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_dialogue_graph_add_node() {
        let mut graph = DialogueGraph::new();
        graph.add_node(DialogueNode::new("1", "First"));
        graph.add_node(DialogueNode::new("2", "Second"));
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_dialogue_graph_with_node_builder() {
        let graph = DialogueGraph::new()
            .with_node(DialogueNode::new("1", "First"))
            .with_node(DialogueNode::new("2", "Second"));
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_validate_empty_graph() {
        let graph = DialogueGraph::default();
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_validate_single_node_no_responses() {
        let graph = DialogueGraph {
            nodes: vec![DialogueNode {
                id: "start".into(),
                text: "Hello!".into(),
                responses: vec![],
            }],
        };
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_validate_valid_chain() {
        let graph = DialogueGraph {
            nodes: vec![
                DialogueNode {
                    id: "start".into(),
                    text: "Hello!".into(),
                    responses: vec![DialogueResponse {
                        text: "Continue".into(),
                        next_id: Some("middle".into()),
                    }],
                },
                DialogueNode {
                    id: "middle".into(),
                    text: "How are you?".into(),
                    responses: vec![DialogueResponse {
                        text: "End".into(),
                        next_id: Some("end".into()),
                    }],
                },
                DialogueNode {
                    id: "end".into(),
                    text: "Goodbye!".into(),
                    responses: vec![],
                },
            ],
        };
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_next_id() {
        let graph = DialogueGraph {
            nodes: vec![DialogueNode {
                id: "start".into(),
                text: "Hello!".into(),
                responses: vec![DialogueResponse {
                    text: "Bye".into(),
                    next_id: Some("nonexistent".into()),
                }],
            }],
        };
        let result = graph.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonexistent"));
    }

    #[test]
    fn test_validate_none_next_id_is_valid() {
        let graph = DialogueGraph {
            nodes: vec![DialogueNode {
                id: "start".into(),
                text: "Hello!".into(),
                responses: vec![DialogueResponse {
                    text: "Bye".into(),
                    next_id: None,
                }],
            }],
        };
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_validate_multiple_responses() {
        let graph = DialogueGraph {
            nodes: vec![
                DialogueNode {
                    id: "start".into(),
                    text: "Choose your path".into(),
                    responses: vec![
                        DialogueResponse {
                            text: "Option A".into(),
                            next_id: Some("path_a".into()),
                        },
                        DialogueResponse {
                            text: "Option B".into(),
                            next_id: Some("path_b".into()),
                        },
                        DialogueResponse {
                            text: "End now".into(),
                            next_id: None,
                        },
                    ],
                },
                DialogueNode {
                    id: "path_a".into(),
                    text: "You chose A".into(),
                    responses: vec![],
                },
                DialogueNode {
                    id: "path_b".into(),
                    text: "You chose B".into(),
                    responses: vec![],
                },
            ],
        };
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_get_node_found() {
        let graph = DialogueGraph {
            nodes: vec![
                DialogueNode {
                    id: "node1".into(),
                    text: "First".into(),
                    responses: vec![],
                },
                DialogueNode {
                    id: "node2".into(),
                    text: "Second".into(),
                    responses: vec![],
                },
            ],
        };
        let node = graph.get_node("node2");
        assert!(node.is_some());
        assert_eq!(node.unwrap().text, "Second");
    }

    #[test]
    fn test_get_node_not_found() {
        let graph = DialogueGraph {
            nodes: vec![DialogueNode {
                id: "node1".into(),
                text: "First".into(),
                responses: vec![],
            }],
        };
        assert!(graph.get_node("nonexistent").is_none());
    }

    #[test]
    fn test_get_node_empty_graph() {
        let graph = DialogueGraph::default();
        assert!(graph.get_node("any").is_none());
    }

    #[test]
    fn test_get_node_mut() {
        let mut graph = DialogueGraph::new().with_node(DialogueNode::new("test", "Original"));

        if let Some(node) = graph.get_node_mut("test") {
            node.text = "Modified".to_string();
        }

        assert_eq!(graph.get_node("test").unwrap().text, "Modified");
    }

    #[test]
    fn test_dialogue_graph_node_count() {
        let graph = DialogueGraph::new()
            .with_node(DialogueNode::new("1", "A"))
            .with_node(DialogueNode::new("2", "B"))
            .with_node(DialogueNode::new("3", "C"));
        assert_eq!(graph.node_count(), 3);
    }

    #[test]
    fn test_dialogue_graph_is_empty() {
        let empty = DialogueGraph::new();
        assert!(empty.is_empty());

        let non_empty = empty.with_node(DialogueNode::new("1", "A"));
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_dialogue_graph_has_node() {
        let graph = DialogueGraph::new().with_node(DialogueNode::new("exists", "Hello"));
        assert!(graph.has_node("exists"));
        assert!(!graph.has_node("not_exists"));
    }

    #[test]
    fn test_dialogue_graph_terminal_nodes() {
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("start", "Hello")
                    .with_response(DialogueResponse::with_next("Continue", "end")),
            )
            .with_node(DialogueNode::new("end", "Goodbye"));

        let terminals = graph.terminal_nodes();
        assert_eq!(terminals.len(), 1);
        assert_eq!(terminals[0].id, "end");
    }

    #[test]
    fn test_dialogue_graph_terminal_count() {
        let graph = DialogueGraph::new()
            .with_node(DialogueNode::new("1", "A"))
            .with_node(DialogueNode::new("2", "B"))
            .with_node(
                DialogueNode::new("3", "C").with_response(DialogueResponse::new("Continue")),
            );
        assert_eq!(graph.terminal_count(), 2);
    }

    #[test]
    fn test_dialogue_graph_choice_nodes() {
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("choice", "Choose")
                    .with_response(DialogueResponse::new("A"))
                    .with_response(DialogueResponse::new("B")),
            )
            .with_node(
                DialogueNode::new("linear", "Hello")
                    .with_response(DialogueResponse::new("Continue")),
            );

        let choices = graph.choice_nodes();
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].id, "choice");
    }

    #[test]
    fn test_dialogue_graph_choice_count() {
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("c1", "Choice 1")
                    .with_response(DialogueResponse::new("A"))
                    .with_response(DialogueResponse::new("B")),
            )
            .with_node(
                DialogueNode::new("c2", "Choice 2")
                    .with_response(DialogueResponse::new("X"))
                    .with_response(DialogueResponse::new("Y"))
                    .with_response(DialogueResponse::new("Z")),
            );
        assert_eq!(graph.choice_count(), 2);
    }

    #[test]
    fn test_dialogue_graph_linear_nodes() {
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("linear", "Hello")
                    .with_response(DialogueResponse::new("Continue")),
            )
            .with_node(DialogueNode::new("terminal", "End"));

        let linear = graph.linear_nodes();
        assert_eq!(linear.len(), 1);
        assert_eq!(linear[0].id, "linear");
    }

    #[test]
    fn test_dialogue_graph_total_response_count() {
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("1", "A")
                    .with_response(DialogueResponse::new("R1"))
                    .with_response(DialogueResponse::new("R2")),
            )
            .with_node(DialogueNode::new("2", "B").with_response(DialogueResponse::new("R3")));
        assert_eq!(graph.total_response_count(), 3);
    }

    #[test]
    fn test_dialogue_graph_first_node() {
        let graph = DialogueGraph::new()
            .with_node(DialogueNode::new("first", "Hello"))
            .with_node(DialogueNode::new("second", "World"));
        assert_eq!(graph.first_node().unwrap().id, "first");

        let empty = DialogueGraph::new();
        assert!(empty.first_node().is_none());
    }

    #[test]
    fn test_dialogue_graph_root_nodes() {
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("start", "Hello")
                    .with_response(DialogueResponse::with_next("Continue", "middle")),
            )
            .with_node(
                DialogueNode::new("middle", "How are you?")
                    .with_response(DialogueResponse::with_next("Continue", "end")),
            )
            .with_node(DialogueNode::new("end", "Goodbye"));

        let roots = graph.root_nodes();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, "start");
    }

    #[test]
    fn test_dialogue_graph_node_ids() {
        let graph = DialogueGraph::new()
            .with_node(DialogueNode::new("a", "A"))
            .with_node(DialogueNode::new("b", "B"))
            .with_node(DialogueNode::new("c", "C"));

        let ids = graph.node_ids();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"c"));
    }

    #[test]
    fn test_dialogue_graph_find_nodes_by_text() {
        let graph = DialogueGraph::new()
            .with_node(DialogueNode::new("1", "Hello World"))
            .with_node(DialogueNode::new("2", "Goodbye World"))
            .with_node(DialogueNode::new("3", "Something else"));

        let found = graph.find_nodes_by_text("world");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_dialogue_graph_is_valid() {
        let valid = DialogueGraph::new()
            .with_node(
                DialogueNode::new("1", "Hello")
                    .with_response(DialogueResponse::with_next("Go", "2")),
            )
            .with_node(DialogueNode::new("2", "Bye"));
        assert!(valid.is_valid());

        let invalid = DialogueGraph::new().with_node(
            DialogueNode::new("1", "Hello")
                .with_response(DialogueResponse::with_next("Go", "nonexistent")),
        );
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_dialogue_graph_validation_errors() {
        let graph = DialogueGraph::new().with_node(
            DialogueNode::new("1", "Hello")
                .with_response(DialogueResponse::with_next("A", "missing_a"))
                .with_response(DialogueResponse::with_next("B", "missing_b")),
        );

        let errors = graph.validation_errors();
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().any(|e| e.contains("missing_a")));
        assert!(errors.iter().any(|e| e.contains("missing_b")));
    }

    #[test]
    fn test_dialogue_graph_summary() {
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("choice", "Choose")
                    .with_response(DialogueResponse::new("A"))
                    .with_response(DialogueResponse::new("B")),
            )
            .with_node(DialogueNode::new("end", "Goodbye"));

        let summary = graph.summary();
        assert!(summary.contains("2 nodes"));
        assert!(summary.contains("2 responses"));
        assert!(summary.contains("1 terminals"));
        assert!(summary.contains("1 choices"));
    }

    #[test]
    fn test_dialogue_graph_max_depth_empty() {
        let graph = DialogueGraph::new();
        assert_eq!(graph.max_depth(), 0);
    }

    #[test]
    fn test_dialogue_graph_max_depth_single() {
        let graph = DialogueGraph::new().with_node(DialogueNode::new("only", "Hello"));
        assert_eq!(graph.max_depth(), 1);
    }

    #[test]
    fn test_dialogue_graph_max_depth_chain() {
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("1", "A").with_response(DialogueResponse::with_next("Go", "2")),
            )
            .with_node(
                DialogueNode::new("2", "B").with_response(DialogueResponse::with_next("Go", "3")),
            )
            .with_node(DialogueNode::new("3", "C"));
        assert_eq!(graph.max_depth(), 3);
    }

    #[test]
    fn test_dialogue_graph_display() {
        let graph = DialogueGraph::new().with_node(DialogueNode::new("1", "Hello"));
        let display = format!("{}", graph);
        assert!(display.contains("DialogueGraph"));
        assert!(display.contains("1 nodes"));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let graph = DialogueGraph {
            nodes: vec![DialogueNode {
                id: "start".into(),
                text: "Hello!".into(),
                responses: vec![DialogueResponse {
                    text: "Reply".into(),
                    next_id: Some("end".into()),
                }],
            }],
        };
        let json = serde_json::to_string(&graph).unwrap();
        let parsed: DialogueGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.nodes[0].id, "start");
    }

    #[test]
    fn test_clone() {
        let node = DialogueNode {
            id: "test".into(),
            text: "Text".into(),
            responses: vec![],
        };
        let cloned = node.clone();
        assert_eq!(cloned.id, node.id);
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_empty_id_and_text() {
        let node = DialogueNode::new("", "");
        assert!(node.id.is_empty());
        assert!(node.text.is_empty());
        assert!(node.has_id(""));
    }

    #[test]
    fn test_unicode_text() {
        let node = DialogueNode::new("unicode", "Hello 世界! 🎮");
        assert!(node.text_contains("世界"));
        assert!(node.text_contains("🎮"));
    }

    #[test]
    fn test_very_long_text_truncation() {
        let long_text = "A".repeat(1000);
        let node = DialogueNode::new("test", &long_text);
        let truncated = node.truncated_text(50);
        assert!(truncated.len() <= 50);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_graph_with_cycle_detection() {
        // Cycles shouldn't cause infinite loops in max_depth
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("a", "A").with_response(DialogueResponse::with_next("Go", "b")),
            )
            .with_node(
                DialogueNode::new("b", "B").with_response(DialogueResponse::with_next("Back", "a")),
            ); // Cycle!

        // Should not hang - cycle detection returns 0 for visited nodes
        let depth = graph.max_depth();
        assert!(depth <= 2);
    }

    #[test]
    fn test_response_chain_building() {
        let response = DialogueResponse::new("Start").next("step2");
        assert_eq!(response.text, "Start");
        assert!(response.has_next_id("step2"));
    }

    #[test]
    fn test_dialogue_graph_complex_structure() {
        // Create a complex branching dialogue
        let graph = DialogueGraph::new()
            .with_node(
                DialogueNode::new("start", "Welcome!")
                    .with_response(DialogueResponse::with_next("Accept quest", "accept"))
                    .with_response(DialogueResponse::with_next("Decline quest", "decline"))
                    .with_response(DialogueResponse::new("Leave")),
            )
            .with_node(
                DialogueNode::new("accept", "Thank you!")
                    .with_response(DialogueResponse::with_next("Continue", "quest")),
            )
            .with_node(
                DialogueNode::new("decline", "Maybe another time...")
                    .with_response(DialogueResponse::with_next("Actually, I'll help", "accept"))
                    .with_response(DialogueResponse::new("Goodbye")),
            )
            .with_node(DialogueNode::new("quest", "Go to the forest"));

        assert!(graph.is_valid());
        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.terminal_count(), 1);
        assert_eq!(graph.choice_count(), 2);
        assert_eq!(graph.total_response_count(), 6);
    }
}
