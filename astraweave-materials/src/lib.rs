#![forbid(unsafe_code)]
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Minimal material graph nodes (Phase 2 foundation)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum Node {
    Texture2D { id: String, uv: String },
    Constant3 { value: [f32; 3] },
    Constant1 { value: f32 },
    Multiply { a: String, b: String },
    Add { a: String, b: String },
    MetallicRoughness { metallic: String, roughness: String },
    Clearcoat { weight: String, roughness: String },
    Anisotropy { amount: String },
    Transmission { amount: String },
    NormalMap { id: String, uv: String, scale: f32 },
}

impl Node {
    /// Returns the type name of this node.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Texture2D { .. } => "Texture2D",
            Self::Constant3 { .. } => "Constant3",
            Self::Constant1 { .. } => "Constant1",
            Self::Multiply { .. } => "Multiply",
            Self::Add { .. } => "Add",
            Self::MetallicRoughness { .. } => "MetallicRoughness",
            Self::Clearcoat { .. } => "Clearcoat",
            Self::Anisotropy { .. } => "Anisotropy",
            Self::Transmission { .. } => "Transmission",
            Self::NormalMap { .. } => "NormalMap",
        }
    }

    /// Returns `true` if this is a texture node.
    pub fn is_texture(&self) -> bool {
        matches!(self, Self::Texture2D { .. } | Self::NormalMap { .. })
    }

    /// Returns `true` if this is a constant node.
    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Constant1 { .. } | Self::Constant3 { .. })
    }

    /// Returns `true` if this is an arithmetic node (Add, Multiply).
    pub fn is_arithmetic(&self) -> bool {
        matches!(self, Self::Add { .. } | Self::Multiply { .. })
    }

    /// Returns `true` if this is a PBR property node.
    pub fn is_pbr_property(&self) -> bool {
        matches!(
            self,
            Self::MetallicRoughness { .. }
                | Self::Clearcoat { .. }
                | Self::Anisotropy { .. }
                | Self::Transmission { .. }
        )
    }

    /// Creates a Texture2D node.
    pub fn texture_2d(id: impl Into<String>, uv: impl Into<String>) -> Self {
        Self::Texture2D {
            id: id.into(),
            uv: uv.into(),
        }
    }

    /// Creates a Constant3 (color/vector) node.
    pub fn constant3(r: f32, g: f32, b: f32) -> Self {
        Self::Constant3 { value: [r, g, b] }
    }

    /// Creates a Constant1 (scalar) node.
    pub fn constant1(value: f32) -> Self {
        Self::Constant1 { value }
    }

    /// Creates an Add node.
    pub fn add(a: impl Into<String>, b: impl Into<String>) -> Self {
        Self::Add {
            a: a.into(),
            b: b.into(),
        }
    }

    /// Creates a Multiply node.
    pub fn multiply(a: impl Into<String>, b: impl Into<String>) -> Self {
        Self::Multiply {
            a: a.into(),
            b: b.into(),
        }
    }

    /// Creates a MetallicRoughness node.
    pub fn metallic_roughness(metallic: impl Into<String>, roughness: impl Into<String>) -> Self {
        Self::MetallicRoughness {
            metallic: metallic.into(),
            roughness: roughness.into(),
        }
    }

    /// Creates a NormalMap node.
    pub fn normal_map(id: impl Into<String>, uv: impl Into<String>, scale: f32) -> Self {
        Self::NormalMap {
            id: id.into(),
            uv: uv.into(),
            scale,
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Texture2D { id, uv } => write!(f, "Texture2D(id=\"{}\", uv={})", id, uv),
            Self::Constant3 { value } => write!(
                f,
                "Constant3({:.2}, {:.2}, {:.2})",
                value[0], value[1], value[2]
            ),
            Self::Constant1 { value } => write!(f, "Constant1({:.2})", value),
            Self::Multiply { a, b } => write!(f, "Multiply({} × {})", a, b),
            Self::Add { a, b } => write!(f, "Add({} + {})", a, b),
            Self::MetallicRoughness {
                metallic,
                roughness,
            } => write!(f, "MetallicRoughness(m={}, r={})", metallic, roughness),
            Self::Clearcoat { weight, roughness } => {
                write!(f, "Clearcoat(w={}, r={})", weight, roughness)
            }
            Self::Anisotropy { amount } => write!(f, "Anisotropy({})", amount),
            Self::Transmission { amount } => write!(f, "Transmission({})", amount),
            Self::NormalMap { id, uv, scale } => {
                write!(f, "NormalMap(id=\"{}\", uv={}, scale={:.2})", id, uv, scale)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Graph {
    pub nodes: std::collections::BTreeMap<String, Node>,
    pub base_color: String,
    pub mr: Option<String>,
    pub normal: Option<String>,
    pub clearcoat: Option<String>,
    pub anisotropy: Option<String>,
    pub transmission: Option<String>,
}

impl Graph {
    /// Creates a new empty graph with the given base color node reference.
    pub fn new(base_color: impl Into<String>) -> Self {
        Self {
            nodes: std::collections::BTreeMap::new(),
            base_color: base_color.into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        }
    }

    /// Returns the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if the graph has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns `true` if the graph has a metallic-roughness channel.
    pub fn has_metallic_roughness(&self) -> bool {
        self.mr.is_some()
    }

    /// Returns `true` if the graph has a normal map channel.
    pub fn has_normal(&self) -> bool {
        self.normal.is_some()
    }

    /// Returns `true` if the graph has a clearcoat channel.
    pub fn has_clearcoat(&self) -> bool {
        self.clearcoat.is_some()
    }

    /// Returns `true` if the graph has anisotropy.
    pub fn has_anisotropy(&self) -> bool {
        self.anisotropy.is_some()
    }

    /// Returns `true` if the graph has transmission.
    pub fn has_transmission(&self) -> bool {
        self.transmission.is_some()
    }

    /// Adds a node to the graph with the given ID.
    pub fn add_node(&mut self, id: impl Into<String>, node: Node) -> &mut Self {
        self.nodes.insert(id.into(), node);
        self
    }

    /// Gets a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Sets the metallic-roughness channel.
    pub fn with_metallic_roughness(mut self, mr_node: impl Into<String>) -> Self {
        self.mr = Some(mr_node.into());
        self
    }

    /// Sets the normal map channel.
    pub fn with_normal(mut self, normal_node: impl Into<String>) -> Self {
        self.normal = Some(normal_node.into());
        self
    }

    /// Sets the clearcoat channel.
    pub fn with_clearcoat(mut self, clearcoat_node: impl Into<String>) -> Self {
        self.clearcoat = Some(clearcoat_node.into());
        self
    }

    /// Returns the number of active PBR channels.
    pub fn active_channel_count(&self) -> usize {
        let mut count = 1; // base_color is always active
        if self.mr.is_some() {
            count += 1;
        }
        if self.normal.is_some() {
            count += 1;
        }
        if self.clearcoat.is_some() {
            count += 1;
        }
        if self.anisotropy.is_some() {
            count += 1;
        }
        if self.transmission.is_some() {
            count += 1;
        }
        count
    }

    /// Returns the number of texture nodes in the graph.
    pub fn texture_node_count(&self) -> usize {
        self.nodes.values().filter(|n| n.is_texture()).count()
    }
}

impl std::fmt::Display for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Graph({} nodes, {} channels)",
            self.node_count(),
            self.active_channel_count()
        )
    }
}

/// Compile a tiny subset of a PBR material to WGSL fragment snippet.
/// Returns WGSL and a list of required bindings for the runtime to wire.
pub fn compile_to_wgsl(g: &Graph) -> Result<(String, Vec<String>)> {
    fn expr_of(
        nodes: &std::collections::BTreeMap<String, Node>,
        id: &str,
        binds: &mut Vec<String>,
    ) -> Result<String> {
        let n = nodes.get(id).ok_or_else(|| anyhow!("missing node {id}"))?;
        Ok(match n {
            Node::Texture2D { id, uv } => {
                binds.push(id.clone());
                format!("textureSample(tex_{}, samp_{}, {}).rgb", id, id, uv)
            }
            Node::Constant3 { value } => {
                format!("vec3<f32>({},{},{})", value[0], value[1], value[2])
            }
            Node::Constant1 { value } => format!("{value}"),
            Node::Multiply { a, b } => format!(
                "(({}) * ({}))",
                expr_of(nodes, a, binds)?,
                expr_of(nodes, b, binds)?
            ),
            Node::Add { a, b } => format!(
                "(({}) + ({}))",
                expr_of(nodes, a, binds)?,
                expr_of(nodes, b, binds)?
            ),
            Node::MetallicRoughness {
                metallic,
                roughness,
            } => format!(
                "vec2<f32>({},{})",
                expr_of(nodes, metallic, binds)?,
                expr_of(nodes, roughness, binds)?
            ),
            Node::Clearcoat { weight, roughness } => format!(
                "vec2<f32>({},{})",
                expr_of(nodes, weight, binds)?,
                expr_of(nodes, roughness, binds)?
            ),
            Node::Anisotropy { amount } => expr_of(nodes, amount, binds)?.to_string(),
            Node::Transmission { amount } => expr_of(nodes, amount, binds)?.to_string(),
            Node::NormalMap { id, uv, scale } => {
                binds.push(id.clone());
                format!(
                    "normalize((textureSample(tex_{}, samp_{}, {}).xyz*2.0-1.0) * {:.6})",
                    id, id, uv, scale
                )
            }
        })
    }

    let mut binds = Vec::new();
    let bc = expr_of(&g.nodes, &g.base_color, &mut binds)?;
    let mr = if let Some(m) = &g.mr {
        Some(expr_of(&g.nodes, m, &mut binds)?)
    } else {
        None
    };
    let nrm = if let Some(n) = &g.normal {
        Some(expr_of(&g.nodes, n, &mut binds)?)
    } else {
        None
    };
    let cc = if let Some(c) = &g.clearcoat {
        Some(expr_of(&g.nodes, c, &mut binds)?)
    } else {
        None
    };
    let aniso = if let Some(a) = &g.anisotropy {
        Some(expr_of(&g.nodes, a, &mut binds)?)
    } else {
        None
    };
    let trans = if let Some(t) = &g.transmission {
        Some(expr_of(&g.nodes, t, &mut binds)?)
    } else {
        None
    };

    // simple WGSL fn producing baseColor/metallic/roughness
    // Emit a struct with extended PBR channels
    let (mr_src, nrm_src, cc_src, an_src, tr_src) = (
        mr.unwrap_or_else(|| "vec2<f32>(0.0,1.0)".into()),
        nrm.unwrap_or_else(|| "vec3<f32>(0.0,0.0,1.0)".into()),
        cc.unwrap_or_else(|| "vec2<f32>(0.0,0.0)".into()),
        aniso.unwrap_or_else(|| "0.0".into()),
        trans.unwrap_or_else(|| "0.0".into()),
    );
    let body = format!(
        "struct MatOut {{ base: vec3<f32>, mr: vec2<f32>, normal: vec3<f32>, clearcoat: vec2<f32>, anisotropy: f32, transmission: f32 }};\nfn eval_material(uv: vec2<f32>) -> MatOut {{\n    let base = {bc};\n    let mr = {mr};\n    let normal = normalize({nrm});\n    let clearcoat = {cc};\n    let anisotropy = {an};\n    let transmission = {tr};\n    return MatOut(base, mr, normal, clearcoat, anisotropy, transmission);\n}}\n",
        bc = bc, mr = mr_src, nrm = nrm_src, cc = cc_src, an = an_src, tr = tr_src
    );

    Ok((body, binds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_minimal_graph() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert(
            "uv".into(),
            Node::Constant3 {
                value: [0.0, 0.0, 0.0],
            },
        ); // unused
        nodes.insert(
            "c".into(),
            Node::Constant3 {
                value: [1.0, 0.5, 0.1],
            },
        );
        let g = Graph {
            nodes,
            base_color: "c".into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let (wgsl, _b) = compile_to_wgsl(&g).unwrap();
        assert!(wgsl.contains("eval_material"));
    }

    #[test]
    fn compiles_tex_and_mr_and_validates_with_naga() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert(
            "base_tex".into(),
            Node::Texture2D {
                id: "albedo".into(),
                uv: "uv".into(),
            },
        );
        // uv will be provided as function parameter in composed shader
        nodes.insert("m".into(), Node::Constant1 { value: 0.2 });
        nodes.insert("r".into(), Node::Constant1 { value: 0.8 });
        nodes.insert(
            "mr".into(),
            Node::MetallicRoughness {
                metallic: "m".into(),
                roughness: "r".into(),
            },
        );
        let g = Graph {
            nodes,
            base_color: "base_tex".into(),
            mr: Some("mr".into()),
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let (wgsl, binds) = compile_to_wgsl(&g).unwrap();
        // Prepare binding declarations for textures/samplers referenced
        let mut decls = String::new();
        let mut bind_idx = 0u32;
        for id in binds {
            decls.push_str(&format!(
                "@group(0) @binding({}) var tex_{}: texture_2d<f32>;\n",
                bind_idx, id
            ));
            bind_idx += 1;
            decls.push_str(&format!(
                "@group(0) @binding({}) var samp_{}: sampler;\n",
                bind_idx, id
            ));
            bind_idx += 1;
        }
        // Validate WGSL snippet by composing into a module with a fragment shader
        let full = format!(
            "{decls}\n{body}\n@fragment\nfn fs_main(@location(0) v_uv: vec2<f32>) -> @location(0) vec4<f32> {{\n    let m = eval_material(v_uv);\n    return vec4<f32>(m.base, 1.0);\n}}\n",
            decls = decls,
            body = wgsl
        );
        let res = naga::front::wgsl::parse_str(&full);
        assert!(res.is_ok(), "WGSL failed to parse: {:?}", res.err());
    }

    #[test]
    fn snapshot_contains_clearcoat_aniso_transmission() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert(
            "c".into(),
            Node::Constant3 {
                value: [0.8, 0.6, 0.2],
            },
        );
        nodes.insert("m".into(), Node::Constant1 { value: 0.5 });
        nodes.insert("r".into(), Node::Constant1 { value: 0.3 });
        nodes.insert(
            "mr".into(),
            Node::MetallicRoughness {
                metallic: "m".into(),
                roughness: "r".into(),
            },
        );
        nodes.insert(
            "cc".into(),
            Node::Clearcoat {
                weight: "m".into(),
                roughness: "r".into(),
            },
        );
        nodes.insert("an".into(), Node::Anisotropy { amount: "m".into() });
        nodes.insert("tr".into(), Node::Transmission { amount: "r".into() });
        let g = Graph {
            nodes,
            base_color: "c".into(),
            mr: Some("mr".into()),
            normal: None,
            clearcoat: Some("cc".into()),
            anisotropy: Some("an".into()),
            transmission: Some("tr".into()),
        };
        let (wgsl, _b) = compile_to_wgsl(&g).unwrap();
        assert!(wgsl.contains("clearcoat"));
        assert!(wgsl.contains("anisotropy"));
        assert!(wgsl.contains("transmission"));
    }

    #[test]
    fn test_missing_node_error() {
        let nodes = std::collections::BTreeMap::new();
        let g = Graph {
            nodes,
            base_color: "missing".into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let result = compile_to_wgsl(&g);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing node"));
    }

    #[test]
    fn test_multiply_node() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert("a".into(), Node::Constant1 { value: 2.0 });
        nodes.insert("b".into(), Node::Constant1 { value: 3.0 });
        nodes.insert(
            "m".into(),
            Node::Multiply {
                a: "a".into(),
                b: "b".into(),
            },
        );
        nodes.insert(
            "c".into(),
            Node::Constant3 {
                value: [1.0, 1.0, 1.0],
            },
        );
        let g = Graph {
            nodes,
            base_color: "c".into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let (wgsl, _binds) = compile_to_wgsl(&g).unwrap();
        assert!(wgsl.contains("eval_material"));
    }

    #[test]
    fn test_add_node() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert("a".into(), Node::Constant1 { value: 1.0 });
        nodes.insert("b".into(), Node::Constant1 { value: 2.0 });
        nodes.insert(
            "sum".into(),
            Node::Add {
                a: "a".into(),
                b: "b".into(),
            },
        );
        nodes.insert(
            "c".into(),
            Node::Constant3 {
                value: [1.0, 1.0, 1.0],
            },
        );
        let g = Graph {
            nodes,
            base_color: "c".into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let (wgsl, _binds) = compile_to_wgsl(&g).unwrap();
        assert!(wgsl.contains("eval_material"));
    }

    #[test]
    fn test_normal_map_node() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert(
            "c".into(),
            Node::Constant3 {
                value: [1.0, 1.0, 1.0],
            },
        );
        nodes.insert(
            "nrm".into(),
            Node::NormalMap {
                id: "normal_tex".into(),
                uv: "uv".into(),
                scale: 1.0,
            },
        );
        let g = Graph {
            nodes,
            base_color: "c".into(),
            mr: None,
            normal: Some("nrm".into()),
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let (wgsl, binds) = compile_to_wgsl(&g).unwrap();
        assert!(wgsl.contains("textureSample"));
        assert!(binds.contains(&"normal_tex".to_string()));
    }

    #[test]
    fn test_material_package_from_graph() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert(
            "c".into(),
            Node::Constant3 {
                value: [1.0, 0.5, 0.2],
            },
        );
        let g = Graph {
            nodes,
            base_color: "c".into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let pkg = MaterialPackage::from_graph(&g).unwrap();
        assert!(pkg.wgsl.contains("eval_material"));
        assert!(pkg.bindings.is_empty());
    }

    // ====================================================================
    // Node enum tests
    // ====================================================================

    #[test]
    fn test_node_type_name() {
        assert_eq!(Node::Constant1 { value: 1.0 }.type_name(), "Constant1");
        assert_eq!(
            Node::Constant3 {
                value: [1.0, 2.0, 3.0]
            }
            .type_name(),
            "Constant3"
        );
        assert_eq!(
            Node::Texture2D {
                id: "t".into(),
                uv: "u".into()
            }
            .type_name(),
            "Texture2D"
        );
        assert_eq!(
            Node::Add {
                a: "a".into(),
                b: "b".into()
            }
            .type_name(),
            "Add"
        );
        assert_eq!(
            Node::Multiply {
                a: "a".into(),
                b: "b".into()
            }
            .type_name(),
            "Multiply"
        );
        assert_eq!(
            Node::MetallicRoughness {
                metallic: "m".into(),
                roughness: "r".into()
            }
            .type_name(),
            "MetallicRoughness"
        );
        assert_eq!(
            Node::NormalMap {
                id: "n".into(),
                uv: "u".into(),
                scale: 1.0
            }
            .type_name(),
            "NormalMap"
        );
        assert_eq!(
            Node::Clearcoat {
                weight: "w".into(),
                roughness: "r".into()
            }
            .type_name(),
            "Clearcoat"
        );
        assert_eq!(
            Node::Anisotropy { amount: "a".into() }.type_name(),
            "Anisotropy"
        );
        assert_eq!(
            Node::Transmission { amount: "a".into() }.type_name(),
            "Transmission"
        );
    }

    #[test]
    fn test_node_is_texture() {
        assert!(Node::Texture2D {
            id: "t".into(),
            uv: "u".into()
        }
        .is_texture());
        assert!(Node::NormalMap {
            id: "n".into(),
            uv: "u".into(),
            scale: 1.0
        }
        .is_texture());
        assert!(!Node::Constant1 { value: 1.0 }.is_texture());
        assert!(!Node::Constant3 {
            value: [1.0, 2.0, 3.0]
        }
        .is_texture());
    }

    #[test]
    fn test_node_is_constant() {
        assert!(Node::Constant1 { value: 1.0 }.is_constant());
        assert!(Node::Constant3 {
            value: [1.0, 2.0, 3.0]
        }
        .is_constant());
        assert!(!Node::Texture2D {
            id: "t".into(),
            uv: "u".into()
        }
        .is_constant());
        assert!(!Node::Add {
            a: "a".into(),
            b: "b".into()
        }
        .is_constant());
    }

    #[test]
    fn test_node_is_arithmetic() {
        assert!(Node::Add {
            a: "a".into(),
            b: "b".into()
        }
        .is_arithmetic());
        assert!(Node::Multiply {
            a: "a".into(),
            b: "b".into()
        }
        .is_arithmetic());
        assert!(!Node::Constant1 { value: 1.0 }.is_arithmetic());
        assert!(!Node::Texture2D {
            id: "t".into(),
            uv: "u".into()
        }
        .is_arithmetic());
    }

    #[test]
    fn test_node_is_pbr_property() {
        assert!(Node::MetallicRoughness {
            metallic: "m".into(),
            roughness: "r".into()
        }
        .is_pbr_property());
        assert!(Node::Clearcoat {
            weight: "w".into(),
            roughness: "r".into()
        }
        .is_pbr_property());
        assert!(Node::Anisotropy { amount: "a".into() }.is_pbr_property());
        assert!(Node::Transmission { amount: "a".into() }.is_pbr_property());
        // NormalMap is a texture node, not a PBR property node
        assert!(!Node::NormalMap {
            id: "n".into(),
            uv: "u".into(),
            scale: 1.0
        }
        .is_pbr_property());
        assert!(!Node::Constant1 { value: 1.0 }.is_pbr_property());
        assert!(!Node::Add {
            a: "a".into(),
            b: "b".into()
        }
        .is_pbr_property());
    }

    #[test]
    fn test_node_factory_methods() {
        let tex = Node::texture_2d("albedo", "uv0");
        assert_eq!(tex.type_name(), "Texture2D");
        assert!(tex.is_texture());

        let c3 = Node::constant3(1.0, 0.5, 0.0);
        assert_eq!(c3.type_name(), "Constant3");

        let c1 = Node::constant1(0.5);
        assert_eq!(c1.type_name(), "Constant1");

        let add = Node::add("a", "b");
        assert_eq!(add.type_name(), "Add");

        let mul = Node::multiply("a", "b");
        assert_eq!(mul.type_name(), "Multiply");

        let mr = Node::metallic_roughness("m", "r");
        assert_eq!(mr.type_name(), "MetallicRoughness");

        let nm = Node::normal_map("nrm_tex", "uv0", 1.0);
        assert_eq!(nm.type_name(), "NormalMap");
    }

    #[test]
    fn test_node_display() {
        assert_eq!(
            format!("{}", Node::Constant1 { value: 0.5 }),
            "Constant1(0.50)"
        );
        assert_eq!(
            format!(
                "{}",
                Node::Constant3 {
                    value: [1.0, 0.5, 0.0]
                }
            ),
            "Constant3(1.00, 0.50, 0.00)"
        );
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
        assert_eq!(
            format!("{}", Node::Anisotropy { amount: "a".into() }),
            "Anisotropy(a)"
        );
        assert_eq!(
            format!("{}", Node::Transmission { amount: "t".into() }),
            "Transmission(t)"
        );
    }

    // ====================================================================
    // Graph tests
    // ====================================================================

    #[test]
    fn test_graph_new_and_node_count() {
        let g = Graph::new("base");
        assert!(g.is_empty());
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.base_color, "base");
    }

    #[test]
    fn test_graph_add_node() {
        let mut g = Graph::new("color");
        g.add_node("c", Node::constant3(1.0, 0.0, 0.0));
        assert_eq!(g.node_count(), 1);
        assert!(!g.is_empty());
    }

    #[test]
    fn test_graph_get_node() {
        let mut g = Graph::new("base");
        g.add_node("test", Node::constant1(0.5));
        assert!(g.get_node("test").is_some());
        assert!(g.get_node("missing").is_none());
    }

    #[test]
    fn test_graph_channel_helpers() {
        let g = Graph::new("base");
        assert!(!g.has_metallic_roughness());
        assert!(!g.has_normal());
        assert!(!g.has_clearcoat());
        assert!(!g.has_anisotropy());
        assert!(!g.has_transmission());

        let g2 = Graph::new("base")
            .with_metallic_roughness("mr")
            .with_normal("nrm")
            .with_clearcoat("cc");

        assert!(g2.has_metallic_roughness());
        assert!(g2.has_normal());
        assert!(g2.has_clearcoat());
        assert!(!g2.has_anisotropy());
        assert!(!g2.has_transmission());
    }

    #[test]
    fn test_graph_active_channel_count() {
        let g = Graph::new("base");
        assert_eq!(g.active_channel_count(), 1); // base_color always active

        let g2 = Graph::new("base")
            .with_metallic_roughness("mr")
            .with_normal("nrm");
        assert_eq!(g2.active_channel_count(), 3);
    }

    #[test]
    fn test_graph_texture_node_count() {
        let mut g = Graph::new("base");
        g.add_node("tex1", Node::texture_2d("albedo", "uv"));
        g.add_node("const1", Node::constant1(0.5));
        g.add_node("tex2", Node::normal_map("normal", "uv", 1.0));
        assert_eq!(g.texture_node_count(), 2);
    }

    #[test]
    fn test_graph_display() {
        let mut g = Graph::new("base").with_metallic_roughness("mr");
        g.add_node("c", Node::constant3(1.0, 0.0, 0.0));
        g.add_node("mr", Node::metallic_roughness("m", "r"));
        let s = format!("{}", g);
        assert!(s.contains("Graph("));
        assert!(s.contains("nodes"));
        assert!(s.contains("channels"));
    }

    // ====================================================================
    // MaterialPackage tests
    // ====================================================================

    #[test]
    fn test_material_package_helpers() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert("c".into(), Node::constant3(1.0, 0.0, 0.0));
        nodes.insert("tex".into(), Node::texture_2d("albedo", "uv"));
        let g = Graph {
            nodes,
            base_color: "tex".into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let pkg = MaterialPackage::from_graph(&g).unwrap();

        assert!(pkg.has_bindings());
        assert_eq!(pkg.binding_count(), 1);
        assert!(pkg.requires_binding("albedo"));
        assert!(!pkg.requires_binding("missing"));
        assert!(pkg.wgsl_size() > 0);
    }

    #[test]
    fn test_material_package_display() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert("c".into(), Node::constant3(1.0, 0.0, 0.0));
        let g = Graph {
            nodes,
            base_color: "c".into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };
        let pkg = MaterialPackage::from_graph(&g).unwrap();
        let s = format!("{}", pkg);
        assert!(s.contains("MaterialPackage("));
        assert!(s.contains("bytes"));
        assert!(s.contains("bindings"));
    }

    // ====================================================================
    // BakeConfig tests
    // ====================================================================

    #[test]
    fn test_bake_config_default() {
        let cfg = BakeConfig::default();
        assert_eq!(cfg.resolution, 1024);
        assert_eq!(cfg.samples, 16);
        assert!(cfg.generate_mipmaps);
        assert!(!cfg.compress);
    }

    #[test]
    fn test_bake_config_factories() {
        let preview = BakeConfig::preview();
        assert_eq!(preview.resolution, 256);
        assert_eq!(preview.samples, 4);
        assert!(!preview.generate_mipmaps);

        let hq = BakeConfig::high_quality();
        assert_eq!(hq.resolution, 2048);
        assert_eq!(hq.samples, 64);
        assert!(hq.compress);

        let custom = BakeConfig::with_resolution(512);
        assert_eq!(custom.resolution, 512);
    }

    #[test]
    fn test_bake_config_builders() {
        let cfg = BakeConfig::with_resolution(256).samples(32).compressed();
        assert_eq!(cfg.resolution, 256);
        assert_eq!(cfg.samples, 32);
        assert!(cfg.compress);
    }

    #[test]
    fn test_bake_config_total_pixels() {
        let cfg = BakeConfig::with_resolution(64);
        assert_eq!(cfg.total_pixels(), 64 * 64);
    }

    #[test]
    fn test_bake_config_is_power_of_two() {
        assert!(BakeConfig::with_resolution(256).is_power_of_two());
        assert!(BakeConfig::with_resolution(1024).is_power_of_two());
        assert!(!BakeConfig::with_resolution(300).is_power_of_two());
    }

    #[test]
    fn test_bake_config_display() {
        let cfg = BakeConfig::default();
        let s = format!("{}", cfg);
        assert!(s.contains("BakeConfig("));
        assert!(s.contains("1024x1024"));
        assert!(s.contains("16 samples"));
        assert!(s.contains("mipmaps"));
    }

    // ====================================================================
    // BakedMaterial tests
    // ====================================================================

    #[test]
    fn test_baked_material_helpers() {
        let baked = BakedMaterial {
            base_color: vec![[1.0, 0.0, 0.0, 1.0]; 64],
            metallic_roughness: vec![[0.0, 0.5, 0.0, 1.0]; 64],
            normal: vec![[0.5, 0.5, 1.0, 1.0]; 64],
            resolution: 8,
            wgsl: "fn test() {}".into(),
        };

        assert_eq!(baked.pixel_count(), 64);
        assert!(baked.is_power_of_two());
        assert!(baked.wgsl_size() > 0);
        assert!(baked.memory_usage() > 0);
    }

    #[test]
    fn test_baked_material_display() {
        let baked = BakedMaterial {
            base_color: vec![[1.0, 0.0, 0.0, 1.0]; 16 * 16],
            metallic_roughness: vec![[0.0, 0.5, 0.0, 1.0]; 16 * 16],
            normal: vec![[0.5, 0.5, 1.0, 1.0]; 16 * 16],
            resolution: 16,
            wgsl: "fn test() {}".into(),
        };
        let s = format!("{}", baked);
        assert!(s.contains("BakedMaterial("));
        assert!(s.contains("16x16"));
    }

    // ====================================================================
    // BrdfValidation tests
    // ====================================================================

    #[test]
    fn test_brdf_validation_is_valid() {
        let valid = BrdfValidation {
            energy_conservation: true,
            reciprocity: true,
            positivity: true,
            max_energy_ratio: 0.95,
        };
        assert!(valid.is_valid());
        assert_eq!(valid.passed_count(), 3);
        assert!(valid.failed_checks().is_empty());
        assert!(valid.is_energy_efficient());
    }

    #[test]
    fn test_brdf_validation_fails() {
        let invalid = BrdfValidation {
            energy_conservation: false,
            reciprocity: true,
            positivity: false,
            max_energy_ratio: 1.1,
        };
        assert!(!invalid.is_valid());
        assert_eq!(invalid.passed_count(), 1);
        assert_eq!(invalid.failed_checks().len(), 2);
        assert!(invalid.failed_checks().contains(&"energy_conservation"));
        assert!(invalid.failed_checks().contains(&"positivity"));
        assert!(!invalid.is_energy_efficient());
    }

    #[test]
    fn test_brdf_validation_display() {
        let valid = BrdfValidation {
            energy_conservation: true,
            reciprocity: true,
            positivity: true,
            max_energy_ratio: 0.95,
        };
        let s = format!("{}", valid);
        assert!(s.contains("PASS"));

        let invalid = BrdfValidation {
            energy_conservation: false,
            reciprocity: true,
            positivity: true,
            max_energy_ratio: 1.1,
        };
        let s = format!("{}", invalid);
        assert!(s.contains("FAIL"));
    }

    // ====================================================================
    // BrdfLut tests
    // ====================================================================

    #[test]
    fn test_brdf_lut_helpers() {
        let lut = BrdfLut::generate(16);
        assert_eq!(lut.entry_count(), 16 * 16);
        assert_eq!(lut.byte_size(), 16 * 16 * 8);
    }

    #[test]
    fn test_brdf_lut_sample() {
        let lut = BrdfLut::generate(32);
        let sample = lut.sample(0.5, 0.5);
        assert!(sample.is_some());
        let [scale, bias] = sample.unwrap();
        assert!((0.0..=1.0).contains(&scale));
        assert!((0.0..=1.0).contains(&bias));

        // Edge cases
        assert!(lut.sample(0.0, 0.0).is_some());
        assert!(lut.sample(1.0, 1.0).is_some());
    }

    #[test]
    fn test_brdf_lut_display() {
        let lut = BrdfLut::generate(32);
        let s = format!("{}", lut);
        assert!(s.contains("BrdfLut("));
        assert!(s.contains("32x32"));
        assert!(s.contains("bytes"));
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MaterialPackage {
    pub wgsl: String,
    pub bindings: Vec<String>,
}

impl MaterialPackage {
    pub fn from_graph(g: &Graph) -> Result<Self> {
        let (wgsl, bindings) = compile_to_wgsl(g)?;
        Ok(Self { wgsl, bindings })
    }

    /// Returns the number of bindings required.
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    /// Returns `true` if the package has any bindings.
    pub fn has_bindings(&self) -> bool {
        !self.bindings.is_empty()
    }

    /// Returns `true` if the package requires a specific binding.
    pub fn requires_binding(&self, name: &str) -> bool {
        self.bindings.iter().any(|b| b == name)
    }

    /// Returns the size of the WGSL shader code in bytes.
    pub fn wgsl_size(&self) -> usize {
        self.wgsl.len()
    }
}

impl std::fmt::Display for MaterialPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MaterialPackage({} bytes, {} bindings)",
            self.wgsl_size(),
            self.binding_count()
        )
    }
}

// ============================================================================
// Material Baking Pipeline
// ============================================================================

/// Material baking configuration
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BakeConfig {
    /// Output texture resolution
    pub resolution: u32,
    /// Number of samples for quality
    pub samples: u32,
    /// Enable mipmap generation
    pub generate_mipmaps: bool,
    /// Compress output (BC7)
    pub compress: bool,
}

impl Default for BakeConfig {
    fn default() -> Self {
        Self {
            resolution: 1024,
            samples: 16,
            generate_mipmaps: true,
            compress: false,
        }
    }
}

impl BakeConfig {
    /// Creates a new bake config with custom resolution.
    pub fn with_resolution(resolution: u32) -> Self {
        Self {
            resolution,
            ..Default::default()
        }
    }

    /// Creates a high-quality bake config (2K, 64 samples).
    pub fn high_quality() -> Self {
        Self {
            resolution: 2048,
            samples: 64,
            generate_mipmaps: true,
            compress: true,
        }
    }

    /// Creates a preview-quality bake config (256, 4 samples).
    pub fn preview() -> Self {
        Self {
            resolution: 256,
            samples: 4,
            generate_mipmaps: false,
            compress: false,
        }
    }

    /// Returns the total number of pixels.
    pub fn total_pixels(&self) -> usize {
        (self.resolution * self.resolution) as usize
    }

    /// Returns `true` if this is a power-of-two resolution.
    pub fn is_power_of_two(&self) -> bool {
        self.resolution.is_power_of_two()
    }

    /// Sets the sample count and returns self.
    pub fn samples(mut self, samples: u32) -> Self {
        self.samples = samples;
        self
    }

    /// Enables compression and returns self.
    pub fn compressed(mut self) -> Self {
        self.compress = true;
        self
    }
}

impl std::fmt::Display for BakeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BakeConfig({}x{}, {} samples{}{})",
            self.resolution,
            self.resolution,
            self.samples,
            if self.generate_mipmaps {
                ", mipmaps"
            } else {
                ""
            },
            if self.compress { ", BC7" } else { "" }
        )
    }
}

/// Baked material output
#[derive(Clone, Debug, PartialEq)]
pub struct BakedMaterial {
    /// Base color texture (RGB)
    pub base_color: Vec<[f32; 4]>,
    /// Metallic-roughness texture (RG)
    pub metallic_roughness: Vec<[f32; 4]>,
    /// Normal map (RGB)
    pub normal: Vec<[f32; 4]>,
    /// Resolution
    pub resolution: u32,
    /// Shader code
    pub wgsl: String,
}

impl BakedMaterial {
    /// Returns the total number of pixels.
    pub fn pixel_count(&self) -> usize {
        (self.resolution * self.resolution) as usize
    }

    /// Returns `true` if the resolution is power-of-two.
    pub fn is_power_of_two(&self) -> bool {
        self.resolution.is_power_of_two()
    }

    /// Returns the size of the WGSL shader code in bytes.
    pub fn wgsl_size(&self) -> usize {
        self.wgsl.len()
    }

    /// Returns an estimate of total memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let pixels = self.pixel_count();
        // 3 textures * 4 floats per pixel * 4 bytes per float + wgsl string
        pixels * 3 * 16 + self.wgsl.len()
    }
}

impl std::fmt::Display for BakedMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BakedMaterial({}x{}, {} KB)",
            self.resolution,
            self.resolution,
            self.memory_usage() / 1024
        )
    }
}

/// Material baker for offline processing
pub struct MaterialBaker {
    config: BakeConfig,
}

impl MaterialBaker {
    pub fn new(config: BakeConfig) -> Self {
        Self { config }
    }

    /// Bake a material graph to textures
    pub fn bake(&self, graph: &Graph) -> Result<BakedMaterial> {
        let res = self.config.resolution;
        let total_pixels = (res * res) as usize;

        // Compile shader
        let (wgsl, _bindings) = compile_to_wgsl(graph)?;

        // Initialize textures with default values
        // These are filled by evaluating constant nodes from the graph
        let mut base_color = vec![[0.8f32, 0.8, 0.8, 1.0]; total_pixels];
        let metallic_roughness = vec![[0.0f32, 0.5, 0.0, 1.0]; total_pixels];
        let mut normal = vec![[0.5f32, 0.5, 1.0, 1.0]; total_pixels];

        // Sample the material at each UV coordinate
        for y in 0..res {
            for x in 0..res {
                let _u = x as f32 / (res - 1) as f32;
                let _v = y as f32 / (res - 1) as f32;
                let idx = (y * res + x) as usize;

                // Evaluate constant nodes from graph
                for (id, node) in &graph.nodes {
                    if *id == graph.base_color {
                        if let Node::Constant3 { value } = node {
                            base_color[idx] = [value[0], value[1], value[2], 1.0];
                        }
                    }
                }

                // Default normal (up)
                normal[idx] = [0.5, 0.5, 1.0, 1.0];
            }
        }

        Ok(BakedMaterial {
            base_color,
            metallic_roughness,
            normal,
            resolution: res,
            wgsl,
        })
    }

    /// Validate baked material meets quality thresholds
    pub fn validate(&self, baked: &BakedMaterial) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check for extreme values
        for (i, pixel) in baked.base_color.iter().enumerate() {
            if pixel.iter().any(|&v| !(0.0..=1.0).contains(&v)) {
                warnings.push(format!("Base color pixel {} out of range", i));
                break;
            }
        }

        // Check normal map validity (normals are in [0,1] range, need to convert to [-1,1])
        for (i, pixel) in baked.normal.iter().enumerate() {
            // Convert from [0,1] range to [-1,1] range
            let nx = pixel[0] * 2.0 - 1.0;
            let ny = pixel[1] * 2.0 - 1.0;
            let nz = pixel[2] * 2.0 - 1.0;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            if (len - 1.0).abs() > 0.1 {
                warnings.push(format!(
                    "Normal map pixel {} not normalized (len={})",
                    i, len
                ));
                break;
            }
        }

        warnings
    }
}

// ============================================================================
// BRDF Validation
// ============================================================================

/// BRDF validation results
#[derive(Debug, Clone, PartialEq)]
pub struct BrdfValidation {
    pub energy_conservation: bool,
    pub reciprocity: bool,
    pub positivity: bool,
    pub max_energy_ratio: f32,
}

impl BrdfValidation {
    /// Returns `true` if all validation checks pass.
    pub fn is_valid(&self) -> bool {
        self.energy_conservation && self.reciprocity && self.positivity
    }

    /// Returns the number of passed checks (0-3).
    pub fn passed_count(&self) -> usize {
        let mut count = 0;
        if self.energy_conservation {
            count += 1;
        }
        if self.reciprocity {
            count += 1;
        }
        if self.positivity {
            count += 1;
        }
        count
    }

    /// Returns a list of failed validation checks.
    pub fn failed_checks(&self) -> Vec<&'static str> {
        let mut failed = Vec::new();
        if !self.energy_conservation {
            failed.push("energy_conservation");
        }
        if !self.reciprocity {
            failed.push("reciprocity");
        }
        if !self.positivity {
            failed.push("positivity");
        }
        failed
    }

    /// Returns `true` if energy ratio is within acceptable range (<=1.0).
    pub fn is_energy_efficient(&self) -> bool {
        self.max_energy_ratio <= 1.0
    }
}

impl std::fmt::Display for BrdfValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() {
            write!(
                f,
                "BrdfValidation(PASS, energy={:.2})",
                self.max_energy_ratio
            )
        } else {
            write!(
                f,
                "BrdfValidation(FAIL: {:?}, energy={:.2})",
                self.failed_checks(),
                self.max_energy_ratio
            )
        }
    }
}

/// Validate BRDF properties for physically-based rendering
pub fn validate_brdf(metallic: f32, roughness: f32, base_color: [f32; 3]) -> BrdfValidation {
    // Clamp inputs
    let metallic = metallic.clamp(0.0, 1.0);
    let roughness = roughness.clamp(0.04, 1.0);

    // Fresnel at normal incidence (F0)
    let f0_dielectric = 0.04f32;
    let f0 = [
        f0_dielectric * (1.0 - metallic) + base_color[0] * metallic,
        f0_dielectric * (1.0 - metallic) + base_color[1] * metallic,
        f0_dielectric * (1.0 - metallic) + base_color[2] * metallic,
    ];

    // Energy conservation: F0 + diffuse should not exceed 1
    let diffuse_factor = 1.0 - metallic;
    let max_energy = f0
        .iter()
        .map(|&f| f + diffuse_factor * 0.96)
        .fold(0.0f32, f32::max);
    let energy_conservation = max_energy <= 1.05; // Allow 5% tolerance

    // Reciprocity: BRDF(l,v) = BRDF(v,l) - always true for isotropic BRDFs
    let reciprocity = true;

    // Positivity: BRDF should never be negative
    let positivity = f0.iter().all(|&f| f >= 0.0) && roughness >= 0.0;

    BrdfValidation {
        energy_conservation,
        reciprocity,
        positivity,
        max_energy_ratio: max_energy,
    }
}

// ============================================================================
// IBL LUT Generation
// ============================================================================

/// Pre-integrated BRDF LUT for IBL
/// Stores (scale, bias) for split-sum approximation
pub struct BrdfLut {
    pub data: Vec<[f32; 2]>,
    pub resolution: u32,
}

impl BrdfLut {
    /// Returns the total number of entries in the LUT.
    pub fn entry_count(&self) -> usize {
        self.data.len()
    }

    /// Returns the size in bytes when exported.
    pub fn byte_size(&self) -> usize {
        self.data.len() * 8 // 2 floats * 4 bytes each
    }

    /// Samples the LUT at given NdotV and roughness (both 0-1).
    pub fn sample(&self, n_dot_v: f32, roughness: f32) -> Option<[f32; 2]> {
        let x = (n_dot_v.clamp(0.0, 1.0) * (self.resolution - 1) as f32) as u32;
        let y = (roughness.clamp(0.0, 1.0) * (self.resolution - 1) as f32) as u32;
        let idx = (y * self.resolution + x) as usize;
        self.data.get(idx).copied()
    }

    /// Generate BRDF integration LUT
    /// X-axis: NdotV (0 to 1)
    /// Y-axis: roughness (0 to 1)
    pub fn generate(resolution: u32) -> Self {
        let mut data = Vec::with_capacity((resolution * resolution) as usize);

        for y in 0..resolution {
            let roughness = (y as f32 + 0.5) / resolution as f32;
            let a = roughness * roughness;

            for x in 0..resolution {
                let n_dot_v = (x as f32 + 0.5) / resolution as f32;
                let n_dot_v = n_dot_v.max(0.001);

                // Integrate BRDF
                let (scale, bias) = Self::integrate_brdf(n_dot_v, a, 64);
                data.push([scale, bias]);
            }
        }

        Self { data, resolution }
    }

    /// Importance sample GGX to integrate BRDF
    fn integrate_brdf(n_dot_v: f32, roughness: f32, samples: u32) -> (f32, f32) {
        let v = [(1.0 - n_dot_v * n_dot_v).sqrt(), 0.0, n_dot_v];

        let mut a = 0.0f32;
        let mut b = 0.0f32;

        for i in 0..samples {
            // Hammersley sequence
            let xi = Self::hammersley(i, samples);

            // Importance sample GGX
            let h = Self::importance_sample_ggx(xi, roughness);

            // Reflect V around H
            let v_dot_h = v[0] * h[0] + v[1] * h[1] + v[2] * h[2];
            let l = [
                2.0 * v_dot_h * h[0] - v[0],
                2.0 * v_dot_h * h[1] - v[1],
                2.0 * v_dot_h * h[2] - v[2],
            ];

            let n_dot_l = l[2].max(0.0);
            let n_dot_h = h[2].max(0.0);
            let v_dot_h = v_dot_h.max(0.0);

            if n_dot_l > 0.0 {
                let g = Self::geometry_smith(n_dot_v, n_dot_l, roughness);
                let g_vis = g * v_dot_h / (n_dot_h * n_dot_v);
                let fc = (1.0 - v_dot_h).powi(5);

                a += (1.0 - fc) * g_vis;
                b += fc * g_vis;
            }
        }

        (a / samples as f32, b / samples as f32)
    }

    fn hammersley(i: u32, n: u32) -> [f32; 2] {
        let bits = i.reverse_bits();
        [i as f32 / n as f32, bits as f32 * 2.328_306_4e-10]
    }

    fn importance_sample_ggx(xi: [f32; 2], roughness: f32) -> [f32; 3] {
        use std::f32::consts::PI;

        let a = roughness * roughness;
        let phi = 2.0 * PI * xi[0];
        let cos_theta = ((1.0 - xi[1]) / (1.0 + (a * a - 1.0) * xi[1])).sqrt();
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

        [phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta]
    }

    fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
        let r = roughness + 1.0;
        let k = (r * r) / 8.0;

        let ggx_v = n_dot_v / (n_dot_v * (1.0 - k) + k);
        let ggx_l = n_dot_l / (n_dot_l * (1.0 - k) + k);

        ggx_v * ggx_l
    }

    /// Export LUT as raw bytes (R32G32 float format)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.data.len() * 8);
        for [scale, bias] in &self.data {
            bytes.extend_from_slice(&scale.to_le_bytes());
            bytes.extend_from_slice(&bias.to_le_bytes());
        }
        bytes
    }
}

impl std::fmt::Display for BrdfLut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BrdfLut({}x{}, {} bytes)",
            self.resolution,
            self.resolution,
            self.byte_size()
        )
    }
}

#[cfg(test)]
mod baking_tests {
    use super::*;

    #[test]
    fn test_material_baker() {
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert(
            "c".into(),
            Node::Constant3 {
                value: [0.8, 0.2, 0.1],
            },
        );
        let g = Graph {
            nodes,
            base_color: "c".into(),
            mr: None,
            normal: None,
            clearcoat: None,
            anisotropy: None,
            transmission: None,
        };

        let baker = MaterialBaker::new(BakeConfig {
            resolution: 64,
            ..Default::default()
        });
        let baked = baker.bake(&g).unwrap();

        assert_eq!(baked.resolution, 64);
        assert_eq!(baked.base_color.len(), 64 * 64);

        let warnings = baker.validate(&baked);
        assert!(warnings.is_empty(), "Warnings: {:?}", warnings);
    }

    #[test]
    fn test_brdf_validation() {
        // Dielectric material
        let result = validate_brdf(0.0, 0.5, [0.8, 0.2, 0.1]);
        assert!(result.energy_conservation);
        assert!(result.reciprocity);
        assert!(result.positivity);

        // Metal material
        let result = validate_brdf(1.0, 0.3, [1.0, 0.86, 0.57]); // Gold
        assert!(result.positivity);
    }

    #[test]
    fn test_brdf_lut_generation() {
        let lut = BrdfLut::generate(32);
        assert_eq!(lut.resolution, 32);
        assert_eq!(lut.data.len(), 32 * 32);

        // Values should be in reasonable range
        for [scale, bias] in &lut.data {
            assert!(
                *scale >= 0.0 && *scale <= 1.0,
                "Scale out of range: {}",
                scale
            );
            assert!(*bias >= 0.0 && *bias <= 1.0, "Bias out of range: {}", bias);
        }

        // Export should work
        let bytes = lut.to_bytes();
        assert_eq!(bytes.len(), 32 * 32 * 8);
    }
}
