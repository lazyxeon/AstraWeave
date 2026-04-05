// =============================================================================
// AstraWeave Materials — Mutation-Resistant Comprehensive Tests
// =============================================================================
// Targets: Node enum (10 variants, classification, constructors, Display),
//          Graph struct (channels, counting, building, Display),
//          compile_to_wgsl (WGSL generation, bindings, errors),
//          MaterialPackage (from_graph, bindings, Display),
//          BakeConfig (defaults, factories, builders, Display),
//          BakedMaterial (pixel_count, memory, Display),
//          MaterialBaker (bake, validate),
//          BrdfValidation (is_valid, passed_count, failed_checks, Display),
//          validate_brdf (dielectric, metal, clamping),
//          BrdfLut (generate, sample, to_bytes, Display)
// =============================================================================

use astraweave_materials::*;

// ---------------------------------------------------------------------------
// Helper: build a minimal valid Graph with a Constant3 base_color
// ---------------------------------------------------------------------------
fn minimal_graph() -> Graph {
    let mut g = Graph::new("base");
    g.add_node("base", Node::constant3(1.0, 0.0, 0.0));
    g
}

fn graph_with_texture_base() -> Graph {
    let mut g = Graph::new("tex");
    g.add_node("tex", Node::texture_2d("albedo", "uv0"));
    g
}

// ===========================================================================
// 1. Node — type_name exact strings for ALL 10 variants
// ===========================================================================

#[test]
fn node_type_name_texture2d() {
    let n = Node::Texture2D {
        id: "t".into(),
        uv: "u".into(),
    };
    assert_eq!(n.type_name(), "Texture2D");
}

#[test]
fn node_type_name_constant3() {
    let n = Node::Constant3 {
        value: [0.0, 0.0, 0.0],
    };
    assert_eq!(n.type_name(), "Constant3");
}

#[test]
fn node_type_name_constant1() {
    let n = Node::Constant1 { value: 0.0 };
    assert_eq!(n.type_name(), "Constant1");
}

#[test]
fn node_type_name_multiply() {
    let n = Node::Multiply {
        a: "a".into(),
        b: "b".into(),
    };
    assert_eq!(n.type_name(), "Multiply");
}

#[test]
fn node_type_name_add() {
    let n = Node::Add {
        a: "a".into(),
        b: "b".into(),
    };
    assert_eq!(n.type_name(), "Add");
}

#[test]
fn node_type_name_metallic_roughness() {
    let n = Node::MetallicRoughness {
        metallic: "m".into(),
        roughness: "r".into(),
    };
    assert_eq!(n.type_name(), "MetallicRoughness");
}

#[test]
fn node_type_name_clearcoat() {
    let n = Node::Clearcoat {
        weight: "w".into(),
        roughness: "r".into(),
    };
    assert_eq!(n.type_name(), "Clearcoat");
}

#[test]
fn node_type_name_anisotropy() {
    let n = Node::Anisotropy { amount: "a".into() };
    assert_eq!(n.type_name(), "Anisotropy");
}

#[test]
fn node_type_name_transmission() {
    let n = Node::Transmission { amount: "t".into() };
    assert_eq!(n.type_name(), "Transmission");
}

#[test]
fn node_type_name_normal_map() {
    let n = Node::NormalMap {
        id: "n".into(),
        uv: "u".into(),
        scale: 1.0,
    };
    assert_eq!(n.type_name(), "NormalMap");
}

// ===========================================================================
// 2. Node — is_texture (true for Texture2D and NormalMap, false for all others)
// ===========================================================================

#[test]
fn node_is_texture_true_texture2d() {
    assert!(Node::Texture2D {
        id: "t".into(),
        uv: "u".into()
    }
    .is_texture());
}

#[test]
fn node_is_texture_true_normal_map() {
    assert!(Node::NormalMap {
        id: "n".into(),
        uv: "u".into(),
        scale: 1.0
    }
    .is_texture());
}

#[test]
fn node_is_texture_false_constant1() {
    assert!(!Node::Constant1 { value: 1.0 }.is_texture());
}

#[test]
fn node_is_texture_false_constant3() {
    assert!(!Node::Constant3 {
        value: [1.0, 0.0, 0.0]
    }
    .is_texture());
}

#[test]
fn node_is_texture_false_add() {
    assert!(!Node::Add {
        a: "a".into(),
        b: "b".into()
    }
    .is_texture());
}

#[test]
fn node_is_texture_false_multiply() {
    assert!(!Node::Multiply {
        a: "a".into(),
        b: "b".into()
    }
    .is_texture());
}

#[test]
fn node_is_texture_false_metallic_roughness() {
    assert!(!Node::MetallicRoughness {
        metallic: "m".into(),
        roughness: "r".into()
    }
    .is_texture());
}

#[test]
fn node_is_texture_false_clearcoat() {
    assert!(!Node::Clearcoat {
        weight: "w".into(),
        roughness: "r".into()
    }
    .is_texture());
}

#[test]
fn node_is_texture_false_anisotropy() {
    assert!(!Node::Anisotropy { amount: "a".into() }.is_texture());
}

#[test]
fn node_is_texture_false_transmission() {
    assert!(!Node::Transmission { amount: "t".into() }.is_texture());
}

// ===========================================================================
// 3. Node — is_constant (true for Constant1 and Constant3 only)
// ===========================================================================

#[test]
fn node_is_constant_true_constant1() {
    assert!(Node::Constant1 { value: 0.5 }.is_constant());
}

#[test]
fn node_is_constant_true_constant3() {
    assert!(Node::Constant3 {
        value: [0.0, 0.0, 0.0]
    }
    .is_constant());
}

#[test]
fn node_is_constant_false_texture2d() {
    assert!(!Node::Texture2D {
        id: "t".into(),
        uv: "u".into()
    }
    .is_constant());
}

#[test]
fn node_is_constant_false_add() {
    assert!(!Node::Add {
        a: "a".into(),
        b: "b".into()
    }
    .is_constant());
}

#[test]
fn node_is_constant_false_multiply() {
    assert!(!Node::Multiply {
        a: "a".into(),
        b: "b".into()
    }
    .is_constant());
}

#[test]
fn node_is_constant_false_metallic_roughness() {
    assert!(!Node::MetallicRoughness {
        metallic: "m".into(),
        roughness: "r".into()
    }
    .is_constant());
}

#[test]
fn node_is_constant_false_normal_map() {
    assert!(!Node::NormalMap {
        id: "n".into(),
        uv: "u".into(),
        scale: 1.0
    }
    .is_constant());
}

#[test]
fn node_is_constant_false_clearcoat() {
    assert!(!Node::Clearcoat {
        weight: "w".into(),
        roughness: "r".into()
    }
    .is_constant());
}

// ===========================================================================
// 4. Node — is_arithmetic (true for Add and Multiply only)
// ===========================================================================

#[test]
fn node_is_arithmetic_true_add() {
    assert!(Node::Add {
        a: "a".into(),
        b: "b".into()
    }
    .is_arithmetic());
}

#[test]
fn node_is_arithmetic_true_multiply() {
    assert!(Node::Multiply {
        a: "a".into(),
        b: "b".into()
    }
    .is_arithmetic());
}

#[test]
fn node_is_arithmetic_false_constant1() {
    assert!(!Node::Constant1 { value: 1.0 }.is_arithmetic());
}

#[test]
fn node_is_arithmetic_false_constant3() {
    assert!(!Node::Constant3 { value: [0.0; 3] }.is_arithmetic());
}

#[test]
fn node_is_arithmetic_false_texture2d() {
    assert!(!Node::Texture2D {
        id: "t".into(),
        uv: "u".into()
    }
    .is_arithmetic());
}

#[test]
fn node_is_arithmetic_false_metallic_roughness() {
    assert!(!Node::MetallicRoughness {
        metallic: "m".into(),
        roughness: "r".into()
    }
    .is_arithmetic());
}

#[test]
fn node_is_arithmetic_false_normal_map() {
    assert!(!Node::NormalMap {
        id: "n".into(),
        uv: "u".into(),
        scale: 1.0
    }
    .is_arithmetic());
}

// ===========================================================================
// 5. Node — is_pbr_property (true for MR, Clearcoat, Anisotropy, Transmission)
// ===========================================================================

#[test]
fn node_is_pbr_true_metallic_roughness() {
    assert!(Node::MetallicRoughness {
        metallic: "m".into(),
        roughness: "r".into()
    }
    .is_pbr_property());
}

#[test]
fn node_is_pbr_true_clearcoat() {
    assert!(Node::Clearcoat {
        weight: "w".into(),
        roughness: "r".into()
    }
    .is_pbr_property());
}

#[test]
fn node_is_pbr_true_anisotropy() {
    assert!(Node::Anisotropy { amount: "a".into() }.is_pbr_property());
}

#[test]
fn node_is_pbr_true_transmission() {
    assert!(Node::Transmission { amount: "t".into() }.is_pbr_property());
}

#[test]
fn node_is_pbr_false_texture2d() {
    assert!(!Node::Texture2D {
        id: "t".into(),
        uv: "u".into()
    }
    .is_pbr_property());
}

#[test]
fn node_is_pbr_false_constant1() {
    assert!(!Node::Constant1 { value: 1.0 }.is_pbr_property());
}

#[test]
fn node_is_pbr_false_constant3() {
    assert!(!Node::Constant3 { value: [0.0; 3] }.is_pbr_property());
}

#[test]
fn node_is_pbr_false_add() {
    assert!(!Node::Add {
        a: "a".into(),
        b: "b".into()
    }
    .is_pbr_property());
}

#[test]
fn node_is_pbr_false_multiply() {
    assert!(!Node::Multiply {
        a: "a".into(),
        b: "b".into()
    }
    .is_pbr_property());
}

#[test]
fn node_is_pbr_false_normal_map() {
    assert!(!Node::NormalMap {
        id: "n".into(),
        uv: "u".into(),
        scale: 1.0
    }
    .is_pbr_property());
}

// ===========================================================================
// 6. Node — constructors return correct variants
// ===========================================================================

#[test]
fn node_constructor_texture_2d() {
    let n = Node::texture_2d("albedo", "uv0");
    assert_eq!(n.type_name(), "Texture2D");
    assert!(n.is_texture());
    assert!(!n.is_constant());
    assert!(!n.is_arithmetic());
    assert!(!n.is_pbr_property());
}

#[test]
fn node_constructor_constant3() {
    let n = Node::constant3(0.5, 0.3, 0.1);
    assert_eq!(n.type_name(), "Constant3");
    assert!(n.is_constant());
    assert!(!n.is_texture());
}

#[test]
fn node_constructor_constant1() {
    let n = Node::constant1(0.7);
    assert_eq!(n.type_name(), "Constant1");
    assert!(n.is_constant());
}

#[test]
fn node_constructor_add() {
    let n = Node::add("x", "y");
    assert_eq!(n.type_name(), "Add");
    assert!(n.is_arithmetic());
    assert!(!n.is_constant());
}

#[test]
fn node_constructor_multiply() {
    let n = Node::multiply("x", "y");
    assert_eq!(n.type_name(), "Multiply");
    assert!(n.is_arithmetic());
}

#[test]
fn node_constructor_metallic_roughness() {
    let n = Node::metallic_roughness("m", "r");
    assert_eq!(n.type_name(), "MetallicRoughness");
    assert!(n.is_pbr_property());
    assert!(!n.is_texture());
}

#[test]
fn node_constructor_normal_map() {
    let n = Node::normal_map("nrm_tex", "uv0", 2.5);
    assert_eq!(n.type_name(), "NormalMap");
    assert!(n.is_texture());
    assert!(!n.is_pbr_property());
}

// ===========================================================================
// 7. Node — Display exact format strings
// ===========================================================================

#[test]
fn node_display_constant1_exact() {
    assert_eq!(
        format!("{}", Node::Constant1 { value: 0.5 }),
        "Constant1(0.50)"
    );
}

#[test]
fn node_display_constant1_zero() {
    assert_eq!(
        format!("{}", Node::Constant1 { value: 0.0 }),
        "Constant1(0.00)"
    );
}

#[test]
fn node_display_constant3_exact() {
    assert_eq!(
        format!(
            "{}",
            Node::Constant3 {
                value: [1.0, 0.5, 0.0]
            }
        ),
        "Constant3(1.00, 0.50, 0.00)"
    );
}

#[test]
fn node_display_texture2d_exact() {
    assert_eq!(
        format!(
            "{}",
            Node::Texture2D {
                id: "albedo".into(),
                uv: "uv0".into()
            }
        ),
        "Texture2D(id=\"albedo\", uv=uv0)"
    );
}

#[test]
fn node_display_add_exact() {
    assert_eq!(
        format!(
            "{}",
            Node::Add {
                a: "x".into(),
                b: "y".into()
            }
        ),
        "Add(x + y)"
    );
}

#[test]
fn node_display_multiply_exact() {
    assert_eq!(
        format!(
            "{}",
            Node::Multiply {
                a: "x".into(),
                b: "y".into()
            }
        ),
        "Multiply(x × y)"
    );
}

#[test]
fn node_display_metallic_roughness_exact() {
    assert_eq!(
        format!(
            "{}",
            Node::MetallicRoughness {
                metallic: "m".into(),
                roughness: "r".into()
            }
        ),
        "MetallicRoughness(m=m, r=r)"
    );
}

#[test]
fn node_display_normal_map_exact() {
    assert_eq!(
        format!(
            "{}",
            Node::NormalMap {
                id: "nrm".into(),
                uv: "uv".into(),
                scale: 1.0
            }
        ),
        "NormalMap(id=\"nrm\", uv=uv, scale=1.00)"
    );
}

#[test]
fn node_display_clearcoat_exact() {
    assert_eq!(
        format!(
            "{}",
            Node::Clearcoat {
                weight: "w".into(),
                roughness: "r".into()
            }
        ),
        "Clearcoat(w=w, r=r)"
    );
}

#[test]
fn node_display_anisotropy_exact() {
    assert_eq!(
        format!("{}", Node::Anisotropy { amount: "a".into() }),
        "Anisotropy(a)"
    );
}

#[test]
fn node_display_transmission_exact() {
    assert_eq!(
        format!("{}", Node::Transmission { amount: "t".into() }),
        "Transmission(t)"
    );
}

// ===========================================================================
// 8. Node — serde roundtrip for every variant
// ===========================================================================

#[test]
fn node_serde_roundtrip_texture2d() {
    let n = Node::texture_2d("albedo", "uv0");
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

#[test]
fn node_serde_roundtrip_constant3() {
    let n = Node::constant3(0.5, 0.3, 0.1);
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

#[test]
fn node_serde_roundtrip_constant1() {
    let n = Node::constant1(0.7);
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

#[test]
fn node_serde_roundtrip_add() {
    let n = Node::add("a", "b");
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

#[test]
fn node_serde_roundtrip_multiply() {
    let n = Node::multiply("a", "b");
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

#[test]
fn node_serde_roundtrip_metallic_roughness() {
    let n = Node::metallic_roughness("m", "r");
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

#[test]
fn node_serde_roundtrip_normal_map() {
    let n = Node::normal_map("nrm", "uv", 1.5);
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

#[test]
fn node_serde_roundtrip_clearcoat() {
    let n = Node::Clearcoat {
        weight: "w".into(),
        roughness: "r".into(),
    };
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

#[test]
fn node_serde_roundtrip_anisotropy() {
    let n = Node::Anisotropy { amount: "a".into() };
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

#[test]
fn node_serde_roundtrip_transmission() {
    let n = Node::Transmission { amount: "t".into() };
    let json = serde_json::to_string(&n).unwrap();
    let back: Node = serde_json::from_str(&json).unwrap();
    assert_eq!(n, back);
}

// ===========================================================================
// 9. Node — clone independence
// ===========================================================================

#[test]
fn node_clone_is_independent() {
    let a = Node::constant3(1.0, 2.0, 3.0);
    let b = a.clone();
    assert_eq!(a, b);
    // Different allocations — just ensure PartialEq works after clone
    assert_eq!(a.type_name(), b.type_name());
}

// ===========================================================================
// 10. Graph — construction and base_color
// ===========================================================================

#[test]
fn graph_new_sets_base_color() {
    let g = Graph::new("my_base");
    assert_eq!(g.base_color, "my_base");
}

#[test]
fn graph_new_is_empty() {
    let g = Graph::new("base");
    assert!(g.is_empty());
    assert_eq!(g.node_count(), 0);
}

#[test]
fn graph_new_no_channels() {
    let g = Graph::new("base");
    assert!(!g.has_metallic_roughness());
    assert!(!g.has_normal());
    assert!(!g.has_clearcoat());
    assert!(!g.has_anisotropy());
    assert!(!g.has_transmission());
}

// ===========================================================================
// 11. Graph — add_node / get_node / node_count
// ===========================================================================

#[test]
fn graph_add_node_increments_count() {
    let mut g = Graph::new("b");
    assert_eq!(g.node_count(), 0);
    g.add_node("a", Node::constant1(1.0));
    assert_eq!(g.node_count(), 1);
    g.add_node("b", Node::constant1(2.0));
    assert_eq!(g.node_count(), 2);
}

#[test]
fn graph_add_node_makes_non_empty() {
    let mut g = Graph::new("b");
    assert!(g.is_empty());
    g.add_node("a", Node::constant1(1.0));
    assert!(!g.is_empty());
}

#[test]
fn graph_get_node_existing() {
    let mut g = Graph::new("b");
    g.add_node("test", Node::constant1(0.5));
    let n = g.get_node("test");
    assert!(n.is_some());
    assert_eq!(n.unwrap().type_name(), "Constant1");
}

#[test]
fn graph_get_node_missing() {
    let g = Graph::new("b");
    assert!(g.get_node("nonexistent").is_none());
}

#[test]
fn graph_add_node_overwrite_same_key() {
    let mut g = Graph::new("b");
    g.add_node("k", Node::constant1(1.0));
    g.add_node("k", Node::constant3(0.0, 0.0, 0.0));
    assert_eq!(g.node_count(), 1);
    assert_eq!(g.get_node("k").unwrap().type_name(), "Constant3");
}

// ===========================================================================
// 12. Graph — channel builder methods
// ===========================================================================

#[test]
fn graph_with_metallic_roughness_sets_channel() {
    let g = Graph::new("b").with_metallic_roughness("mr");
    assert!(g.has_metallic_roughness());
    assert!(!g.has_normal());
}

#[test]
fn graph_with_normal_sets_channel() {
    let g = Graph::new("b").with_normal("nrm");
    assert!(g.has_normal());
    assert!(!g.has_metallic_roughness());
}

#[test]
fn graph_with_clearcoat_sets_channel() {
    let g = Graph::new("b").with_clearcoat("cc");
    assert!(g.has_clearcoat());
    assert!(!g.has_anisotropy());
}

#[test]
fn graph_chain_all_channels() {
    let g = Graph::new("b")
        .with_metallic_roughness("mr")
        .with_normal("nrm")
        .with_clearcoat("cc");
    assert!(g.has_metallic_roughness());
    assert!(g.has_normal());
    assert!(g.has_clearcoat());
}

// ===========================================================================
// 13. Graph — active_channel_count (starts at 1 for base_color)
// ===========================================================================

#[test]
fn graph_active_channel_count_base_only() {
    let g = Graph::new("b");
    assert_eq!(g.active_channel_count(), 1);
}

#[test]
fn graph_active_channel_count_with_mr() {
    let g = Graph::new("b").with_metallic_roughness("mr");
    assert_eq!(g.active_channel_count(), 2);
}

#[test]
fn graph_active_channel_count_with_mr_and_normal() {
    let g = Graph::new("b")
        .with_metallic_roughness("mr")
        .with_normal("nrm");
    assert_eq!(g.active_channel_count(), 3);
}

#[test]
fn graph_active_channel_count_all_five_channels() {
    // base_color + mr + normal + clearcoat + anisotropy + transmission = 6
    let mut g = Graph::new("b")
        .with_metallic_roughness("mr")
        .with_normal("nrm")
        .with_clearcoat("cc");
    g.anisotropy = Some("aniso".into());
    g.transmission = Some("trans".into());
    assert_eq!(g.active_channel_count(), 6);
}

// ===========================================================================
// 14. Graph — texture_node_count
// ===========================================================================

#[test]
fn graph_texture_node_count_zero() {
    let mut g = Graph::new("b");
    g.add_node("b", Node::constant3(1.0, 0.0, 0.0));
    assert_eq!(g.texture_node_count(), 0);
}

#[test]
fn graph_texture_node_count_one_texture() {
    let mut g = Graph::new("b");
    g.add_node("t1", Node::texture_2d("albedo", "uv"));
    assert_eq!(g.texture_node_count(), 1);
}

#[test]
fn graph_texture_node_count_mixed() {
    let mut g = Graph::new("b");
    g.add_node("t1", Node::texture_2d("albedo", "uv"));
    g.add_node("c1", Node::constant1(0.5));
    g.add_node("n1", Node::normal_map("nrm", "uv", 1.0));
    assert_eq!(g.texture_node_count(), 2);
}

#[test]
fn graph_texture_node_count_constants_not_counted() {
    let mut g = Graph::new("b");
    g.add_node("c1", Node::constant1(1.0));
    g.add_node("c2", Node::constant3(0.0, 0.0, 0.0));
    g.add_node("add", Node::add("c1", "c2"));
    assert_eq!(g.texture_node_count(), 0);
}

// ===========================================================================
// 15. Graph — Display
// ===========================================================================

#[test]
fn graph_display_contains_nodes_and_channels() {
    let mut g = Graph::new("b").with_metallic_roughness("mr");
    g.add_node("c", Node::constant3(1.0, 0.0, 0.0));
    let s = format!("{}", g);
    assert!(s.contains("Graph("), "missing 'Graph(' in: {}", s);
    assert!(s.contains("nodes"), "missing 'nodes' in: {}", s);
    assert!(s.contains("channels"), "missing 'channels' in: {}", s);
}

#[test]
fn graph_display_empty() {
    let g = Graph::new("b");
    let s = format!("{}", g);
    assert!(s.contains("0 nodes") || s.contains("Graph(0"), "got: {}", s);
}

// ===========================================================================
// 16. Graph — serde roundtrip
// ===========================================================================

#[test]
fn graph_serde_roundtrip_minimal() {
    let g = minimal_graph();
    let json = serde_json::to_string(&g).unwrap();
    let back: Graph = serde_json::from_str(&json).unwrap();
    assert_eq!(g, back);
}

#[test]
fn graph_serde_roundtrip_with_channels() {
    let mut g = Graph::new("b")
        .with_metallic_roughness("mr")
        .with_normal("nrm")
        .with_clearcoat("cc");
    g.add_node("b", Node::constant3(1.0, 0.0, 0.0));
    g.add_node("mr", Node::metallic_roughness("m", "r"));
    g.add_node("m", Node::constant1(0.0));
    g.add_node("r", Node::constant1(0.5));
    g.add_node("nrm", Node::normal_map("nrm_tex", "uv", 1.0));
    let json = serde_json::to_string(&g).unwrap();
    let back: Graph = serde_json::from_str(&json).unwrap();
    assert_eq!(g, back);
}

// ===========================================================================
// 17. compile_to_wgsl — success paths
// ===========================================================================

#[test]
fn compile_minimal_graph_contains_eval_material() {
    let g = minimal_graph();
    let (wgsl, _) = compile_to_wgsl(&g).unwrap();
    assert!(
        wgsl.contains("eval_material"),
        "missing eval_material in WGSL"
    );
}

#[test]
fn compile_minimal_graph_no_bindings() {
    let g = minimal_graph();
    let (_, bindings) = compile_to_wgsl(&g).unwrap();
    assert!(bindings.is_empty());
}

#[test]
fn compile_texture_graph_collects_binding() {
    let g = graph_with_texture_base();
    let (wgsl, bindings) = compile_to_wgsl(&g).unwrap();
    assert!(wgsl.contains("textureSample"), "missing textureSample");
    assert!(
        bindings.contains(&"albedo".to_string()),
        "missing albedo binding"
    );
}

#[test]
fn compile_with_mr_channel() {
    let mut g = Graph::new("base").with_metallic_roughness("mr");
    g.add_node("base", Node::constant3(1.0, 0.0, 0.0));
    g.add_node("mr", Node::metallic_roughness("m", "r"));
    g.add_node("m", Node::constant1(0.0));
    g.add_node("r", Node::constant1(0.5));
    let (wgsl, _) = compile_to_wgsl(&g).unwrap();
    assert!(wgsl.contains("eval_material"));
}

#[test]
fn compile_with_normal_channel() {
    let mut g = Graph::new("base").with_normal("nrm");
    g.add_node("base", Node::constant3(1.0, 0.0, 0.0));
    g.add_node("nrm", Node::normal_map("nrm_tex", "uv0", 1.0));
    let (wgsl, bindings) = compile_to_wgsl(&g).unwrap();
    assert!(wgsl.contains("eval_material"));
    assert!(bindings.contains(&"nrm_tex".to_string()));
}

#[test]
fn compile_add_node_in_graph() {
    let mut g = Graph::new("sum");
    g.add_node("a", Node::constant3(1.0, 0.0, 0.0));
    g.add_node("b", Node::constant3(0.0, 1.0, 0.0));
    g.add_node("sum", Node::add("a", "b"));
    let (wgsl, _) = compile_to_wgsl(&g).unwrap();
    assert!(wgsl.contains("eval_material"));
}

#[test]
fn compile_multiply_node_in_graph() {
    let mut g = Graph::new("prod");
    g.add_node("a", Node::constant3(1.0, 1.0, 1.0));
    g.add_node("b", Node::constant1(0.5));
    g.add_node("prod", Node::multiply("a", "b"));
    let (wgsl, _) = compile_to_wgsl(&g).unwrap();
    assert!(wgsl.contains("eval_material"));
}

#[test]
fn compile_clearcoat_channel() {
    let mut g = Graph::new("base").with_clearcoat("cc");
    g.add_node("base", Node::constant3(1.0, 0.0, 0.0));
    g.add_node(
        "cc",
        Node::Clearcoat {
            weight: "w".into(),
            roughness: "r".into(),
        },
    );
    g.add_node("w", Node::constant1(1.0));
    g.add_node("r", Node::constant1(0.1));
    let (wgsl, _) = compile_to_wgsl(&g).unwrap();
    assert!(wgsl.contains("eval_material"));
}

// ===========================================================================
// 18. compile_to_wgsl — error path (missing node)
// ===========================================================================

#[test]
fn compile_missing_base_node_is_error() {
    let g = Graph::new("nonexistent");
    let result = compile_to_wgsl(&g);
    assert!(
        result.is_err(),
        "Expected error for missing base_color node"
    );
}

#[test]
fn compile_missing_referenced_node_is_error() {
    let mut g = Graph::new("sum");
    g.add_node("sum", Node::add("missing_a", "missing_b"));
    let result = compile_to_wgsl(&g);
    assert!(result.is_err());
}

// ===========================================================================
// 19. MaterialPackage
// ===========================================================================

#[test]
fn material_package_from_minimal_graph() {
    let g = minimal_graph();
    let pkg = MaterialPackage::from_graph(&g).unwrap();
    assert!(pkg.wgsl.contains("eval_material"));
    assert!(!pkg.has_bindings());
    assert_eq!(pkg.binding_count(), 0);
}

#[test]
fn material_package_with_texture_has_binding() {
    let g = graph_with_texture_base();
    let pkg = MaterialPackage::from_graph(&g).unwrap();
    assert!(pkg.has_bindings());
    assert_eq!(pkg.binding_count(), 1);
    assert!(pkg.requires_binding("albedo"));
    assert!(!pkg.requires_binding("nonexistent"));
}

#[test]
fn material_package_wgsl_size_nonzero() {
    let g = minimal_graph();
    let pkg = MaterialPackage::from_graph(&g).unwrap();
    assert!(pkg.wgsl_size() > 0);
}

#[test]
fn material_package_display_format() {
    let g = minimal_graph();
    let pkg = MaterialPackage::from_graph(&g).unwrap();
    let s = format!("{}", pkg);
    assert!(s.contains("MaterialPackage("), "got: {}", s);
    assert!(s.contains("bytes"), "got: {}", s);
    assert!(s.contains("bindings"), "got: {}", s);
}

#[test]
fn material_package_serde_roundtrip() {
    let g = minimal_graph();
    let pkg = MaterialPackage::from_graph(&g).unwrap();
    let json = serde_json::to_string(&pkg).unwrap();
    let back: MaterialPackage = serde_json::from_str(&json).unwrap();
    assert_eq!(pkg, back);
}

#[test]
fn material_package_from_invalid_graph_is_error() {
    let g = Graph::new("missing");
    let result = MaterialPackage::from_graph(&g);
    assert!(result.is_err());
}

// ===========================================================================
// 20. BakeConfig — Default exact values
// ===========================================================================

#[test]
fn bake_config_default_resolution() {
    assert_eq!(BakeConfig::default().resolution, 1024);
}

#[test]
fn bake_config_default_samples() {
    assert_eq!(BakeConfig::default().samples, 16);
}

#[test]
fn bake_config_default_generate_mipmaps() {
    assert!(BakeConfig::default().generate_mipmaps);
}

#[test]
fn bake_config_default_compress() {
    assert!(!BakeConfig::default().compress);
}

// ===========================================================================
// 21. BakeConfig — factory methods exact values
// ===========================================================================

#[test]
fn bake_config_preview_resolution() {
    assert_eq!(BakeConfig::preview().resolution, 256);
}

#[test]
fn bake_config_preview_samples() {
    assert_eq!(BakeConfig::preview().samples, 4);
}

#[test]
fn bake_config_preview_no_mipmaps() {
    assert!(!BakeConfig::preview().generate_mipmaps);
}

#[test]
fn bake_config_preview_no_compress() {
    assert!(!BakeConfig::preview().compress);
}

#[test]
fn bake_config_high_quality_resolution() {
    assert_eq!(BakeConfig::high_quality().resolution, 2048);
}

#[test]
fn bake_config_high_quality_samples() {
    assert_eq!(BakeConfig::high_quality().samples, 64);
}

#[test]
fn bake_config_high_quality_mipmaps() {
    assert!(BakeConfig::high_quality().generate_mipmaps);
}

#[test]
fn bake_config_high_quality_compress() {
    assert!(BakeConfig::high_quality().compress);
}

#[test]
fn bake_config_with_resolution_sets_value() {
    assert_eq!(BakeConfig::with_resolution(512).resolution, 512);
}

#[test]
fn bake_config_with_resolution_uses_defaults_for_rest() {
    let cfg = BakeConfig::with_resolution(512);
    assert_eq!(cfg.samples, 16);
    assert!(cfg.generate_mipmaps);
    assert!(!cfg.compress);
}

// ===========================================================================
// 22. BakeConfig — builder methods
// ===========================================================================

#[test]
fn bake_config_samples_builder() {
    let cfg = BakeConfig::with_resolution(256).samples(32);
    assert_eq!(cfg.samples, 32);
    assert_eq!(cfg.resolution, 256);
}

#[test]
fn bake_config_compressed_builder() {
    let cfg = BakeConfig::with_resolution(256).compressed();
    assert!(cfg.compress);
    assert_eq!(cfg.resolution, 256);
}

#[test]
fn bake_config_chain_builders() {
    let cfg = BakeConfig::with_resolution(128).samples(8).compressed();
    assert_eq!(cfg.resolution, 128);
    assert_eq!(cfg.samples, 8);
    assert!(cfg.compress);
}

// ===========================================================================
// 23. BakeConfig — total_pixels and is_power_of_two
// ===========================================================================

#[test]
fn bake_config_total_pixels_64() {
    assert_eq!(BakeConfig::with_resolution(64).total_pixels(), 64 * 64);
}

#[test]
fn bake_config_total_pixels_1024() {
    assert_eq!(
        BakeConfig::with_resolution(1024).total_pixels(),
        1024 * 1024
    );
}

#[test]
fn bake_config_is_power_of_two_true_256() {
    assert!(BakeConfig::with_resolution(256).is_power_of_two());
}

#[test]
fn bake_config_is_power_of_two_true_1024() {
    assert!(BakeConfig::with_resolution(1024).is_power_of_two());
}

#[test]
fn bake_config_is_power_of_two_false_300() {
    assert!(!BakeConfig::with_resolution(300).is_power_of_two());
}

#[test]
fn bake_config_is_power_of_two_false_100() {
    assert!(!BakeConfig::with_resolution(100).is_power_of_two());
}

// ===========================================================================
// 24. BakeConfig — Display
// ===========================================================================

#[test]
fn bake_config_display_default() {
    let s = format!("{}", BakeConfig::default());
    assert!(s.contains("BakeConfig("), "got: {}", s);
    assert!(s.contains("1024x1024"), "got: {}", s);
    assert!(s.contains("16 samples"), "got: {}", s);
    assert!(s.contains("mipmaps"), "got: {}", s);
    assert!(!s.contains("BC7"), "compress=false but got BC7 in: {}", s);
}

#[test]
fn bake_config_display_high_quality() {
    let s = format!("{}", BakeConfig::high_quality());
    assert!(s.contains("2048x2048"), "got: {}", s);
    assert!(s.contains("64 samples"), "got: {}", s);
    assert!(s.contains("BC7"), "compress=true but no BC7 in: {}", s);
}

#[test]
fn bake_config_display_preview_no_mipmaps() {
    let s = format!("{}", BakeConfig::preview());
    assert!(s.contains("256x256"), "got: {}", s);
    assert!(
        !s.contains("mipmaps"),
        "mipmaps=false but got mipmaps in: {}",
        s
    );
}

// ===========================================================================
// 25. BakedMaterial
// ===========================================================================

#[test]
fn baked_material_pixel_count() {
    let baked = BakedMaterial {
        base_color: vec![[1.0; 4]; 16],
        metallic_roughness: vec![[0.0; 4]; 16],
        normal: vec![[0.5; 4]; 16],
        resolution: 4,
        wgsl: String::new(),
    };
    assert_eq!(baked.pixel_count(), 16);
}

#[test]
fn baked_material_is_power_of_two_true() {
    let baked = BakedMaterial {
        base_color: vec![],
        metallic_roughness: vec![],
        normal: vec![],
        resolution: 8,
        wgsl: String::new(),
    };
    assert!(baked.is_power_of_two());
}

#[test]
fn baked_material_is_power_of_two_false() {
    let baked = BakedMaterial {
        base_color: vec![],
        metallic_roughness: vec![],
        normal: vec![],
        resolution: 6,
        wgsl: String::new(),
    };
    assert!(!baked.is_power_of_two());
}

#[test]
fn baked_material_wgsl_size() {
    let baked = BakedMaterial {
        base_color: vec![],
        metallic_roughness: vec![],
        normal: vec![],
        resolution: 4,
        wgsl: "fn test() {}".to_string(),
    };
    assert_eq!(baked.wgsl_size(), 12);
}

#[test]
fn baked_material_memory_usage() {
    let baked = BakedMaterial {
        base_color: vec![[1.0; 4]; 4],
        metallic_roughness: vec![[0.0; 4]; 4],
        normal: vec![[0.5; 4]; 4],
        resolution: 2,
        wgsl: "abc".to_string(),
    };
    // memory = pixel_count * 3 * 16 + wgsl.len() = 4 * 3 * 16 + 3 = 195
    assert_eq!(baked.memory_usage(), 4 * 3 * 16 + 3);
}

#[test]
fn baked_material_display() {
    let baked = BakedMaterial {
        base_color: vec![[1.0; 4]; 64],
        metallic_roughness: vec![[0.0; 4]; 64],
        normal: vec![[0.5; 4]; 64],
        resolution: 8,
        wgsl: "fn test() {}".to_string(),
    };
    let s = format!("{}", baked);
    assert!(s.contains("BakedMaterial("), "got: {}", s);
    assert!(s.contains("8x8"), "got: {}", s);
}

// ===========================================================================
// 26. MaterialBaker — bake & validate
// ===========================================================================

#[test]
fn material_baker_bake_correct_resolution() {
    let baker = MaterialBaker::new(BakeConfig::with_resolution(32));
    let g = minimal_graph();
    let baked = baker.bake(&g).unwrap();
    assert_eq!(baked.resolution, 32);
    assert_eq!(baked.base_color.len(), 32 * 32);
    assert_eq!(baked.metallic_roughness.len(), 32 * 32);
    assert_eq!(baked.normal.len(), 32 * 32);
}

#[test]
fn material_baker_bake_fills_constant_color() {
    let baker = MaterialBaker::new(BakeConfig::with_resolution(4));
    let mut g = Graph::new("c");
    g.add_node("c", Node::constant3(0.8, 0.2, 0.1));
    let baked = baker.bake(&g).unwrap();
    // Every pixel should have the constant color
    for pixel in &baked.base_color {
        assert!((pixel[0] - 0.8).abs() < 1e-5);
        assert!((pixel[1] - 0.2).abs() < 1e-5);
        assert!((pixel[2] - 0.1).abs() < 1e-5);
        assert!((pixel[3] - 1.0).abs() < 1e-5);
    }
}

#[test]
fn material_baker_validate_good_material_no_warnings() {
    let baker = MaterialBaker::new(BakeConfig::with_resolution(4));
    let g = minimal_graph();
    let baked = baker.bake(&g).unwrap();
    let warnings = baker.validate(&baked);
    assert!(warnings.is_empty(), "unexpected warnings: {:?}", warnings);
}

#[test]
fn material_baker_bake_contains_wgsl() {
    let baker = MaterialBaker::new(BakeConfig::with_resolution(4));
    let g = minimal_graph();
    let baked = baker.bake(&g).unwrap();
    assert!(baked.wgsl.contains("eval_material"));
}

// ===========================================================================
// 27. BrdfValidation — is_valid
// ===========================================================================

#[test]
fn brdf_validation_all_true_is_valid() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 0.9,
    };
    assert!(v.is_valid());
}

#[test]
fn brdf_validation_one_false_not_valid() {
    let v = BrdfValidation {
        energy_conservation: false,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 0.9,
    };
    assert!(!v.is_valid());
}

#[test]
fn brdf_validation_reciprocity_false_not_valid() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: false,
        positivity: true,
        max_energy_ratio: 0.9,
    };
    assert!(!v.is_valid());
}

#[test]
fn brdf_validation_positivity_false_not_valid() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: true,
        positivity: false,
        max_energy_ratio: 0.9,
    };
    assert!(!v.is_valid());
}

#[test]
fn brdf_validation_all_false_not_valid() {
    let v = BrdfValidation {
        energy_conservation: false,
        reciprocity: false,
        positivity: false,
        max_energy_ratio: 1.5,
    };
    assert!(!v.is_valid());
}

// ===========================================================================
// 28. BrdfValidation — passed_count
// ===========================================================================

#[test]
fn brdf_validation_passed_count_3() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 0.9,
    };
    assert_eq!(v.passed_count(), 3);
}

#[test]
fn brdf_validation_passed_count_2() {
    let v = BrdfValidation {
        energy_conservation: false,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 0.9,
    };
    assert_eq!(v.passed_count(), 2);
}

#[test]
fn brdf_validation_passed_count_1() {
    let v = BrdfValidation {
        energy_conservation: false,
        reciprocity: true,
        positivity: false,
        max_energy_ratio: 1.5,
    };
    assert_eq!(v.passed_count(), 1);
}

#[test]
fn brdf_validation_passed_count_0() {
    let v = BrdfValidation {
        energy_conservation: false,
        reciprocity: false,
        positivity: false,
        max_energy_ratio: 1.5,
    };
    assert_eq!(v.passed_count(), 0);
}

// ===========================================================================
// 29. BrdfValidation — failed_checks
// ===========================================================================

#[test]
fn brdf_validation_failed_checks_empty_when_all_pass() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 0.9,
    };
    assert!(v.failed_checks().is_empty());
}

#[test]
fn brdf_validation_failed_checks_energy_conservation() {
    let v = BrdfValidation {
        energy_conservation: false,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 1.1,
    };
    let failed = v.failed_checks();
    assert_eq!(failed.len(), 1);
    assert!(failed.contains(&"energy_conservation"));
}

#[test]
fn brdf_validation_failed_checks_two_failures() {
    let v = BrdfValidation {
        energy_conservation: false,
        reciprocity: true,
        positivity: false,
        max_energy_ratio: 1.1,
    };
    let failed = v.failed_checks();
    assert_eq!(failed.len(), 2);
    assert!(failed.contains(&"energy_conservation"));
    assert!(failed.contains(&"positivity"));
}

#[test]
fn brdf_validation_failed_checks_all_three() {
    let v = BrdfValidation {
        energy_conservation: false,
        reciprocity: false,
        positivity: false,
        max_energy_ratio: 1.5,
    };
    let failed = v.failed_checks();
    assert_eq!(failed.len(), 3);
    assert!(failed.contains(&"energy_conservation"));
    assert!(failed.contains(&"reciprocity"));
    assert!(failed.contains(&"positivity"));
}

// ===========================================================================
// 30. BrdfValidation — is_energy_efficient
// ===========================================================================

#[test]
fn brdf_validation_energy_efficient_below_one() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 0.95,
    };
    assert!(v.is_energy_efficient());
}

#[test]
fn brdf_validation_energy_efficient_exactly_one() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 1.0,
    };
    assert!(v.is_energy_efficient());
}

#[test]
fn brdf_validation_not_energy_efficient_above_one() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 1.01,
    };
    assert!(!v.is_energy_efficient());
}

// ===========================================================================
// 31. BrdfValidation — Display
// ===========================================================================

#[test]
fn brdf_validation_display_pass() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 0.95,
    };
    let s = format!("{}", v);
    assert!(s.contains("PASS"), "expected PASS in: {}", s);
    assert!(!s.contains("FAIL"), "unexpected FAIL in: {}", s);
}

#[test]
fn brdf_validation_display_fail() {
    let v = BrdfValidation {
        energy_conservation: false,
        reciprocity: true,
        positivity: true,
        max_energy_ratio: 1.1,
    };
    let s = format!("{}", v);
    assert!(s.contains("FAIL"), "expected FAIL in: {}", s);
    assert!(!s.contains("PASS"), "unexpected PASS in: {}", s);
}

// ===========================================================================
// 32. validate_brdf — dielectric
// ===========================================================================

#[test]
fn validate_brdf_dielectric_energy_conserved() {
    let r = validate_brdf(0.0, 0.5, [0.8, 0.2, 0.1]);
    assert!(r.energy_conservation);
    assert!(r.reciprocity);
    assert!(r.positivity);
    assert!(r.is_valid());
}

#[test]
fn validate_brdf_dielectric_energy_efficient() {
    let r = validate_brdf(0.0, 0.5, [0.5, 0.5, 0.5]);
    assert!(r.is_energy_efficient());
}

// ===========================================================================
// 33. validate_brdf — metal
// ===========================================================================

#[test]
fn validate_brdf_metal_gold() {
    let r = validate_brdf(1.0, 0.3, [1.0, 0.86, 0.57]);
    assert!(r.positivity);
    assert!(r.reciprocity);
}

#[test]
fn validate_brdf_metal_silver() {
    let r = validate_brdf(1.0, 0.2, [0.97, 0.96, 0.91]);
    assert!(r.positivity);
}

// ===========================================================================
// 34. validate_brdf — edge cases & clamping
// ===========================================================================

#[test]
fn validate_brdf_zero_roughness_clamped_to_004() {
    // roughness 0.0 gets clamped to 0.04
    let r = validate_brdf(0.0, 0.0, [0.5, 0.5, 0.5]);
    assert!(r.positivity);
    assert!(r.reciprocity);
}

#[test]
fn validate_brdf_all_zero_base_color() {
    let r = validate_brdf(0.0, 0.5, [0.0, 0.0, 0.0]);
    assert!(r.energy_conservation);
    assert!(r.positivity);
}

#[test]
fn validate_brdf_all_one_base_color() {
    let r = validate_brdf(0.0, 0.5, [1.0, 1.0, 1.0]);
    assert!(r.positivity);
}

#[test]
fn validate_brdf_negative_metallic_clamped() {
    // negative metallic → clamped to 0.0
    let r = validate_brdf(-1.0, 0.5, [0.5, 0.5, 0.5]);
    assert!(r.positivity);
    assert!(r.reciprocity);
}

#[test]
fn validate_brdf_over_one_metallic_clamped() {
    // metallic >1 → clamped to 1.0
    let r = validate_brdf(2.0, 0.5, [0.5, 0.5, 0.5]);
    assert!(r.positivity);
}

// ===========================================================================
// 35. BrdfLut — generate, entry_count, byte_size
// ===========================================================================

#[test]
fn brdf_lut_generate_resolution() {
    let lut = BrdfLut::generate(16);
    assert_eq!(lut.resolution, 16);
}

#[test]
fn brdf_lut_entry_count() {
    let lut = BrdfLut::generate(16);
    assert_eq!(lut.entry_count(), 16 * 16);
}

#[test]
fn brdf_lut_byte_size() {
    let lut = BrdfLut::generate(16);
    // 2 floats * 4 bytes = 8 bytes per entry
    assert_eq!(lut.byte_size(), 16 * 16 * 8);
}

// ===========================================================================
// 36. BrdfLut — sample
// ===========================================================================

#[test]
fn brdf_lut_sample_center() {
    let lut = BrdfLut::generate(32);
    let s = lut.sample(0.5, 0.5);
    assert!(s.is_some());
    let [scale, bias] = s.unwrap();
    assert!(
        (0.0..=1.0).contains(&scale),
        "scale out of range: {}",
        scale
    );
    assert!((0.0..=1.0).contains(&bias), "bias out of range: {}", bias);
}

#[test]
fn brdf_lut_sample_origin() {
    let lut = BrdfLut::generate(16);
    assert!(lut.sample(0.0, 0.0).is_some());
}

#[test]
fn brdf_lut_sample_max() {
    let lut = BrdfLut::generate(16);
    assert!(lut.sample(1.0, 1.0).is_some());
}

#[test]
fn brdf_lut_all_values_in_range() {
    let lut = BrdfLut::generate(16);
    for [scale, bias] in &lut.data {
        assert!(*scale >= 0.0 && *scale <= 1.0, "scale={}", scale);
        assert!(*bias >= 0.0 && *bias <= 1.0, "bias={}", bias);
    }
}

// ===========================================================================
// 37. BrdfLut — to_bytes
// ===========================================================================

#[test]
fn brdf_lut_to_bytes_length() {
    let lut = BrdfLut::generate(8);
    let bytes = lut.to_bytes();
    assert_eq!(bytes.len(), 8 * 8 * 8); // 64 entries * 8 bytes
}

#[test]
fn brdf_lut_to_bytes_roundtrip_first_entry() {
    let lut = BrdfLut::generate(8);
    let bytes = lut.to_bytes();
    // First entry: [scale, bias] → 4 bytes each, little endian
    let scale = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let bias = f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    assert!((scale - lut.data[0][0]).abs() < 1e-7);
    assert!((bias - lut.data[0][1]).abs() < 1e-7);
}

// ===========================================================================
// 38. BrdfLut — Display
// ===========================================================================

#[test]
fn brdf_lut_display() {
    let lut = BrdfLut::generate(32);
    let s = format!("{}", lut);
    assert!(s.contains("BrdfLut("), "got: {}", s);
    assert!(s.contains("32x32"), "got: {}", s);
    assert!(s.contains("bytes"), "got: {}", s);
}

// ===========================================================================
// 39. Edge cases — empty graph compilation
// ===========================================================================

#[test]
fn compile_graph_with_anisotropy_channel() {
    let mut g = Graph::new("base");
    g.add_node("base", Node::constant3(1.0, 0.0, 0.0));
    g.add_node(
        "aniso",
        Node::Anisotropy {
            amount: "amt".into(),
        },
    );
    g.add_node("amt", Node::constant1(0.5));
    g.anisotropy = Some("aniso".into());
    let (wgsl, _) = compile_to_wgsl(&g).unwrap();
    assert!(wgsl.contains("eval_material"));
}

#[test]
fn compile_graph_with_transmission_channel() {
    let mut g = Graph::new("base");
    g.add_node("base", Node::constant3(1.0, 0.0, 0.0));
    g.add_node(
        "trans",
        Node::Transmission {
            amount: "amt".into(),
        },
    );
    g.add_node("amt", Node::constant1(0.3));
    g.transmission = Some("trans".into());
    let (wgsl, _) = compile_to_wgsl(&g).unwrap();
    assert!(wgsl.contains("eval_material"));
}

// ===========================================================================
// 40. Multiple textures collect all bindings
// ===========================================================================

#[test]
fn compile_multiple_textures_collects_all_bindings() {
    let mut g = Graph::new("tex1").with_normal("nrm");
    g.add_node("tex1", Node::texture_2d("albedo_tex", "uv0"));
    g.add_node("nrm", Node::normal_map("normal_tex", "uv0", 1.0));
    let (_, bindings) = compile_to_wgsl(&g).unwrap();
    assert!(bindings.contains(&"albedo_tex".to_string()));
    assert!(bindings.contains(&"normal_tex".to_string()));
    assert_eq!(bindings.len(), 2);
}

// ===========================================================================
// 41. Graph clone independence
// ===========================================================================

#[test]
fn graph_clone_is_independent() {
    let mut g1 = Graph::new("b");
    g1.add_node("c", Node::constant3(1.0, 0.0, 0.0));
    let mut g2 = g1.clone();
    g2.add_node("extra", Node::constant1(0.5));
    assert_eq!(g1.node_count(), 1);
    assert_eq!(g2.node_count(), 2);
}

// ===========================================================================
// 42. BakeConfig serde roundtrip
// ===========================================================================

#[test]
fn bake_config_serde_roundtrip() {
    let cfg = BakeConfig::high_quality();
    let json = serde_json::to_string(&cfg).unwrap();
    let back: BakeConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(cfg, back);
}

// ===========================================================================
// 43. BrdfValidation clone
// ===========================================================================

#[test]
fn brdf_validation_clone_equality() {
    let v = BrdfValidation {
        energy_conservation: true,
        reciprocity: false,
        positivity: true,
        max_energy_ratio: 0.8,
    };
    let v2 = v.clone();
    assert_eq!(v, v2);
    assert_eq!(v.is_valid(), v2.is_valid());
    assert_eq!(v.passed_count(), v2.passed_count());
}

// ===========================================================================
// 44. MaterialBaker — validate detects out-of-range pixels
// ===========================================================================

#[test]
fn material_baker_validate_detects_out_of_range_base_color() {
    let baker = MaterialBaker::new(BakeConfig::with_resolution(2));
    let baked = BakedMaterial {
        base_color: vec![[-0.1, 0.0, 0.0, 1.0]; 4],
        metallic_roughness: vec![[0.0, 0.5, 0.0, 1.0]; 4],
        normal: vec![[0.5, 0.5, 1.0, 1.0]; 4],
        resolution: 2,
        wgsl: String::new(),
    };
    let warnings = baker.validate(&baked);
    assert!(
        !warnings.is_empty(),
        "expected warning for negative base color"
    );
}

// ===========================================================================
// 45. BakeConfig — total_pixels edge: resolution 1
// ===========================================================================

#[test]
fn bake_config_total_pixels_resolution_1() {
    assert_eq!(BakeConfig::with_resolution(1).total_pixels(), 1);
}

#[test]
fn bake_config_is_power_of_two_resolution_1() {
    assert!(BakeConfig::with_resolution(1).is_power_of_two());
}

// ===========================================================================
// 46. validate_brdf — reciprocity always true (isotropic)
// ===========================================================================

#[test]
fn validate_brdf_reciprocity_always_true() {
    // For various inputs, reciprocity should always be true (isotropic BRDF)
    for m in [0.0, 0.5, 1.0] {
        for r in [0.1, 0.5, 1.0] {
            let result = validate_brdf(m, r, [0.5, 0.5, 0.5]);
            assert!(
                result.reciprocity,
                "expected reciprocity=true for m={}, r={}",
                m, r
            );
        }
    }
}

// ===========================================================================
// 47. Node PartialEq — different variants not equal
// ===========================================================================

#[test]
fn node_partial_eq_different_variants() {
    let a = Node::constant1(1.0);
    let b = Node::constant3(1.0, 0.0, 0.0);
    assert_ne!(a, b);
}

#[test]
fn node_partial_eq_same_variant_different_values() {
    let a = Node::constant1(1.0);
    let b = Node::constant1(2.0);
    assert_ne!(a, b);
}

#[test]
fn node_partial_eq_same_variant_same_values() {
    let a = Node::constant1(0.5);
    let b = Node::constant1(0.5);
    assert_eq!(a, b);
}

// ===========================================================================
// Mutation Kill Tests — targeted at surviving mutants
// ===========================================================================

/// Kills `lib.rs:164` `has_anisotropy → false`
/// Previous tests only checked graphs WITHOUT anisotropy.
#[test]
fn graph_with_anisotropy_reports_has_anisotropy_true() {
    let mut g = Graph::new("base");
    g.anisotropy = Some("aniso".into());
    assert!(
        g.has_anisotropy(),
        "graph with anisotropy set must return true"
    );
}

/// Kills `lib.rs:169` `has_transmission → false`
#[test]
fn graph_with_transmission_reports_has_transmission_true() {
    let mut g = Graph::new("base");
    g.transmission = Some("trans".into());
    assert!(
        g.has_transmission(),
        "graph with transmission set must return true"
    );
}

/// Kills `lib.rs:1017` `wgsl_size → 1`
/// Previous test only checked `> 0`. The actual shader is much larger than 1 byte.
#[test]
fn material_package_wgsl_size_is_realistic() {
    let g = minimal_graph();
    let pkg = MaterialPackage::from_graph(&g).unwrap();
    assert!(
        pkg.wgsl_size() > 10,
        "wgsl shader must be >10 bytes, got {}",
        pkg.wgsl_size()
    );
}

/// Kills validate_brdf arithmetic mutations (lines 1310-1311)
/// With metallic=0.5, base_color=[0.8, 0.2, 0.1]:
///   f0[0] = 0.04*(1-0.5) + 0.8*0.5 = 0.02 + 0.4 = 0.42
///   diffuse = 1 - 0.5 = 0.5
///   max_energy = 0.42 + 0.5*0.96 = 0.9
/// Any arithmetic mutation changes this exact value.
#[test]
fn validate_brdf_exact_max_energy_ratio() {
    let r = validate_brdf(0.5, 0.5, [0.8, 0.2, 0.1]);
    assert!(
        (r.max_energy_ratio - 0.9).abs() < 0.01,
        "expected max_energy_ratio ≈ 0.9, got {}",
        r.max_energy_ratio
    );
}

/// With metallic=0, base_color=[0.5,0.5,0.5]:
///   f0 = [0.04, 0.04, 0.04]
///   max_energy = 0.04 + 1.0*0.96 = 1.0
#[test]
fn validate_brdf_dielectric_exact_max_energy() {
    let r = validate_brdf(0.0, 0.5, [0.5, 0.5, 0.5]);
    assert!(
        (r.max_energy_ratio - 1.0).abs() < 0.01,
        "expected max_energy_ratio ≈ 1.0, got {}",
        r.max_energy_ratio
    );
}

/// With metallic=1. f0=[base_color], diffuse=0, max_energy = max(base_color)
#[test]
fn validate_brdf_full_metal_max_energy_equals_max_base_color() {
    let r = validate_brdf(1.0, 0.5, [0.9, 0.7, 0.3]);
    assert!(
        (r.max_energy_ratio - 0.9).abs() < 0.01,
        "expected max_energy_ratio ≈ 0.9, got {}",
        r.max_energy_ratio
    );
}

/// Kills MaterialBaker::validate normal map mutations (line 1234)
/// Validate should detect denormalized normal maps.
#[test]
fn material_baker_validate_detects_bad_normals() {
    let baker = MaterialBaker::new(BakeConfig::with_resolution(2));
    let g = minimal_graph();
    let mut baked = baker.bake(&g).unwrap();
    // Set a wildly denormalized normal — length ≈ √(4+4+4) = 3.46, far from 1.0
    baked.normal[0] = [1.0, 1.0, 1.0, 1.0]; // maps to nx=1, ny=1, nz=1, len=√3≈1.73
                                            // Actually, normal is in [0,1] range, converted: nx=1*2-1=1, ny=1*2-1=1, nz=1*2-1=1
                                            // len = √(1+1+1) = √3 ≈ 1.732, (1.732-1.0).abs() = 0.732 > 0.1 → should warn
    let warnings = baker.validate(&baked);
    assert!(
        !warnings.is_empty(),
        "denormalized normals should produce warnings"
    );
}

/// Kills BrdfLut math mutations by verifying specific LUT values.
/// At high NdotV (head-on) with low roughness, scale should be high, bias low.
/// At low NdotV (grazing) with low roughness, Fresnel dominance → bias > scale.
#[test]
fn brdf_lut_sample_values_discriminate_math() {
    let lut = BrdfLut::generate(32);

    // Head-on view (NdotV≈1), smooth surface (roughness≈0)
    // Scale should be high (>0.5), bias should be low (<0.3)
    let [s_hi, b_hi] = lut.sample(0.95, 0.1).unwrap();
    assert!(
        s_hi > 0.5,
        "head-on smooth: scale should be >0.5, got {}",
        s_hi
    );
    assert!(
        b_hi < 0.3,
        "head-on smooth: bias should be <0.3, got {}",
        b_hi
    );

    // Grazing view (NdotV≈0), smooth surface → strong Fresnel
    let [s_lo, b_lo] = lut.sample(0.05, 0.1).unwrap();
    assert!(
        b_lo > s_lo,
        "grazing smooth: bias should exceed scale, got s={}, b={}",
        s_lo,
        b_lo
    );
}

/// Verify geometry_smith behavior through LUT — rough surfaces should reduce specular
#[test]
fn brdf_lut_rough_surface_reduces_specular() {
    let lut = BrdfLut::generate(32);
    let [s_smooth, _] = lut.sample(0.5, 0.1).unwrap();
    let [s_rough, _] = lut.sample(0.5, 0.9).unwrap();
    // Smoother surfaces should have higher specular scale
    assert!(
        s_smooth > s_rough,
        "smooth should have higher scale than rough: smooth={}, rough={}",
        s_smooth,
        s_rough
    );
}
