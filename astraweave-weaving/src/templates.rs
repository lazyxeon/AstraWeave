//! Closed weave templates. A cast names an id. The tick does not build a graph.

use crate::field::{ModifierKind, SimModifier, MAT_ICE, MAT_WATER, MAT_WOOD};

pub const TEMPLATE_HEAT: u16 = 1;
pub const TEMPLATE_FREEZE: u16 = 2;
pub const TEMPLATE_DENSITY: u16 = 3;
pub const TEMPLATE_GRAVITY: u16 = 4;
pub const TEMPLATE_WIND: u16 = 5;
pub const TEMPLATE_LOWER_WATER: u16 = 6;
pub const TEMPLATE_DECAY: u16 = 7;

pub const MAX_NODES: usize = 8;
pub const MAX_TEMPLATES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node {
    pub kind: ModifierKind,
    pub radius: u16,
    pub a: i16,
    pub b: i16,
    pub c: i16,
}

#[derive(Clone, Copy, Debug)]
pub struct Template {
    pub id: u16,
    pub nodes: [Node; MAX_NODES],
    pub node_count: u8,
    /// Ice sheet half-extent in cells. Zero means no sheet.
    pub ice_radius: u8,
}

impl Template {
    pub const fn vacant(id: u16) -> Self {
        Self::empty(id)
    }

    const fn empty(id: u16) -> Self {
        Self {
            id,
            nodes: [Node {
                kind: ModifierKind::HeatSource,
                radius: 0,
                a: 0,
                b: 0,
                c: 0,
            }; MAX_NODES],
            node_count: 0,
            ice_radius: 0,
        }
    }
}

pub fn builtin(id: u16) -> Option<Template> {
    let mut template = Template::empty(id);
    match id {
        TEMPLATE_HEAT => {
            template.nodes[0] = Node {
                kind: ModifierKind::MaterialStamp,
                radius: 0,
                a: MAT_WOOD as i16,
                b: 0,
                c: 0,
            };
            template.nodes[1] = Node {
                kind: ModifierKind::HeatSource,
                radius: 0,
                a: 400,
                b: 0,
                c: 0,
            };
            template.node_count = 2;
        }
        TEMPLATE_FREEZE => {
            template.nodes[0] = Node {
                kind: ModifierKind::MaterialStamp,
                radius: 2,
                a: MAT_ICE as i16,
                b: 0,
                c: 0,
            };
            template.nodes[1] = Node {
                kind: ModifierKind::HeatSource,
                radius: 2,
                a: -200,
                b: 0,
                c: 0,
            };
            template.node_count = 2;
            template.ice_radius = 2;
        }
        TEMPLATE_DENSITY => {
            template.nodes[0] = Node {
                kind: ModifierKind::DensityScale,
                radius: 1,
                a: 400,
                b: 0,
                c: 0,
            };
            template.node_count = 1;
        }
        TEMPLATE_GRAVITY => {
            // +1000 milli-g cancels the world -1000, which is zero-g.
            template.nodes[0] = Node {
                kind: ModifierKind::GravityDelta,
                radius: 2,
                a: 1000,
                b: 0,
                c: 0,
            };
            template.node_count = 1;
        }
        TEMPLATE_WIND => {
            template.nodes[0] = Node {
                kind: ModifierKind::Wind,
                radius: 0,
                a: 1000,
                b: 0,
                c: 400,
            };
            template.node_count = 1;
        }
        TEMPLATE_LOWER_WATER => {
            template.nodes[0] = Node {
                kind: ModifierKind::FillStamp,
                radius: 2,
                a: MAT_WATER as i16,
                b: 0,
                c: 0,
            };
            template.node_count = 1;
        }
        TEMPLATE_DECAY => {
            template.nodes[0] = Node {
                kind: ModifierKind::DensityScale,
                radius: 1,
                a: 50,
                b: 0,
                c: 0,
            };
            template.node_count = 1;
        }
        _ => return None,
    }
    Some(template)
}

pub fn modifiers_for(
    template: &Template,
    origin: [i32; 3],
    id_base: u32,
) -> ([SimModifier; MAX_NODES], u8) {
    let mut out = [SimModifier {
        id: 0,
        kind: ModifierKind::HeatSource,
        origin,
        radius: 0,
        a: 0,
        b: 0,
        c: 0,
    }; MAX_NODES];
    let count = template.node_count as usize;
    for (i, (slot, node)) in out
        .iter_mut()
        .zip(template.nodes.iter())
        .take(count)
        .enumerate()
    {
        *slot = SimModifier {
            id: id_base.saturating_add(i as u32),
            kind: node.kind,
            origin,
            radius: node.radius,
            a: node.a,
            b: node.b,
            c: node.c,
        };
    }
    (out, template.node_count)
}

/// TOML loader for extra templates. Unknown keys fail. This runs at load, not in the tick.
pub fn parse_template_toml(text: &str) -> Result<Template, String> {
    let value: toml::Value = toml::from_str(text).map_err(|err| err.to_string())?;
    let id = value
        .get("id")
        .and_then(|v| v.as_integer())
        .ok_or_else(|| "template id".to_string())? as u16;
    if builtin(id).is_some() {
        return Err(format!("template {id} is an engine id"));
    }
    let mut template = Template::empty(id);
    let nodes = value
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "nodes".to_string())?;
    if nodes.len() > MAX_NODES {
        return Err("too many nodes".to_string());
    }
    for (i, node) in nodes.iter().enumerate() {
        let kind = node
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "kind".to_string())?;
        let kind = match kind {
            "heat" => ModifierKind::HeatSource,
            "wind" => ModifierKind::Wind,
            "gravity" => ModifierKind::GravityDelta,
            "material" => ModifierKind::MaterialStamp,
            "fill" => ModifierKind::FillStamp,
            "density" => ModifierKind::DensityScale,
            _ => return Err(format!("unknown kind {kind}")),
        };
        let radius = node.get("radius").and_then(|v| v.as_integer()).unwrap_or(0) as u16;
        let a = node.get("a").and_then(|v| v.as_integer()).unwrap_or(0) as i16;
        let b = node.get("b").and_then(|v| v.as_integer()).unwrap_or(0) as i16;
        let c = node.get("c").and_then(|v| v.as_integer()).unwrap_or(0) as i16;
        template.nodes[i] = Node {
            kind,
            radius,
            a,
            b,
            c,
        };
    }
    template.node_count = nodes.len() as u8;
    template.ice_radius = value
        .get("ice_radius")
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u8;
    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_closed() {
        assert!(builtin(0).is_none());
        assert_eq!(builtin(TEMPLATE_FREEZE).unwrap().ice_radius, 2);
        assert!(builtin(TEMPLATE_HEAT).unwrap().node_count >= 1);
    }

    #[test]
    fn toml_rejects_engine_id() {
        let text = "id = 1\nnodes = []\n";
        assert!(parse_template_toml(text).is_err());
    }

    #[test]
    fn toml_loads_a_density_row() {
        let text = r#"
id = 40
ice_radius = 0
nodes = [{ kind = "density", radius = 1, a = 120 }]
"#;
        let template = parse_template_toml(text).unwrap();
        assert_eq!(template.id, 40);
        assert_eq!(template.node_count, 1);
        assert_eq!(template.nodes[0].a, 120);
    }
}
