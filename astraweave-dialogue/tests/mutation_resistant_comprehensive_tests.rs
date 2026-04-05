//! Mutation-resistant tests for astraweave-dialogue public API
//! Targets: DialogueNode, DialogueResponse, DialogueGraph
//! Focus: exact return values, boundary conditions, off-by-one, negation, operator swaps

#![allow(clippy::bool_assert_comparison, clippy::manual_range_contains)]

use astraweave_dialogue::runner::{DialogueRunner, RunnerState};
use astraweave_dialogue::{DialogueGraph, DialogueNode, DialogueResponse};

// ============================= DialogueNode Construction =============================

#[test]
fn node_new_stores_exact_id() {
    let n = DialogueNode::new("alpha", "text");
    assert_eq!(n.id, "alpha");
    assert_ne!(n.id, "");
    assert_ne!(n.id, "Alpha");
}

#[test]
fn node_new_stores_exact_text() {
    let n = DialogueNode::new("id", "some text here");
    assert_eq!(n.text, "some text here");
    assert_ne!(n.text, "");
    assert_ne!(n.text, "some text");
}

#[test]
fn node_new_responses_empty_vec() {
    let n = DialogueNode::new("id", "text");
    assert_eq!(n.responses.len(), 0);
    assert!(n.responses.is_empty());
}

#[test]
fn node_default_all_fields_empty() {
    let n = DialogueNode::default();
    assert_eq!(n.id, "");
    assert_eq!(n.text, "");
    assert_eq!(n.responses.len(), 0);
}

#[test]
fn node_with_responses_replaces_empty() {
    let r = vec![
        DialogueResponse::new("A"),
        DialogueResponse::new("B"),
        DialogueResponse::new("C"),
    ];
    let n = DialogueNode::new("x", "y").with_responses(r);
    assert_eq!(n.response_count(), 3);
    assert_eq!(n.responses[0].text, "A");
    assert_eq!(n.responses[1].text, "B");
    assert_eq!(n.responses[2].text, "C");
}

#[test]
fn node_with_response_appends_not_replaces() {
    let n = DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("first"))
        .with_response(DialogueResponse::new("second"));
    assert_eq!(n.response_count(), 2);
    assert_eq!(n.responses[0].text, "first");
    assert_eq!(n.responses[1].text, "second");
}

// ============================= DialogueNode Boolean Methods =============================

#[test]
fn node_has_responses_false_when_zero() {
    let n = DialogueNode::new("x", "y");
    assert!(!n.has_responses());
    assert_eq!(n.has_responses(), false);
}

#[test]
fn node_has_responses_true_when_one() {
    let n = DialogueNode::new("x", "y").with_response(DialogueResponse::new("r"));
    assert!(n.has_responses());
    assert_eq!(n.has_responses(), true);
}

#[test]
fn node_has_responses_true_when_many() {
    let n = DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("a"))
        .with_response(DialogueResponse::new("b"))
        .with_response(DialogueResponse::new("c"));
    assert!(n.has_responses());
}

#[test]
fn node_response_count_exact_values() {
    assert_eq!(DialogueNode::new("x", "y").response_count(), 0);
    assert_eq!(
        DialogueNode::new("x", "y")
            .with_response(DialogueResponse::new("a"))
            .response_count(),
        1
    );
    assert_eq!(
        DialogueNode::new("x", "y")
            .with_response(DialogueResponse::new("a"))
            .with_response(DialogueResponse::new("b"))
            .response_count(),
        2
    );
}

#[test]
fn node_is_terminal_when_no_responses() {
    let n = DialogueNode::new("end", "bye");
    assert!(n.is_terminal());
    assert_eq!(n.is_terminal(), true);
}

#[test]
fn node_is_terminal_false_with_one_response() {
    let n = DialogueNode::new("x", "y").with_response(DialogueResponse::new("r"));
    assert!(!n.is_terminal());
    assert_eq!(n.is_terminal(), false);
}

#[test]
fn node_is_choice_requires_two_or_more() {
    // 0 responses -> false
    assert!(!DialogueNode::new("x", "y").is_choice());
    // 1 response -> false
    assert!(!DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("a"))
        .is_choice());
    // 2 responses -> true
    assert!(DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("a"))
        .with_response(DialogueResponse::new("b"))
        .is_choice());
    // 3 responses -> true
    assert!(DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("a"))
        .with_response(DialogueResponse::new("b"))
        .with_response(DialogueResponse::new("c"))
        .is_choice());
}

#[test]
fn node_is_linear_requires_exactly_one() {
    // 0 -> false
    assert!(!DialogueNode::new("x", "y").is_linear());
    // 1 -> true
    assert!(DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("a"))
        .is_linear());
    // 2 -> false
    assert!(!DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("a"))
        .with_response(DialogueResponse::new("b"))
        .is_linear());
}

// ============================= DialogueNode Accessor Methods =============================

#[test]
fn node_first_response_returns_first() {
    let n = DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("alpha"))
        .with_response(DialogueResponse::new("beta"));
    let first = n.first_response().unwrap();
    assert_eq!(first.text, "alpha");
    assert_ne!(first.text, "beta");
}

#[test]
fn node_first_response_none_when_empty() {
    assert!(DialogueNode::new("x", "y").first_response().is_none());
}

#[test]
fn node_last_response_returns_last() {
    let n = DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("alpha"))
        .with_response(DialogueResponse::new("omega"));
    let last = n.last_response().unwrap();
    assert_eq!(last.text, "omega");
    assert_ne!(last.text, "alpha");
}

#[test]
fn node_last_response_none_when_empty() {
    assert!(DialogueNode::new("x", "y").last_response().is_none());
}

#[test]
fn node_last_response_same_as_first_when_one() {
    let n = DialogueNode::new("x", "y").with_response(DialogueResponse::new("only"));
    assert_eq!(n.first_response().unwrap().text, "only");
    assert_eq!(n.last_response().unwrap().text, "only");
}

#[test]
fn node_get_response_boundary_indices() {
    let n = DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("zero"))
        .with_response(DialogueResponse::new("one"))
        .with_response(DialogueResponse::new("two"));

    assert_eq!(n.get_response(0).unwrap().text, "zero");
    assert_eq!(n.get_response(1).unwrap().text, "one");
    assert_eq!(n.get_response(2).unwrap().text, "two");
    assert!(n.get_response(3).is_none());
    assert!(n.get_response(usize::MAX).is_none());
}

// ============================= DialogueNode Text Methods =============================

#[test]
fn node_truncated_text_short_text_unchanged() {
    let n = DialogueNode::new("x", "Hi");
    assert_eq!(n.truncated_text(10), "Hi");
    assert_eq!(n.truncated_text(2), "Hi");
}

#[test]
fn node_truncated_text_adds_ellipsis() {
    let n = DialogueNode::new("x", "Hello World");
    let t = n.truncated_text(8);
    assert!(t.ends_with("..."));
    assert!(t.len() <= 8);
}

#[test]
fn node_truncated_text_exact_length_no_truncation() {
    let n = DialogueNode::new("x", "12345");
    assert_eq!(n.truncated_text(5), "12345");
    assert_eq!(n.truncated_text(6), "12345");
}

#[test]
fn node_truncated_text_zero_length() {
    let n = DialogueNode::new("x", "Hello");
    let t = n.truncated_text(0);
    // saturating_sub(3) on 0 gives 0, so empty prefix + "..."
    assert!(t.len() <= 3);
}

#[test]
fn node_summary_terminal_format() {
    let n = DialogueNode::new("end_node", "Goodbye cruel world");
    let s = n.summary();
    assert!(s.contains("[end_node]"), "should contain node id: {}", s);
    assert!(s.contains("(end)"), "terminal should say (end): {}", s);
}

#[test]
fn node_summary_choice_format_shows_count() {
    let n = DialogueNode::new("c", "Pick")
        .with_response(DialogueResponse::new("A"))
        .with_response(DialogueResponse::new("B"))
        .with_response(DialogueResponse::new("C"));
    let s = n.summary();
    assert!(s.contains("3 choices"), "should show 3 choices: {}", s);
}

#[test]
fn node_summary_linear_format() {
    let n = DialogueNode::new("l", "Next").with_response(DialogueResponse::new("Continue"));
    let s = n.summary();
    assert!(
        s.contains("(continue)"),
        "linear should say (continue): {}",
        s
    );
}

#[test]
fn node_has_id_exact_match() {
    let n = DialogueNode::new("test_id_123", "text");
    assert!(n.has_id("test_id_123"));
    assert!(!n.has_id("test_id_12"));
    assert!(!n.has_id("test_id_1234"));
    assert!(!n.has_id("TEST_ID_123"));
    assert!(!n.has_id(""));
}

#[test]
fn node_has_id_empty_id_matches_empty() {
    let n = DialogueNode::new("", "text");
    assert!(n.has_id(""));
    assert!(!n.has_id("x"));
}

#[test]
fn node_text_contains_case_insensitive() {
    let n = DialogueNode::new("x", "Hello World");
    assert!(n.text_contains("hello"));
    assert!(n.text_contains("HELLO"));
    assert!(n.text_contains("Hello"));
    assert!(n.text_contains("world"));
    assert!(n.text_contains("WORLD"));
    assert!(!n.text_contains("goodbye"));
    assert!(!n.text_contains("xyz"));
}

#[test]
fn node_text_contains_empty_substr_matches() {
    let n = DialogueNode::new("x", "Hello");
    assert!(n.text_contains(""));
}

#[test]
fn node_next_node_ids_filters_none() {
    let n = DialogueNode::new("x", "y")
        .with_response(DialogueResponse::with_next("A", "node_a"))
        .with_response(DialogueResponse::new("Terminal"))
        .with_response(DialogueResponse::with_next("B", "node_b"));
    let ids = n.next_node_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"node_a"));
    assert!(ids.contains(&"node_b"));
}

#[test]
fn node_next_node_ids_empty_when_all_terminal() {
    let n = DialogueNode::new("x", "y")
        .with_response(DialogueResponse::new("End1"))
        .with_response(DialogueResponse::new("End2"));
    assert_eq!(n.next_node_ids().len(), 0);
}

#[test]
fn node_next_node_ids_empty_when_no_responses() {
    let n = DialogueNode::new("x", "y");
    assert!(n.next_node_ids().is_empty());
}

#[test]
fn node_leads_to_positive_and_negative() {
    let n = DialogueNode::new("x", "y").with_response(DialogueResponse::with_next("Go", "target"));
    assert!(n.leads_to("target"));
    assert!(!n.leads_to("wrong"));
    assert!(!n.leads_to(""));
}

#[test]
fn node_leads_to_false_when_no_responses() {
    let n = DialogueNode::new("x", "y");
    assert!(!n.leads_to("anything"));
}

#[test]
fn node_display_terminal_contains_keyword() {
    let n = DialogueNode::new("end", "Farewell");
    let d = format!("{}", n);
    assert!(d.contains("terminal"), "display: {}", d);
    assert!(d.contains("end"), "display: {}", d);
}

#[test]
fn node_display_with_responses_shows_count() {
    let n = DialogueNode::new("s", "Hello")
        .with_response(DialogueResponse::new("A"))
        .with_response(DialogueResponse::new("B"));
    let d = format!("{}", n);
    assert!(d.contains("2 responses"), "display: {}", d);
}

// ============================= DialogueResponse Construction =============================

#[test]
fn response_new_exact_fields() {
    let r = DialogueResponse::new("Click me");
    assert_eq!(r.text, "Click me");
    assert!(r.next_id.is_none());
    assert_ne!(r.text, "");
}

#[test]
fn response_default_all_empty() {
    let r = DialogueResponse::default();
    assert_eq!(r.text, "");
    assert_eq!(r.next_id, None);
}

#[test]
fn response_with_next_stores_both() {
    let r = DialogueResponse::with_next("Go", "destination");
    assert_eq!(r.text, "Go");
    assert_eq!(r.next_id, Some("destination".to_string()));
    assert_ne!(r.next_id, None);
}

#[test]
fn response_next_builder_sets_id() {
    let r = DialogueResponse::new("Start").next("step2");
    assert_eq!(r.next_id, Some("step2".to_string()));
    assert_eq!(r.text, "Start"); // text preserved
}

#[test]
fn response_next_builder_overwrites_none() {
    let r = DialogueResponse::new("text");
    assert!(r.next_id.is_none());
    let r2 = r.next("target");
    assert_eq!(r2.next_id, Some("target".to_string()));
}

// ============================= DialogueResponse Boolean Methods =============================

#[test]
fn response_has_next_with_id() {
    assert!(DialogueResponse::with_next("a", "b").has_next());
    assert_eq!(DialogueResponse::with_next("a", "b").has_next(), true);
}

#[test]
fn response_has_next_without_id() {
    assert!(!DialogueResponse::new("a").has_next());
    assert_eq!(DialogueResponse::new("a").has_next(), false);
}

#[test]
fn response_is_terminal_complementary_to_has_next() {
    let with = DialogueResponse::with_next("a", "b");
    assert!(!with.is_terminal());
    assert!(with.has_next());

    let without = DialogueResponse::new("a");
    assert!(without.is_terminal());
    assert!(!without.has_next());
}

#[test]
fn response_has_next_id_exact_match() {
    let r = DialogueResponse::with_next("go", "target_123");
    assert!(r.has_next_id("target_123"));
    assert!(!r.has_next_id("target_12"));
    assert!(!r.has_next_id("target_1234"));
    assert!(!r.has_next_id("TARGET_123"));
    assert!(!r.has_next_id(""));
}

#[test]
fn response_has_next_id_false_when_none() {
    let r = DialogueResponse::new("end");
    assert!(!r.has_next_id("anything"));
    assert!(!r.has_next_id(""));
}

#[test]
fn response_next_node_id_returns_correct() {
    assert_eq!(
        DialogueResponse::with_next("a", "target").next_node_id(),
        Some("target")
    );
    assert_eq!(DialogueResponse::new("a").next_node_id(), None);
}

// ============================= DialogueResponse Text Methods =============================

#[test]
fn response_truncated_text_short_unchanged() {
    let r = DialogueResponse::new("Hi");
    assert_eq!(r.truncated_text(5), "Hi");
    assert_eq!(r.truncated_text(2), "Hi");
}

#[test]
fn response_truncated_text_long_adds_ellipsis() {
    let r = DialogueResponse::new("This is a long response");
    let t = r.truncated_text(10);
    assert!(t.ends_with("..."));
    assert!(t.len() <= 10);
}

#[test]
fn response_truncated_text_exact_boundary() {
    let r = DialogueResponse::new("12345");
    assert_eq!(r.truncated_text(5), "12345");
    assert_eq!(r.truncated_text(4), "1...");
}

#[test]
fn response_summary_with_next() {
    let r = DialogueResponse::with_next("Continue", "next_node");
    let s = r.summary();
    assert!(s.contains("-> next_node"), "summary: {}", s);
    assert!(!s.contains("(end)"));
}

#[test]
fn response_summary_terminal() {
    let r = DialogueResponse::new("The End");
    let s = r.summary();
    assert!(s.contains("(end)"), "summary: {}", s);
    assert!(!s.contains("->"));
}

#[test]
fn response_is_empty_exact() {
    assert!(DialogueResponse::new("").is_empty());
    assert!(!DialogueResponse::new(" ").is_empty());
    assert!(!DialogueResponse::new("x").is_empty());
}

#[test]
fn response_text_len_exact_values() {
    assert_eq!(DialogueResponse::new("").text_len(), 0);
    assert_eq!(DialogueResponse::new("a").text_len(), 1);
    assert_eq!(DialogueResponse::new("hello").text_len(), 5);
    assert_eq!(DialogueResponse::new("hello world").text_len(), 11);
}

#[test]
fn response_text_contains_case_insensitive() {
    let r = DialogueResponse::new("Accept the Quest");
    assert!(r.text_contains("quest"));
    assert!(r.text_contains("QUEST"));
    assert!(r.text_contains("accept"));
    assert!(!r.text_contains("decline"));
}

#[test]
fn response_text_contains_empty_matches() {
    assert!(DialogueResponse::new("hello").text_contains(""));
}

#[test]
fn response_display_with_next() {
    let r = DialogueResponse::with_next("Go", "target");
    let d = format!("{}", r);
    assert!(d.contains("Response"), "display: {}", d);
    assert!(d.contains("-> target"), "display: {}", d);
}

#[test]
fn response_display_terminal() {
    let r = DialogueResponse::new("Goodbye");
    let d = format!("{}", r);
    assert!(d.contains("(end)"), "display: {}", d);
}

// ============================= DialogueGraph Construction =============================

#[test]
fn graph_new_is_empty() {
    let g = DialogueGraph::new();
    assert!(g.is_empty());
    assert_eq!(g.node_count(), 0);
    assert_eq!(g.nodes.len(), 0);
}

#[test]
fn graph_default_is_empty() {
    let g = DialogueGraph::default();
    assert!(g.is_empty());
    assert_eq!(g.node_count(), 0);
}

#[test]
fn graph_with_nodes_preserves_order() {
    let g = DialogueGraph::with_nodes(vec![
        DialogueNode::new("first", "A"),
        DialogueNode::new("second", "B"),
        DialogueNode::new("third", "C"),
    ]);
    assert_eq!(g.node_count(), 3);
    assert_eq!(g.nodes[0].id, "first");
    assert_eq!(g.nodes[1].id, "second");
    assert_eq!(g.nodes[2].id, "third");
}

#[test]
fn graph_add_node_increments_count() {
    let mut g = DialogueGraph::new();
    assert_eq!(g.node_count(), 0);
    g.add_node(DialogueNode::new("1", "A"));
    assert_eq!(g.node_count(), 1);
    g.add_node(DialogueNode::new("2", "B"));
    assert_eq!(g.node_count(), 2);
}

#[test]
fn graph_with_node_builder_chains() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("a", "A"))
        .with_node(DialogueNode::new("b", "B"));
    assert_eq!(g.node_count(), 2);
    assert_eq!(g.nodes[0].id, "a");
    assert_eq!(g.nodes[1].id, "b");
}

// ============================= DialogueGraph Validation =============================

#[test]
fn graph_validate_empty_is_ok() {
    assert!(DialogueGraph::new().validate().is_ok());
}

#[test]
fn graph_validate_valid_chain() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("start", "Hello")
                .with_response(DialogueResponse::with_next("Go", "end")),
        )
        .with_node(DialogueNode::new("end", "Bye"));
    assert!(g.validate().is_ok());
}

#[test]
fn graph_validate_broken_reference_is_err() {
    let g = DialogueGraph::new().with_node(
        DialogueNode::new("start", "Hello")
            .with_response(DialogueResponse::with_next("Go", "nonexistent")),
    );
    let result = g.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("nonexistent"), "error: {}", err_msg);
}

#[test]
fn graph_validate_none_next_id_ok() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("start", "Hello").with_response(DialogueResponse::new("End")));
    assert!(g.validate().is_ok());
}

#[test]
fn graph_validate_self_referencing_ok() {
    let g = DialogueGraph::new().with_node(
        DialogueNode::new("loop", "Repeat")
            .with_response(DialogueResponse::with_next("Again", "loop")),
    );
    assert!(g.validate().is_ok());
}

#[test]
fn graph_is_valid_mirrors_validate() {
    let valid = DialogueGraph::new()
        .with_node(
            DialogueNode::new("a", "A").with_response(DialogueResponse::with_next("Go", "b")),
        )
        .with_node(DialogueNode::new("b", "B"));
    assert!(valid.is_valid());
    assert!(valid.validate().is_ok());

    let invalid = DialogueGraph::new().with_node(
        DialogueNode::new("a", "A").with_response(DialogueResponse::with_next("Go", "missing")),
    );
    assert!(!invalid.is_valid());
    assert!(invalid.validate().is_err());
}

#[test]
fn graph_validation_errors_lists_all_broken() {
    let g = DialogueGraph::new().with_node(
        DialogueNode::new("start", "Choose")
            .with_response(DialogueResponse::with_next("A", "missing_a"))
            .with_response(DialogueResponse::with_next("B", "missing_b"))
            .with_response(DialogueResponse::new("End")),
    );
    let errors = g.validation_errors();
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().any(|e| e.contains("missing_a")));
    assert!(errors.iter().any(|e| e.contains("missing_b")));
}

#[test]
fn graph_validation_errors_empty_when_valid() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("a", "A").with_response(DialogueResponse::with_next("Go", "b")),
        )
        .with_node(DialogueNode::new("b", "B"));
    assert!(g.validation_errors().is_empty());
}

// ============================= DialogueGraph Lookup =============================

#[test]
fn graph_get_node_found() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("alpha", "A"))
        .with_node(DialogueNode::new("beta", "B"));
    let n = g.get_node("beta").unwrap();
    assert_eq!(n.id, "beta");
    assert_eq!(n.text, "B");
}

#[test]
fn graph_get_node_not_found() {
    let g = DialogueGraph::new().with_node(DialogueNode::new("x", "X"));
    assert!(g.get_node("y").is_none());
    assert!(g.get_node("").is_none());
}

#[test]
fn graph_get_node_empty_graph() {
    assert!(DialogueGraph::new().get_node("any").is_none());
}

#[test]
fn graph_get_node_mut_modifies() {
    let mut g = DialogueGraph::new().with_node(DialogueNode::new("target", "Original"));
    g.get_node_mut("target").unwrap().text = "Modified".to_string();
    assert_eq!(g.get_node("target").unwrap().text, "Modified");
}

#[test]
fn graph_get_node_mut_not_found() {
    let mut g = DialogueGraph::new().with_node(DialogueNode::new("x", "X"));
    assert!(g.get_node_mut("missing").is_none());
}

#[test]
fn graph_has_node_exact() {
    let g = DialogueGraph::new().with_node(DialogueNode::new("exists", "E"));
    assert!(g.has_node("exists"));
    assert!(!g.has_node("not_exists"));
    assert!(!g.has_node("EXISTS"));
    assert!(!g.has_node(""));
}

// ============================= DialogueGraph Counting Methods =============================

#[test]
fn graph_node_count_incremental() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("1", "A"))
        .with_node(DialogueNode::new("2", "B"))
        .with_node(DialogueNode::new("3", "C"))
        .with_node(DialogueNode::new("4", "D"));
    assert_eq!(g.node_count(), 4);
    assert_ne!(g.node_count(), 3);
    assert_ne!(g.node_count(), 5);
}

#[test]
fn graph_is_empty_transitions() {
    let g = DialogueGraph::new();
    assert!(g.is_empty());
    let g = g.with_node(DialogueNode::new("1", "A"));
    assert!(!g.is_empty());
}

#[test]
fn graph_terminal_nodes_exact() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("start", "Hello")
                .with_response(DialogueResponse::with_next("Go", "mid")),
        )
        .with_node(
            DialogueNode::new("mid", "Middle")
                .with_response(DialogueResponse::with_next("Go", "end1"))
                .with_response(DialogueResponse::with_next("Go", "end2")),
        )
        .with_node(DialogueNode::new("end1", "Bye1"))
        .with_node(DialogueNode::new("end2", "Bye2"));

    let terminals = g.terminal_nodes();
    assert_eq!(terminals.len(), 2);
    assert_eq!(g.terminal_count(), 2);
    let ids: Vec<&str> = terminals.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"end1"));
    assert!(ids.contains(&"end2"));
    assert!(!ids.contains(&"start"));
    assert!(!ids.contains(&"mid"));
}

#[test]
fn graph_terminal_count_all_terminal() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("a", "A"))
        .with_node(DialogueNode::new("b", "B"));
    assert_eq!(g.terminal_count(), 2);
}

#[test]
fn graph_terminal_count_none_terminal() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("a", "A").with_response(DialogueResponse::new("r")));
    assert_eq!(g.terminal_count(), 0);
}

#[test]
fn graph_choice_nodes_exact() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("choice", "Pick")
                .with_response(DialogueResponse::new("A"))
                .with_response(DialogueResponse::new("B")),
        )
        .with_node(
            DialogueNode::new("linear", "Next").with_response(DialogueResponse::new("Continue")),
        )
        .with_node(DialogueNode::new("terminal", "End"));

    let choices = g.choice_nodes();
    assert_eq!(choices.len(), 1);
    assert_eq!(g.choice_count(), 1);
    assert_eq!(choices[0].id, "choice");
}

#[test]
fn graph_choice_count_zero_when_none() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("a", "A"))
        .with_node(DialogueNode::new("b", "B").with_response(DialogueResponse::new("r")));
    assert_eq!(g.choice_count(), 0);
}

#[test]
fn graph_linear_nodes_exact() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("linear1", "A").with_response(DialogueResponse::new("next")))
        .with_node(DialogueNode::new("linear2", "B").with_response(DialogueResponse::new("next")))
        .with_node(
            DialogueNode::new("choice", "C")
                .with_response(DialogueResponse::new("x"))
                .with_response(DialogueResponse::new("y")),
        )
        .with_node(DialogueNode::new("end", "D"));

    let linear = g.linear_nodes();
    assert_eq!(linear.len(), 2);
    let ids: Vec<&str> = linear.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"linear1"));
    assert!(ids.contains(&"linear2"));
}

#[test]
fn graph_total_response_count_sums_all() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("a", "A")
                .with_response(DialogueResponse::new("r1"))
                .with_response(DialogueResponse::new("r2")),
        )
        .with_node(DialogueNode::new("b", "B").with_response(DialogueResponse::new("r3")))
        .with_node(DialogueNode::new("c", "C")); // 0 responses

    assert_eq!(g.total_response_count(), 3);
    assert_ne!(g.total_response_count(), 2);
    assert_ne!(g.total_response_count(), 4);
}

#[test]
fn graph_total_response_count_zero_when_all_terminal() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("a", "A"))
        .with_node(DialogueNode::new("b", "B"));
    assert_eq!(g.total_response_count(), 0);
}

// ============================= DialogueGraph Navigation =============================

#[test]
fn graph_first_node_returns_first_added() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("first", "F"))
        .with_node(DialogueNode::new("second", "S"));
    assert_eq!(g.first_node().unwrap().id, "first");
    assert_ne!(g.first_node().unwrap().id, "second");
}

#[test]
fn graph_first_node_none_when_empty() {
    assert!(DialogueGraph::new().first_node().is_none());
}

#[test]
fn graph_root_nodes_not_referenced() {
    // start -> mid -> end
    // orphan (not referenced by anyone)
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("start", "S").with_response(DialogueResponse::with_next("Go", "mid")),
        )
        .with_node(
            DialogueNode::new("mid", "M").with_response(DialogueResponse::with_next("Go", "end")),
        )
        .with_node(DialogueNode::new("end", "E"))
        .with_node(DialogueNode::new("orphan", "O"));

    let roots = g.root_nodes();
    let ids: Vec<&str> = roots.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"start"));
    assert!(ids.contains(&"orphan"));
    assert!(!ids.contains(&"mid")); // referenced by start
    assert!(!ids.contains(&"end")); // referenced by mid
}

#[test]
fn graph_root_nodes_all_roots_when_no_links() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("a", "A"))
        .with_node(DialogueNode::new("b", "B"))
        .with_node(DialogueNode::new("c", "C"));
    assert_eq!(g.root_nodes().len(), 3);
}

#[test]
fn graph_node_ids_preserves_order() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("z", "Z"))
        .with_node(DialogueNode::new("a", "A"))
        .with_node(DialogueNode::new("m", "M"));
    let ids = g.node_ids();
    assert_eq!(ids, vec!["z", "a", "m"]);
}

#[test]
fn graph_node_ids_empty_for_empty_graph() {
    assert!(DialogueGraph::new().node_ids().is_empty());
}

#[test]
fn graph_find_nodes_by_text_case_insensitive() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("1", "Hello World"))
        .with_node(DialogueNode::new("2", "Goodbye World"))
        .with_node(DialogueNode::new("3", "No match"));

    let found = g.find_nodes_by_text("world");
    assert_eq!(found.len(), 2);
    let found_upper = g.find_nodes_by_text("WORLD");
    assert_eq!(found_upper.len(), 2);
    let found_none = g.find_nodes_by_text("xyz");
    assert_eq!(found_none.len(), 0);
}

#[test]
fn graph_find_nodes_by_text_empty_matches_all() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("1", "A"))
        .with_node(DialogueNode::new("2", "B"));
    assert_eq!(g.find_nodes_by_text("").len(), 2);
}

// ============================= DialogueGraph Depth =============================

#[test]
fn graph_max_depth_empty_is_zero() {
    assert_eq!(DialogueGraph::new().max_depth(), 0);
}

#[test]
fn graph_max_depth_single_node_is_one() {
    let g = DialogueGraph::new().with_node(DialogueNode::new("x", "X"));
    assert_eq!(g.max_depth(), 1);
    assert_ne!(g.max_depth(), 0);
    assert_ne!(g.max_depth(), 2);
}

#[test]
fn graph_max_depth_chain_of_three() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("1", "A").with_response(DialogueResponse::with_next("Go", "2")),
        )
        .with_node(
            DialogueNode::new("2", "B").with_response(DialogueResponse::with_next("Go", "3")),
        )
        .with_node(DialogueNode::new("3", "C"));
    assert_eq!(g.max_depth(), 3);
}

#[test]
fn graph_max_depth_branching_picks_longest() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("root", "Start")
                .with_response(DialogueResponse::with_next("Short", "end"))
                .with_response(DialogueResponse::with_next("Long", "mid")),
        )
        .with_node(DialogueNode::new("end", "Quick End"))
        .with_node(
            DialogueNode::new("mid", "Middle")
                .with_response(DialogueResponse::with_next("Go", "deep")),
        )
        .with_node(DialogueNode::new("deep", "Deep End"));

    assert_eq!(g.max_depth(), 3); // root -> mid -> deep
}

#[test]
fn graph_max_depth_cycle_does_not_hang() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("a", "A").with_response(DialogueResponse::with_next("Go", "b")),
        )
        .with_node(
            DialogueNode::new("b", "B").with_response(DialogueResponse::with_next("Back", "a")),
        );
    let depth = g.max_depth();
    assert!(depth >= 1 && depth <= 2);
}

// ============================= DialogueGraph Summary & Display =============================

#[test]
fn graph_summary_format_exact() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("choice", "Pick")
                .with_response(DialogueResponse::new("A"))
                .with_response(DialogueResponse::new("B")),
        )
        .with_node(DialogueNode::new("end", "E"));

    let s = g.summary();
    assert!(s.contains("2 nodes"), "summary: {}", s);
    assert!(s.contains("2 responses"), "summary: {}", s);
    assert!(s.contains("1 terminals"), "summary: {}", s);
    assert!(s.contains("1 choices"), "summary: {}", s);
}

#[test]
fn graph_summary_empty() {
    let s = DialogueGraph::new().summary();
    assert!(s.contains("0 nodes"), "summary: {}", s);
    assert!(s.contains("0 responses"), "summary: {}", s);
    assert!(s.contains("0 terminals"), "summary: {}", s);
    assert!(s.contains("0 choices"), "summary: {}", s);
}

#[test]
fn graph_display_matches_summary() {
    let g = DialogueGraph::new().with_node(DialogueNode::new("x", "X"));
    let display = format!("{}", g);
    let summary = g.summary();
    assert_eq!(display, summary);
}

// ============================= Serialization =============================

#[test]
fn node_serde_roundtrip() {
    let original = DialogueNode::new("test_id", "Test text")
        .with_response(DialogueResponse::with_next("Go", "next"))
        .with_response(DialogueResponse::new("End"));
    let json = serde_json::to_string(&original).unwrap();
    let restored: DialogueNode = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, "test_id");
    assert_eq!(restored.text, "Test text");
    assert_eq!(restored.response_count(), 2);
    assert_eq!(restored.responses[0].text, "Go");
    assert_eq!(restored.responses[0].next_id, Some("next".to_string()));
    assert_eq!(restored.responses[1].text, "End");
    assert!(restored.responses[1].next_id.is_none());
}

#[test]
fn response_serde_roundtrip() {
    let r = DialogueResponse::with_next("Continue", "target");
    let json = serde_json::to_string(&r).unwrap();
    let restored: DialogueResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.text, "Continue");
    assert_eq!(restored.next_id, Some("target".to_string()));
}

#[test]
fn response_serde_roundtrip_terminal() {
    let r = DialogueResponse::new("The End");
    let json = serde_json::to_string(&r).unwrap();
    let restored: DialogueResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.text, "The End");
    assert_eq!(restored.next_id, None);
}

#[test]
fn graph_serde_roundtrip_complex() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("start", "Welcome")
                .with_response(DialogueResponse::with_next("Accept", "quest"))
                .with_response(DialogueResponse::new("Decline")),
        )
        .with_node(DialogueNode::new("quest", "Go forth!"));

    let json = serde_json::to_string(&g).unwrap();
    let restored: DialogueGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.node_count(), 2);
    assert!(restored.is_valid());
    assert_eq!(restored.get_node("start").unwrap().response_count(), 2);
    assert_eq!(restored.get_node("quest").unwrap().text, "Go forth!");
}

// ============================= Clone Verification =============================

#[test]
fn node_clone_independent() {
    let original =
        DialogueNode::new("id", "text").with_response(DialogueResponse::with_next("Go", "next"));
    let mut cloned = original.clone();
    cloned.id = "different".to_string();
    cloned.text = "other".to_string();
    assert_eq!(original.id, "id");
    assert_eq!(original.text, "text");
    assert_eq!(cloned.id, "different");
}

#[test]
fn response_clone_independent() {
    let original = DialogueResponse::with_next("Go", "target");
    let mut cloned = original.clone();
    cloned.text = "Modified".to_string();
    assert_eq!(original.text, "Go");
    assert_eq!(cloned.text, "Modified");
}

#[test]
fn graph_clone_independent() {
    let original = DialogueGraph::new().with_node(DialogueNode::new("a", "A"));
    let mut cloned = original.clone();
    cloned.add_node(DialogueNode::new("b", "B"));
    assert_eq!(original.node_count(), 1);
    assert_eq!(cloned.node_count(), 2);
}

// ============================= Edge Cases & Boundary =============================

#[test]
fn node_empty_string_id_and_text() {
    let n = DialogueNode::new("", "");
    assert_eq!(n.id, "");
    assert_eq!(n.text, "");
    assert!(n.has_id(""));
    assert!(!n.has_id("x"));
    assert!(n.is_terminal());
}

#[test]
fn node_unicode_text_handling() {
    let n = DialogueNode::new("uni", "こんにちは世界 🎮");
    assert!(n.text_contains("世界"));
    assert!(n.text_contains("🎮"));
    assert!(!n.text_contains("goodbye"));
}

#[test]
fn node_special_characters_in_id() {
    let n = DialogueNode::new("node-with.dots_and-dashes", "text");
    assert!(n.has_id("node-with.dots_and-dashes"));
    assert!(!n.has_id("node-with.dots_and-dash"));
}

#[test]
fn graph_duplicate_ids_first_found() {
    let g = DialogueGraph::new()
        .with_node(DialogueNode::new("dup", "First"))
        .with_node(DialogueNode::new("dup", "Second"));
    // get_node finds first match
    assert_eq!(g.get_node("dup").unwrap().text, "First");
    assert_eq!(g.node_count(), 2);
}

#[test]
fn graph_validate_with_multiple_errors_reports_per_response() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("n1", "A").with_response(DialogueResponse::with_next("go", "bad1")),
        )
        .with_node(
            DialogueNode::new("n2", "B").with_response(DialogueResponse::with_next("go", "bad2")),
        );
    let errors = g.validation_errors();
    assert_eq!(errors.len(), 2);
    // Check that error messages mention the node ids
    assert!(errors.iter().any(|e| e.contains("n1")));
    assert!(errors.iter().any(|e| e.contains("n2")));
}

#[test]
fn graph_complex_dialogue_tree_counts() {
    let g = DialogueGraph::new()
        .with_node(
            DialogueNode::new("start", "Welcome")
                .with_response(DialogueResponse::with_next("Path A", "path_a"))
                .with_response(DialogueResponse::with_next("Path B", "path_b"))
                .with_response(DialogueResponse::with_next("Quick", "quick_end")),
        )
        .with_node(
            DialogueNode::new("path_a", "On path A")
                .with_response(DialogueResponse::with_next("Continue", "end_a")),
        )
        .with_node(
            DialogueNode::new("path_b", "Choose again")
                .with_response(DialogueResponse::with_next("X", "end_b"))
                .with_response(DialogueResponse::new("Y end")),
        )
        .with_node(DialogueNode::new("quick_end", "Quick goodbye"))
        .with_node(DialogueNode::new("end_a", "End A"))
        .with_node(DialogueNode::new("end_b", "End B"));

    assert_eq!(g.node_count(), 6);
    assert_eq!(g.terminal_count(), 3); // quick_end, end_a, end_b
    assert_eq!(g.choice_count(), 2); // start (3), path_b (2)
    assert_eq!(g.total_response_count(), 6); // 3 + 1 + 2 + 0 + 0 + 0
    assert!(g.is_valid());

    let roots = g.root_nodes();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, "start");
}

#[test]
fn response_text_len_matches_actual_length() {
    let texts = vec!["", "a", "ab", "abc", "a b c", "hello world!"];
    for t in texts {
        let r = DialogueResponse::new(t);
        assert_eq!(r.text_len(), t.len(), "text_len mismatch for '{}'", t);
    }
}

#[test]
fn node_response_interaction_get_first_last_match() {
    let n = DialogueNode::new("x", "y").with_response(DialogueResponse::new("only"));
    assert_eq!(
        n.first_response().unwrap().text,
        n.last_response().unwrap().text
    );
    assert_eq!(n.get_response(0).unwrap().text, "only");
    assert!(n.get_response(1).is_none());
}

#[test]
fn graph_empty_operations_safe() {
    let g = DialogueGraph::new();
    assert!(g.terminal_nodes().is_empty());
    assert!(g.choice_nodes().is_empty());
    assert!(g.linear_nodes().is_empty());
    assert!(g.root_nodes().is_empty());
    assert!(g.node_ids().is_empty());
    assert!(g.find_nodes_by_text("anything").is_empty());
    assert_eq!(g.total_response_count(), 0);
    assert_eq!(g.max_depth(), 0);
    assert!(g.is_valid());
    assert!(g.validation_errors().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// Mutation kill tests — DialogueRunner accessor methods
// ═══════════════════════════════════════════════════════════════════════════

fn make_runner_graph() -> DialogueGraph {
    DialogueGraph::with_nodes(vec![
        DialogueNode::new("start", "Hello!")
            .with_response(DialogueResponse::with_next("Ask more", "mid"))
            .with_response(DialogueResponse::with_next("Bye", "end")),
        DialogueNode::new("mid", "More info.")
            .with_response(DialogueResponse::with_next("Thanks", "end")),
        DialogueNode::new("end", "Farewell."),
    ])
}

/// Kill: available_choices → vec![], vec![""], vec!["xyzzy"]
#[test]
fn available_choices_returns_correct_texts() {
    let mut runner = DialogueRunner::new(make_runner_graph());
    runner.start("start").unwrap();
    let choices = runner.available_choices();
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0], "Ask more");
    assert_eq!(choices[1], "Bye");
}

/// Kill: has_visited → true (when node NOT visited)
/// Kill: has_visited == → !=
#[test]
fn has_visited_returns_false_for_unvisited_node() {
    let mut runner = DialogueRunner::new(make_runner_graph());
    runner.start("start").unwrap();
    assert!(!runner.has_visited("mid"), "mid not yet visited");
    assert!(!runner.has_visited("end"), "end not yet visited");
}

/// Kill: graph() → Default::default()
#[test]
fn graph_accessor_returns_original_graph() {
    let mut runner = DialogueRunner::new(make_runner_graph());
    runner.start("start").unwrap();
    let g = runner.graph();
    assert!(g.get_node("start").is_some());
    assert!(g.get_node("mid").is_some());
    assert!(g.get_node("end").is_some());
}

/// Kill: is_finished → true (before dialogue ends)
#[test]
fn is_finished_false_during_dialogue() {
    let mut runner = DialogueRunner::new(make_runner_graph());
    runner.start("start").unwrap();
    assert!(!runner.is_finished(), "dialogue just started, not finished");
}

/// Kill: is_waiting → true (when idle), is_waiting → false (when waiting)
/// Kill: is_waiting == → !=
#[test]
fn is_waiting_reflects_runner_state() {
    let mut runner = DialogueRunner::new(make_runner_graph());
    // Before start, should NOT be waiting
    assert!(!runner.is_waiting(), "idle runner should not be waiting");
    // After start on a choice node, SHOULD be waiting
    runner.start("start").unwrap();
    assert!(
        runner.is_waiting(),
        "runner at choice node should be waiting"
    );
    // After choosing and reaching terminal, should NOT be waiting
    runner.choose(1).unwrap(); // → end (terminal)
    assert!(
        !runner.is_waiting(),
        "finished runner should not be waiting"
    );
}

/// Kill: peek_events → Vec::leak(Vec::new()) (empty)
#[test]
fn peek_events_shows_pending_events() {
    let mut runner = DialogueRunner::new(make_runner_graph());
    runner.start("start").unwrap();
    let events = runner.peek_events();
    assert!(
        !events.is_empty(),
        "should have NodeEntered event after start"
    );
}
