#[cfg(test)]
mod tests {
    use crate::behavior_graph::document::*;

    #[test]
    fn test_document_defaults() {
        let doc = BehaviorGraphDocument::new_default();
        // One root node created by default
        assert_eq!(doc.nodes().len(), 1);

        let root = doc.node(doc.root_id()).expect("Root node should exist");
        assert_eq!(root.label, "Idle");
        assert!(matches!(root.kind, BehaviorGraphNodeKind::Action { .. }));
    }

    #[test]
    fn test_add_node() {
        let mut doc = BehaviorGraphDocument::new_default();
        let initial_count = doc.nodes().len();

        let id = doc.add_node(
            "MyAction",
            BehaviorGraphNodeKind::Action {
                name: "test".into(),
            },
        );

        assert_eq!(doc.nodes().len(), initial_count + 1);
        assert!(doc.node(id).is_some());

        let node = doc.node(id).unwrap();
        assert_eq!(node.label, "MyAction");
    }

    #[test]
    fn test_add_child_to_sequence() {
        let mut doc = BehaviorGraphDocument::new_default();

        // Add a sequence node
        let seq_id = doc.add_node(
            "Sequence",
            BehaviorGraphNodeKind::Sequence { children: vec![] },
        );

        // Add a child to it
        let child_id = doc
            .add_child_node(
                seq_id,
                "ChildAction",
                BehaviorGraphNodeKind::Action {
                    name: "child".into(),
                },
            )
            .expect("Should successfully add child to sequence");

        // Verify link
        let seq_node = doc.node(seq_id).unwrap();
        match &seq_node.kind {
            BehaviorGraphNodeKind::Sequence { children } => {
                assert!(children.contains(&child_id));
            }
            _ => panic!("Expected Sequence node"),
        }
    }

    #[test]
    fn test_decorator_single_child_limit() {
        let mut doc = BehaviorGraphDocument::new_default();

        let dec_id = doc.add_node(
            "Inverter",
            BehaviorGraphNodeKind::Decorator(DecoratorNode::new(DecoratorKind::Inverter)),
        );

        // First child
        let child1 = doc.add_child_node(
            dec_id,
            "Child1",
            BehaviorGraphNodeKind::Action { name: "c1".into() },
        );
        assert!(child1.is_ok());

        // Second child should fail
        let child2 = doc.add_child_node(
            dec_id,
            "Child2",
            BehaviorGraphNodeKind::Action { name: "c2".into() },
        );
        assert!(child2.is_err()); // DecoratorAlreadyHasChild
    }

    #[test]
    fn test_action_cannot_have_children() {
        let mut doc = BehaviorGraphDocument::new_default();

        let action_id = doc.add_node("Action", BehaviorGraphNodeKind::Action { name: "a".into() });

        let result = doc.add_child_node(
            action_id,
            "Sub",
            BehaviorGraphNodeKind::Action { name: "sub".into() },
        );
        assert!(result.is_err()); // NodeCannotHaveChildren
    }

    #[test]
    fn test_remove_root_fails() {
        let mut doc = BehaviorGraphDocument::new_default();
        let root = doc.root_id();

        let result = doc.remove_node(root);
        assert!(matches!(
            result,
            Err(BehaviorGraphDocumentError::CannotDeleteRoot)
        ));
    }

    #[test]
    fn test_remove_node_and_children_cleanup() {
        // This test assumes remove_node also cleans up parent references?
        // Let's verify implementation of remove_node in previous read.
        // It collects descendants to remove.
        // It "retain"s nodes.
        // Does it remove ID from parent's children list?
        // "for node in &mut self.nodes ..." I need to check the rest of remove_node.

        let mut doc = BehaviorGraphDocument::new_default();
        let seq_id = doc.add_node("Seq", BehaviorGraphNodeKind::Sequence { children: vec![] });
        let child_id = doc
            .add_child_node(
                seq_id,
                "Child",
                BehaviorGraphNodeKind::Action { name: "c".into() },
            )
            .unwrap();

        // Remove child directly
        doc.remove_node(child_id).unwrap();

        assert!(doc.node(child_id).is_none());

        // Verify parent no longer lists it?
        // We will confirm this assumption by running the test.
        let seq = doc.node(seq_id).unwrap();
        match &seq.kind {
            BehaviorGraphNodeKind::Sequence { children } => {
                assert!(
                    !children.contains(&child_id),
                    "Parent should not contain deleted child ID"
                );
            }
            _ => panic!("Expected sequence"),
        }
    }
}
