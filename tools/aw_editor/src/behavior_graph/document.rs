use astraweave_behavior::{BehaviorGraph, BehaviorNode, DecoratorType};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub type NodeId = u64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

impl Default for NodePosition {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecoratorNode {
    pub decorator: DecoratorKind,
    pub child: Option<NodeId>,
}

impl DecoratorNode {
    pub fn new(decorator: DecoratorKind) -> Self {
        Self {
            decorator,
            child: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DecoratorKind {
    Inverter,
    Succeeder,
    Failer,
    Repeat(u32),
    Retry(u32),
}

impl std::fmt::Display for DecoratorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecoratorKind::Inverter => write!(f, "Inverter"),
            DecoratorKind::Succeeder => write!(f, "Succeeder"),
            DecoratorKind::Failer => write!(f, "Failer"),
            DecoratorKind::Repeat(count) => write!(f, "Repeat ({})", count),
            DecoratorKind::Retry(count) => write!(f, "Retry ({})", count),
        }
    }
}

impl Default for DecoratorKind {
    fn default() -> Self {
        Self::Inverter
    }
}

impl DecoratorKind {
    /// Returns all decorator kind variants (using default counts).
    pub fn all_variants() -> &'static [&'static str] {
        &["Inverter", "Succeeder", "Failer", "Repeat", "Retry"]
    }

    /// Returns the display name of this decorator.
    pub fn name(&self) -> &'static str {
        match self {
            DecoratorKind::Inverter => "Inverter",
            DecoratorKind::Succeeder => "Succeeder",
            DecoratorKind::Failer => "Failer",
            DecoratorKind::Repeat(_) => "Repeat",
            DecoratorKind::Retry(_) => "Retry",
        }
    }

    /// Returns an icon for this decorator.
    pub fn icon(&self) -> &'static str {
        match self {
            DecoratorKind::Inverter => "\u{1f504}",
            DecoratorKind::Succeeder => "\u{2705}",
            DecoratorKind::Failer => "\u{274c}",
            DecoratorKind::Repeat(_) => "\u{1f501}",
            DecoratorKind::Retry(_) => "\u{1f502}",
        }
    }

    /// Returns true if this decorator modifies the result.
    pub fn modifies_result(&self) -> bool {
        matches!(
            self,
            DecoratorKind::Inverter | DecoratorKind::Succeeder | DecoratorKind::Failer
        )
    }

    /// Returns true if this decorator loops execution.
    pub fn is_looping(&self) -> bool {
        matches!(self, DecoratorKind::Repeat(_) | DecoratorKind::Retry(_))
    }

    /// Returns the loop count if this is a looping decorator.
    pub fn loop_count(&self) -> Option<u32> {
        match self {
            DecoratorKind::Repeat(count) | DecoratorKind::Retry(count) => Some(*count),
            _ => None,
        }
    }

    fn to_runtime(&self) -> DecoratorType {
        match self {
            DecoratorKind::Inverter => DecoratorType::Inverter,
            DecoratorKind::Succeeder => DecoratorType::Succeeder,
            DecoratorKind::Failer => DecoratorType::Failer,
            DecoratorKind::Repeat(max) => DecoratorType::Repeat(*max),
            DecoratorKind::Retry(max) => DecoratorType::Retry(*max),
        }
    }

    fn from_runtime(decorator: &DecoratorType) -> Self {
        match decorator {
            DecoratorType::Inverter => DecoratorKind::Inverter,
            DecoratorType::Succeeder => DecoratorKind::Succeeder,
            DecoratorType::Failer => DecoratorKind::Failer,
            DecoratorType::Repeat(max) => DecoratorKind::Repeat(*max),
            DecoratorType::Retry(max) => DecoratorKind::Retry(*max),
            _ => DecoratorKind::Succeeder, // Fallback for future decorator types
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum BehaviorGraphNodeKind {
    Action {
        name: String,
    },
    Condition {
        name: String,
    },
    Sequence {
        children: Vec<NodeId>,
    },
    Selector {
        children: Vec<NodeId>,
    },
    Parallel {
        children: Vec<NodeId>,
        success_threshold: usize,
    },
    Decorator(DecoratorNode),
}

impl std::fmt::Display for BehaviorGraphNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BehaviorGraphNodeKind::Action { name } => write!(f, "Action: {}", name),
            BehaviorGraphNodeKind::Condition { name } => write!(f, "Condition: {}", name),
            BehaviorGraphNodeKind::Sequence { children } => {
                write!(f, "Sequence ({} children)", children.len())
            }
            BehaviorGraphNodeKind::Selector { children } => {
                write!(f, "Selector ({} children)", children.len())
            }
            BehaviorGraphNodeKind::Parallel {
                children,
                success_threshold,
            } => {
                write!(
                    f,
                    "Parallel ({} children, {} threshold)",
                    children.len(),
                    success_threshold
                )
            }
            BehaviorGraphNodeKind::Decorator(node) => write!(f, "Decorator: {}", node.decorator),
        }
    }
}

impl BehaviorGraphNodeKind {
    /// Returns all node kind variant names.
    pub fn all_variants() -> &'static [&'static str] {
        &[
            "Action",
            "Condition",
            "Sequence",
            "Selector",
            "Parallel",
            "Decorator",
        ]
    }

    pub fn display_name(&self) -> &str {
        match self {
            BehaviorGraphNodeKind::Action { .. } => "Action",
            BehaviorGraphNodeKind::Condition { .. } => "Condition",
            BehaviorGraphNodeKind::Sequence { .. } => "Sequence",
            BehaviorGraphNodeKind::Selector { .. } => "Selector",
            BehaviorGraphNodeKind::Parallel { .. } => "Parallel",
            BehaviorGraphNodeKind::Decorator(_) => "Decorator",
        }
    }

    /// Returns an icon for this node kind.
    pub fn icon(&self) -> &'static str {
        match self {
            BehaviorGraphNodeKind::Action { .. } => "\u{26a1}",
            BehaviorGraphNodeKind::Condition { .. } => "\u{2753}",
            BehaviorGraphNodeKind::Sequence { .. } => "\u{27a1}\u{fe0f}",
            BehaviorGraphNodeKind::Selector { .. } => "\u{1f500}",
            BehaviorGraphNodeKind::Parallel { .. } => "\u{23f8}",
            BehaviorGraphNodeKind::Decorator(_) => "\u{1f381}",
        }
    }

    /// Returns true if this is a composite node (can have multiple children).
    pub fn is_composite(&self) -> bool {
        matches!(
            self,
            BehaviorGraphNodeKind::Sequence { .. }
                | BehaviorGraphNodeKind::Selector { .. }
                | BehaviorGraphNodeKind::Parallel { .. }
        )
    }

    /// Returns true if this is a leaf node (Action or Condition).
    pub fn is_leaf(&self) -> bool {
        matches!(
            self,
            BehaviorGraphNodeKind::Action { .. } | BehaviorGraphNodeKind::Condition { .. }
        )
    }

    /// Returns true if this is a decorator node.
    pub fn is_decorator(&self) -> bool {
        matches!(self, BehaviorGraphNodeKind::Decorator(_))
    }

    /// Returns the child count for this node.
    pub fn child_count(&self) -> usize {
        match self {
            BehaviorGraphNodeKind::Sequence { children }
            | BehaviorGraphNodeKind::Selector { children }
            | BehaviorGraphNodeKind::Parallel { children, .. } => children.len(),
            BehaviorGraphNodeKind::Decorator(node) => {
                if node.child.is_some() {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn children(&self) -> Vec<NodeId> {
        match self {
            BehaviorGraphNodeKind::Sequence { children }
            | BehaviorGraphNodeKind::Selector { children }
            | BehaviorGraphNodeKind::Parallel { children, .. } => children.clone(),
            BehaviorGraphNodeKind::Decorator(node) => node.child.into_iter().collect(),
            _ => Vec::new(),
        }
    }

    fn children_mut(&mut self) -> Option<&mut Vec<NodeId>> {
        match self {
            BehaviorGraphNodeKind::Sequence { children }
            | BehaviorGraphNodeKind::Selector { children }
            | BehaviorGraphNodeKind::Parallel { children, .. } => Some(children),
            _ => None,
        }
    }

    fn decorator_child_mut(&mut self) -> Option<&mut Option<NodeId>> {
        match self {
            BehaviorGraphNodeKind::Decorator(node) => Some(&mut node.child),
            _ => None,
        }
    }

    pub fn supports_children(&self) -> bool {
        matches!(
            self,
            BehaviorGraphNodeKind::Sequence { .. }
                | BehaviorGraphNodeKind::Selector { .. }
                | BehaviorGraphNodeKind::Parallel { .. }
                | BehaviorGraphNodeKind::Decorator(_)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorGraphNode {
    pub id: NodeId,
    pub label: String,
    pub position: NodePosition,
    pub kind: BehaviorGraphNodeKind,
}

impl BehaviorGraphNode {
    pub fn new(id: NodeId, label: impl Into<String>, kind: BehaviorGraphNodeKind) -> Self {
        Self {
            id,
            label: label.into(),
            position: NodePosition::default(),
            kind,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BehaviorGraphDocument {
    nodes: Vec<BehaviorGraphNode>,
    root_id: NodeId,
    next_id: NodeId,
    file_path: Option<PathBuf>,
    dirty: bool,
}

impl Default for BehaviorGraphDocument {
    fn default() -> Self {
        Self::new_default()
    }
}

impl BehaviorGraphDocument {
    pub fn new_default() -> Self {
        let mut doc = Self {
            nodes: Vec::new(),
            root_id: 0,
            next_id: 1,
            file_path: None,
            dirty: true,
        };
        let root_id = doc.allocate_id();
        doc.nodes.push(BehaviorGraphNode::new(
            root_id,
            "Idle",
            BehaviorGraphNodeKind::Action {
                name: "idle".into(),
            },
        ));
        doc.root_id = root_id;
        doc
    }

    fn allocate_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn node_index(&self, id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    pub fn node(&self, id: NodeId) -> Option<&BehaviorGraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut BehaviorGraphNode> {
        let idx = self.node_index(id)?;
        self.nodes.get_mut(idx)
    }

    pub fn nodes(&self) -> &[BehaviorGraphNode] {
        &self.nodes
    }

    pub fn root_id(&self) -> NodeId {
        self.root_id
    }

    pub fn set_root(&mut self, id: NodeId) -> Result<(), BehaviorGraphDocumentError> {
        if self.node(id).is_none() {
            return Err(BehaviorGraphDocumentError::MissingNode(id));
        }
        self.root_id = id;
        self.dirty = true;
        Ok(())
    }

    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    pub fn set_file_path(&mut self, path: Option<PathBuf>) {
        self.file_path = path;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn add_node(&mut self, label: impl Into<String>, kind: BehaviorGraphNodeKind) -> NodeId {
        let id = self.allocate_id();
        self.nodes.push(BehaviorGraphNode::new(id, label, kind));
        self.dirty = true;
        id
    }

    pub fn add_child_node(
        &mut self,
        parent: NodeId,
        label: impl Into<String>,
        kind: BehaviorGraphNodeKind,
    ) -> Result<NodeId, BehaviorGraphDocumentError> {
        let Some(parent_snapshot) = self.node(parent) else {
            return Err(BehaviorGraphDocumentError::MissingNode(parent));
        };
        if !parent_snapshot.kind.supports_children() {
            return Err(BehaviorGraphDocumentError::NodeCannotHaveChildren(parent));
        }
        let child_id = self.add_node(label, kind);
        let Some(parent_node) = self.node_mut(parent) else {
            return Err(BehaviorGraphDocumentError::MissingNode(parent));
        };
        match parent_node.kind.children_mut() {
            Some(children) => {
                children.push(child_id);
            }
            None => {
                let Some(slot) = parent_node.kind.decorator_child_mut() else {
                    return Err(BehaviorGraphDocumentError::NodeCannotHaveChildren(parent));
                };
                if slot.replace(child_id).is_some() {
                    return Err(BehaviorGraphDocumentError::DecoratorAlreadyHasChild(parent));
                }
            }
        }
        Ok(child_id)
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Result<(), BehaviorGraphDocumentError> {
        if node_id == self.root_id {
            return Err(BehaviorGraphDocumentError::CannotDeleteRoot);
        }
        if self.node(node_id).is_none() {
            return Err(BehaviorGraphDocumentError::MissingNode(node_id));
        }

        let mut removable = HashSet::new();
        self.collect_descendants(node_id, &mut removable);
        removable.insert(node_id);
        self.nodes.retain(|node| !removable.contains(&node.id));

        for node in &mut self.nodes {
            if let Some(children) = node.kind.children_mut() {
                children.retain(|child| !removable.contains(child));
            }
            if let Some(slot) = node.kind.decorator_child_mut() {
                if slot.map(|id| removable.contains(&id)).unwrap_or(false) {
                    *slot = None;
                }
            }
        }
        self.dirty = true;
        Ok(())
    }

    fn collect_descendants(&self, node_id: NodeId, acc: &mut HashSet<NodeId>) {
        if let Some(node) = self.node(node_id) {
            for child in node.kind.children() {
                if acc.insert(child) {
                    self.collect_descendants(child, acc);
                }
            }
        }
    }

    pub fn save_to_path(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<(), BehaviorGraphDocumentError> {
        let serializable = SerializableDocument::from(&*self);
        let pretty = PrettyConfig::new();
        let text = ron::ser::to_string_pretty(&serializable, pretty)
            .map_err(BehaviorGraphDocumentError::Serialize)?;
        // Atomic write: temp file then rename to prevent corruption on crash
        let tmp_path = path.as_ref().with_extension("tmp");
        fs::write(&tmp_path, &text).map_err(|err| BehaviorGraphDocumentError::Io {
            action: "write temp",
            source: err,
        })?;
        fs::rename(&tmp_path, path.as_ref()).map_err(|err| BehaviorGraphDocumentError::Io {
            action: "rename temp",
            source: err,
        })?;
        self.file_path = Some(path.as_ref().to_path_buf());
        self.dirty = false;
        Ok(())
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, BehaviorGraphDocumentError> {
        let content =
            fs::read_to_string(path.as_ref()).map_err(|err| BehaviorGraphDocumentError::Io {
                action: "read",
                source: err,
            })?;
        let serializable: SerializableDocument =
            ron::from_str(&content).map_err(BehaviorGraphDocumentError::Deserialize)?;
        let mut doc = serializable.into_document();
        doc.file_path = Some(path.as_ref().to_path_buf());
        doc.dirty = false;
        Ok(doc)
    }

    pub fn to_runtime(&self) -> Result<BehaviorGraph, BehaviorGraphDocumentError> {
        let mut visited = HashSet::new();
        let root = self.build_runtime_node(self.root_id, &mut visited)?;
        Ok(BehaviorGraph::new(root))
    }

    fn build_runtime_node(
        &self,
        node_id: NodeId,
        visited: &mut HashSet<NodeId>,
    ) -> Result<BehaviorNode, BehaviorGraphDocumentError> {
        if !visited.insert(node_id) {
            return Err(BehaviorGraphDocumentError::CycleDetected(node_id));
        }

        let node = self
            .node(node_id)
            .ok_or(BehaviorGraphDocumentError::MissingNode(node_id))?;
        let runtime_node = match &node.kind {
            BehaviorGraphNodeKind::Action { name } => BehaviorNode::Action(name.clone()),
            BehaviorGraphNodeKind::Condition { name } => BehaviorNode::Condition(name.clone()),
            BehaviorGraphNodeKind::Sequence { children } => BehaviorNode::Sequence(
                children
                    .iter()
                    .map(|child| self.build_runtime_node(*child, visited))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            BehaviorGraphNodeKind::Selector { children } => BehaviorNode::Selector(
                children
                    .iter()
                    .map(|child| self.build_runtime_node(*child, visited))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            BehaviorGraphNodeKind::Parallel {
                children,
                success_threshold,
            } => BehaviorNode::Parallel(
                children
                    .iter()
                    .map(|child| self.build_runtime_node(*child, visited))
                    .collect::<Result<Vec<_>, _>>()?,
                *success_threshold,
            ),
            BehaviorGraphNodeKind::Decorator(data) => {
                let child_id = data
                    .child
                    .ok_or(BehaviorGraphDocumentError::DecoratorMissingChild(node.id))?;
                let child_node = self.build_runtime_node(child_id, visited)?;
                BehaviorNode::Decorator(data.decorator.to_runtime(), Box::new(child_node))
            }
        };
        visited.remove(&node_id);
        Ok(runtime_node)
    }

    pub fn from_runtime(graph: &BehaviorGraph) -> Self {
        let mut doc = BehaviorGraphDocument {
            nodes: Vec::new(),
            root_id: 0,
            next_id: 1,
            file_path: None,
            dirty: true,
        };
        let root_id = doc.import_runtime_node(&graph.root);
        doc.root_id = root_id;
        doc.dirty = false;
        doc
    }

    fn import_runtime_node(&mut self, node: &BehaviorNode) -> NodeId {
        match node {
            BehaviorNode::Action(name) => self.add_node(
                format!("Action: {}", name),
                BehaviorGraphNodeKind::Action { name: name.clone() },
            ),
            BehaviorNode::Condition(name) => self.add_node(
                format!("Condition: {}", name),
                BehaviorGraphNodeKind::Condition { name: name.clone() },
            ),
            BehaviorNode::Sequence(children) => {
                let id = self.allocate_id();
                let child_ids: Vec<NodeId> = children
                    .iter()
                    .map(|child| self.import_runtime_node(child))
                    .collect();
                self.nodes.push(BehaviorGraphNode::new(
                    id,
                    format!("Sequence ({})", child_ids.len()),
                    BehaviorGraphNodeKind::Sequence {
                        children: child_ids,
                    },
                ));
                id
            }
            BehaviorNode::Selector(children) => {
                let id = self.allocate_id();
                let child_ids: Vec<NodeId> = children
                    .iter()
                    .map(|child| self.import_runtime_node(child))
                    .collect();
                self.nodes.push(BehaviorGraphNode::new(
                    id,
                    format!("Selector ({})", child_ids.len()),
                    BehaviorGraphNodeKind::Selector {
                        children: child_ids,
                    },
                ));
                id
            }
            BehaviorNode::Parallel(children, threshold) => {
                let id = self.allocate_id();
                let child_ids: Vec<NodeId> = children
                    .iter()
                    .map(|child| self.import_runtime_node(child))
                    .collect();
                self.nodes.push(BehaviorGraphNode::new(
                    id,
                    format!("Parallel {}", threshold),
                    BehaviorGraphNodeKind::Parallel {
                        children: child_ids,
                        success_threshold: *threshold,
                    },
                ));
                id
            }
            BehaviorNode::Decorator(decorator, child) => {
                let id = self.allocate_id();
                let child_id = self.import_runtime_node(child);
                self.nodes.push(BehaviorGraphNode::new(
                    id,
                    "Decorator".to_string(),
                    BehaviorGraphNodeKind::Decorator(DecoratorNode {
                        decorator: DecoratorKind::from_runtime(decorator),
                        child: Some(child_id),
                    }),
                ));
                id
            }
            _ => {
                // Future node types: create a placeholder action node
                self.add_node(
                    "Unknown".to_string(),
                    BehaviorGraphNodeKind::Action {
                        name: "unknown".to_string(),
                    },
                )
            }
        }
    }

    pub fn rebuild_next_id(&mut self) {
        self.next_id = self.nodes.iter().map(|node| node.id).max().unwrap_or(0) + 1;
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
#[must_use]
pub enum BehaviorGraphDocumentError {
    #[error("node {0} not found in behavior document")]
    MissingNode(NodeId),
    #[error("node {0} cannot accept children")]
    NodeCannotHaveChildren(NodeId),
    #[error("cycles detected in graph (node {0})")]
    CycleDetected(NodeId),
    #[error("decorator node {0} is missing required child")]
    DecoratorMissingChild(NodeId),
    #[error("decorator node {0} already has a child assigned")]
    DecoratorAlreadyHasChild(NodeId),
    #[error("root node cannot be deleted")]
    CannotDeleteRoot,
    #[error("serialization error: {0}")]
    Serialize(ron::Error),
    #[error("deserialization error: {0}")]
    Deserialize(ron::error::SpannedError),
    #[error("IO error while attempting to {action}: {source}")]
    Io {
        action: &'static str,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Serialize, Deserialize)]
struct SerializableDocument {
    root_id: NodeId,
    next_id: NodeId,
    nodes: Vec<BehaviorGraphNode>,
}

impl From<&BehaviorGraphDocument> for SerializableDocument {
    fn from(value: &BehaviorGraphDocument) -> Self {
        Self {
            root_id: value.root_id,
            next_id: value.next_id,
            nodes: value.nodes.clone(),
        }
    }
}

impl SerializableDocument {
    fn into_document(self) -> BehaviorGraphDocument {
        BehaviorGraphDocument {
            nodes: self.nodes,
            root_id: self.root_id,
            next_id: self.next_id,
            file_path: None,
            dirty: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== DecoratorKind Tests ====================

    #[test]
    fn test_decorator_kind_display() {
        assert_eq!(DecoratorKind::Inverter.to_string(), "Inverter");
        assert_eq!(DecoratorKind::Succeeder.to_string(), "Succeeder");
        assert_eq!(DecoratorKind::Failer.to_string(), "Failer");
        assert!(DecoratorKind::Repeat(5).to_string().contains("5"));
        assert!(DecoratorKind::Retry(3).to_string().contains("3"));
    }

    #[test]
    fn test_decorator_kind_all_variants() {
        let variants = DecoratorKind::all_variants();
        assert_eq!(variants.len(), 5);
        assert!(variants.contains(&"Inverter"));
        assert!(variants.contains(&"Repeat"));
    }

    #[test]
    fn test_decorator_kind_name_and_icon() {
        assert_eq!(DecoratorKind::Inverter.name(), "Inverter");
        assert!(!DecoratorKind::Inverter.icon().is_empty());
        assert!(!DecoratorKind::Repeat(1).icon().is_empty());
    }

    #[test]
    fn test_decorator_kind_helpers() {
        assert!(DecoratorKind::Inverter.modifies_result());
        assert!(DecoratorKind::Succeeder.modifies_result());
        assert!(DecoratorKind::Failer.modifies_result());
        assert!(!DecoratorKind::Repeat(1).modifies_result());

        assert!(DecoratorKind::Repeat(5).is_looping());
        assert!(DecoratorKind::Retry(3).is_looping());
        assert!(!DecoratorKind::Inverter.is_looping());
    }

    #[test]
    fn test_decorator_kind_loop_count() {
        assert_eq!(DecoratorKind::Repeat(5).loop_count(), Some(5));
        assert_eq!(DecoratorKind::Retry(3).loop_count(), Some(3));
        assert_eq!(DecoratorKind::Inverter.loop_count(), None);
    }

    #[test]
    fn test_decorator_kind_default() {
        assert_eq!(DecoratorKind::default(), DecoratorKind::Inverter);
    }

    // ==================== BehaviorGraphNodeKind Tests ====================

    #[test]
    fn test_behavior_graph_node_kind_display() {
        let action = BehaviorGraphNodeKind::Action {
            name: "Attack".to_string(),
        };
        assert!(action.to_string().contains("Attack"));

        let sequence = BehaviorGraphNodeKind::Sequence {
            children: vec![1, 2, 3],
        };
        assert!(sequence.to_string().contains("3 children"));

        let parallel = BehaviorGraphNodeKind::Parallel {
            children: vec![1, 2],
            success_threshold: 1,
        };
        assert!(parallel.to_string().contains("2 children"));
    }

    #[test]
    fn test_behavior_graph_node_kind_all_variants() {
        let variants = BehaviorGraphNodeKind::all_variants();
        assert_eq!(variants.len(), 6);
        assert!(variants.contains(&"Action"));
        assert!(variants.contains(&"Sequence"));
        assert!(variants.contains(&"Decorator"));
    }

    #[test]
    fn test_behavior_graph_node_kind_icon() {
        let action = BehaviorGraphNodeKind::Action {
            name: "Test".to_string(),
        };
        assert!(!action.icon().is_empty());

        let sequence = BehaviorGraphNodeKind::Sequence { children: vec![] };
        assert!(!sequence.icon().is_empty());
    }

    #[test]
    fn test_behavior_graph_node_kind_composite_leaf() {
        let action = BehaviorGraphNodeKind::Action {
            name: "Test".to_string(),
        };
        assert!(action.is_leaf());
        assert!(!action.is_composite());

        let condition = BehaviorGraphNodeKind::Condition {
            name: "Check".to_string(),
        };
        assert!(condition.is_leaf());

        let sequence = BehaviorGraphNodeKind::Sequence { children: vec![] };
        assert!(sequence.is_composite());
        assert!(!sequence.is_leaf());

        let selector = BehaviorGraphNodeKind::Selector { children: vec![] };
        assert!(selector.is_composite());

        let parallel = BehaviorGraphNodeKind::Parallel {
            children: vec![],
            success_threshold: 1,
        };
        assert!(parallel.is_composite());
    }

    #[test]
    fn test_behavior_graph_node_kind_decorator() {
        let decorator =
            BehaviorGraphNodeKind::Decorator(DecoratorNode::new(DecoratorKind::Inverter));
        assert!(decorator.is_decorator());
        assert!(!decorator.is_composite());
        assert!(!decorator.is_leaf());
    }

    #[test]
    fn test_behavior_graph_node_kind_child_count() {
        let action = BehaviorGraphNodeKind::Action {
            name: "Test".to_string(),
        };
        assert_eq!(action.child_count(), 0);

        let sequence = BehaviorGraphNodeKind::Sequence {
            children: vec![1, 2, 3],
        };
        assert_eq!(sequence.child_count(), 3);

        let mut decorator_node = DecoratorNode::new(DecoratorKind::Inverter);
        decorator_node.child = Some(1);
        let decorator = BehaviorGraphNodeKind::Decorator(decorator_node);
        assert_eq!(decorator.child_count(), 1);

        let empty_decorator =
            BehaviorGraphNodeKind::Decorator(DecoratorNode::new(DecoratorKind::Inverter));
        assert_eq!(empty_decorator.child_count(), 0);
    }
}
