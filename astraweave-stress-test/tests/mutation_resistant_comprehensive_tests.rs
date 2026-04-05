//! Mutation-resistant comprehensive tests for astraweave-stress-test.
//!
//! Tests the synchronous entity generation functions and struct fields.

#![allow(clippy::manual_range_contains, clippy::len_zero)]

use astraweave_stress_test::*;

// ═══════════════════════════════════════════════════════════════════════════
// CStressEntity struct
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn stress_entity_fields() {
    let e = CStressEntity {
        id: 42,
        position: [1.0, 2.0, 3.0],
        velocity: [0.1, 0.2, 0.3],
        health: 75.0,
        data: vec![1, 2, 3],
        kind: "warrior".to_string(),
    };
    assert_eq!(e.id, 42);
    assert!((e.position[0] - 1.0).abs() < f32::EPSILON);
    assert!((e.position[1] - 2.0).abs() < f32::EPSILON);
    assert!((e.position[2] - 3.0).abs() < f32::EPSILON);
    assert!((e.velocity[0] - 0.1).abs() < f32::EPSILON);
    assert!((e.velocity[1] - 0.2).abs() < f32::EPSILON);
    assert!((e.velocity[2] - 0.3).abs() < f32::EPSILON);
    assert!((e.health - 75.0).abs() < f32::EPSILON);
    assert_eq!(e.data, vec![1, 2, 3]);
    assert_eq!(e.kind, "warrior");
}

#[test]
fn stress_entity_clone() {
    let e = CStressEntity {
        id: 1,
        position: [10.0, 20.0, 30.0],
        velocity: [0.0, 0.0, 0.0],
        health: 100.0,
        data: vec![0xFF],
        kind: "test".into(),
    };
    let e2 = e.clone();
    assert_eq!(e2.id, 1);
    assert_eq!(e2.kind, "test");
    assert_eq!(e2.data, vec![0xFF]);
}

#[test]
fn stress_entity_debug() {
    let e = CStressEntity {
        id: 0,
        position: [0.0; 3],
        velocity: [0.0; 3],
        health: 0.0,
        data: vec![],
        kind: String::new(),
    };
    let dbg = format!("{e:?}");
    assert!(dbg.contains("CStressEntity"));
}

// ═══════════════════════════════════════════════════════════════════════════
// CAIStress struct
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ai_stress_fields() {
    let ai = CAIStress {
        behavior_tree: vec!["patrol".into(), "attack".into()],
        last_decision: 500,
        decision_count: 1000,
    };
    assert_eq!(ai.behavior_tree.len(), 2);
    assert_eq!(ai.behavior_tree[0], "patrol");
    assert_eq!(ai.behavior_tree[1], "attack");
    assert_eq!(ai.last_decision, 500);
    assert_eq!(ai.decision_count, 1000);
}

#[test]
fn ai_stress_clone() {
    let ai = CAIStress {
        behavior_tree: vec!["node".into()],
        last_decision: 10,
        decision_count: 20,
    };
    let ai2 = ai.clone();
    assert_eq!(ai2.last_decision, 10);
    assert_eq!(ai2.decision_count, 20);
    assert_eq!(ai2.behavior_tree.len(), 1);
}

#[test]
fn ai_stress_debug() {
    let ai = CAIStress {
        behavior_tree: vec![],
        last_decision: 0,
        decision_count: 0,
    };
    let dbg = format!("{ai:?}");
    assert!(dbg.contains("CAIStress"));
}

// ═══════════════════════════════════════════════════════════════════════════
// CNetworkStress struct
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn network_stress_fields() {
    let ns = CNetworkStress {
        player_id: "player_7".into(),
        input_buffer: vec![vec![1, 2], vec![3, 4]],
        last_sync: 42,
    };
    assert_eq!(ns.player_id, "player_7");
    assert_eq!(ns.input_buffer.len(), 2);
    assert_eq!(ns.input_buffer[0], vec![1, 2]);
    assert_eq!(ns.input_buffer[1], vec![3, 4]);
    assert_eq!(ns.last_sync, 42);
}

#[test]
fn network_stress_clone() {
    let ns = CNetworkStress {
        player_id: "p".into(),
        input_buffer: vec![],
        last_sync: 0,
    };
    let ns2 = ns.clone();
    assert_eq!(ns2.player_id, "p");
    assert!(ns2.input_buffer.is_empty());
    assert_eq!(ns2.last_sync, 0);
}

#[test]
fn network_stress_debug() {
    let ns = CNetworkStress {
        player_id: "dbg".into(),
        input_buffer: vec![],
        last_sync: 0,
    };
    let dbg = format!("{ns:?}");
    assert!(dbg.contains("CNetworkStress"));
}

// ═══════════════════════════════════════════════════════════════════════════
// StressTestConfig struct
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn config_fields() {
    let cfg = StressTestConfig {
        entity_count: 100,
        ai_entity_count: 50,
        network_entity_count: 25,
        test_duration_seconds: 10,
        max_memory_mb: 512,
    };
    assert_eq!(cfg.entity_count, 100);
    assert_eq!(cfg.ai_entity_count, 50);
    assert_eq!(cfg.network_entity_count, 25);
    assert_eq!(cfg.test_duration_seconds, 10);
    assert_eq!(cfg.max_memory_mb, 512);
}

#[test]
fn config_clone() {
    let cfg = StressTestConfig {
        entity_count: 10,
        ai_entity_count: 5,
        network_entity_count: 3,
        test_duration_seconds: 1,
        max_memory_mb: 64,
    };
    let cfg2 = cfg.clone();
    assert_eq!(cfg2.entity_count, 10);
    assert_eq!(cfg2.ai_entity_count, 5);
}

// ═══════════════════════════════════════════════════════════════════════════
// generate_stress_entities
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn generate_stress_entities_count() {
    let cfg = StressTestConfig {
        entity_count: 50,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    assert_eq!(entities.len(), 50);
}

#[test]
fn generate_stress_entities_zero_count() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    assert!(entities.is_empty());
}

#[test]
fn generate_stress_entities_ids_sequential() {
    let cfg = StressTestConfig {
        entity_count: 10,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    for (i, e) in entities.iter().enumerate() {
        assert_eq!(e.id, i as u32, "entity {i} should have id {i}");
    }
}

#[test]
fn generate_stress_entities_kind_default() {
    let cfg = StressTestConfig {
        entity_count: 5,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    for e in &entities {
        assert_eq!(e.kind, "default", "all entities should have kind='default'");
    }
}

#[test]
fn generate_stress_entities_positions_in_range() {
    let cfg = StressTestConfig {
        entity_count: 100,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    for e in &entities {
        for &p in &e.position {
            assert!(
                p >= -1000.0 && p < 1000.0,
                "position {p} out of range [-1000, 1000)"
            );
        }
    }
}

#[test]
fn generate_stress_entities_velocities_in_range() {
    let cfg = StressTestConfig {
        entity_count: 100,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    for e in &entities {
        for &v in &e.velocity {
            assert!(
                v >= -10.0 && v < 10.0,
                "velocity {v} out of range [-10, 10)"
            );
        }
    }
}

#[test]
fn generate_stress_entities_health_in_range() {
    let cfg = StressTestConfig {
        entity_count: 100,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    for e in &entities {
        assert!(
            e.health >= 0.0 && e.health < 100.0,
            "health {} out of range",
            e.health
        );
    }
}

#[test]
fn generate_stress_entities_data_size_100_to_1000() {
    let cfg = StressTestConfig {
        entity_count: 50,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    for e in &entities {
        assert!(e.data.len() >= 100, "data too small: {}", e.data.len());
        assert!(e.data.len() < 1000, "data too large: {}", e.data.len());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// generate_ai_stress_entities
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn generate_ai_stress_entities_count() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 30,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_ai_stress_entities(&cfg);
    assert_eq!(entities.len(), 30);
}

#[test]
fn generate_ai_stress_entities_zero() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_ai_stress_entities(&cfg);
    assert!(entities.is_empty());
}

#[test]
fn generate_ai_stress_entities_tree_size_5_to_20() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 50,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_ai_stress_entities(&cfg);
    for ai in &entities {
        assert!(
            ai.behavior_tree.len() >= 5,
            "tree too small: {}",
            ai.behavior_tree.len()
        );
        assert!(
            ai.behavior_tree.len() < 20,
            "tree too large: {}",
            ai.behavior_tree.len()
        );
    }
}

#[test]
fn generate_ai_stress_entities_node_naming() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 10,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_ai_stress_entities(&cfg);
    for ai in &entities {
        for node in &ai.behavior_tree {
            assert!(
                node.starts_with("node_"),
                "node should start with 'node_': {node}"
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// generate_network_stress_entities
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn generate_network_stress_entities_count() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 0,
        network_entity_count: 20,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_network_stress_entities(&cfg);
    assert_eq!(entities.len(), 20);
}

#[test]
fn generate_network_stress_entities_zero() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_network_stress_entities(&cfg);
    assert!(entities.is_empty());
}

#[test]
fn generate_network_stress_entities_player_ids() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 0,
        network_entity_count: 5,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_network_stress_entities(&cfg);
    for (i, e) in entities.iter().enumerate() {
        assert_eq!(
            e.player_id,
            format!("player_{i}"),
            "player_id mismatch at index {i}"
        );
    }
}

#[test]
fn generate_network_stress_entities_buffer_1_to_10() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 0,
        network_entity_count: 50,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_network_stress_entities(&cfg);
    for e in &entities {
        assert!(
            e.input_buffer.len() >= 1,
            "buffer too small: {}",
            e.input_buffer.len()
        );
        assert!(
            e.input_buffer.len() < 10,
            "buffer too large: {}",
            e.input_buffer.len()
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// create_stress_test_app
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn create_stress_test_app_succeeds() {
    let cfg = StressTestConfig {
        entity_count: 10,
        ai_entity_count: 5,
        network_entity_count: 3,
        test_duration_seconds: 1,
        max_memory_mb: 64,
    };
    let result = create_stress_test_app(cfg);
    assert!(result.is_ok(), "create_stress_test_app should succeed");
}

#[test]
fn create_stress_test_app_zero_entities() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let result = create_stress_test_app(cfg);
    assert!(result.is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════
// Boundary conditions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn generate_single_entity() {
    let cfg = StressTestConfig {
        entity_count: 1,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].id, 0);
    assert_eq!(entities[0].kind, "default");
}

#[test]
fn generate_single_ai_entity() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 1,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_ai_stress_entities(&cfg);
    assert_eq!(entities.len(), 1);
    assert!(!entities[0].behavior_tree.is_empty());
}

#[test]
fn generate_single_network_entity() {
    let cfg = StressTestConfig {
        entity_count: 0,
        ai_entity_count: 0,
        network_entity_count: 1,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_network_stress_entities(&cfg);
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].player_id, "player_0");
}

#[test]
fn generate_large_batch() {
    let cfg = StressTestConfig {
        entity_count: 1000,
        ai_entity_count: 0,
        network_entity_count: 0,
        test_duration_seconds: 0,
        max_memory_mb: 0,
    };
    let entities = generate_stress_entities(&cfg);
    assert_eq!(entities.len(), 1000);
    // Verify first and last
    assert_eq!(entities[0].id, 0);
    assert_eq!(entities[999].id, 999);
}
