//! Closed weave templates. A cast names an id. The tick emits [`FieldWrite`]s.

use crate::field::{FieldWrite, Shape, WriteOp, FLAG_ICE, PHASE_ICE};

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
    pub shape: Shape,
    pub radius: u8,
    pub op: WriteOp,
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
                shape: Shape::Point,
                radius: 0,
                op: WriteOp::AddTemp {
                    delta: 0,
                    stack: false,
                },
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
                shape: Shape::Point,
                radius: 0,
                op: WriteOp::AddTemp {
                    delta: 400,
                    stack: false,
                },
            };
            template.node_count = 1;
        }
        TEMPLATE_FREEZE => {
            template.nodes[0] = Node {
                shape: Shape::Sphere,
                radius: 2,
                op: WriteOp::SetPhase { phase: PHASE_ICE },
            };
            template.nodes[1] = Node {
                shape: Shape::Sphere,
                radius: 2,
                op: WriteOp::SetFlags {
                    set: FLAG_ICE,
                    clear: 0,
                },
            };
            template.nodes[2] = Node {
                shape: Shape::Sphere,
                radius: 2,
                op: WriteOp::AddTemp {
                    delta: -200,
                    stack: false,
                },
            };
            template.node_count = 3;
            template.ice_radius = 2;
        }
        TEMPLATE_DENSITY => {
            template.nodes[0] = Node {
                shape: Shape::Sphere,
                radius: 1,
                op: WriteOp::AddDensity {
                    delta: 40,
                    stack: false,
                },
            };
            template.node_count = 1;
        }
        TEMPLATE_GRAVITY => {
            // Zone index 1 is the host's zero-g row. The field stores the index.
            template.nodes[0] = Node {
                shape: Shape::Sphere,
                radius: 2,
                op: WriteOp::SetGravity { index: 1 },
            };
            template.node_count = 1;
        }
        TEMPLATE_WIND => {
            template.nodes[0] = Node {
                shape: Shape::Point,
                radius: 0,
                op: WriteOp::SetWind { x: 1, y: 0, z: 0 },
            };
            template.node_count = 1;
        }
        TEMPLATE_LOWER_WATER => {
            template.nodes[0] = Node {
                shape: Shape::Sphere,
                radius: 2,
                op: WriteOp::AddVolume { delta: -255 },
            };
            template.node_count = 1;
        }
        TEMPLATE_DECAY => {
            template.nodes[0] = Node {
                shape: Shape::Sphere,
                radius: 1,
                op: WriteOp::AddDensity {
                    delta: -40,
                    stack: false,
                },
            };
            template.node_count = 1;
        }
        _ => return None,
    }
    Some(template)
}

pub fn writes_for(
    template: &Template,
    origin: [i32; 3],
    seq_base: u32,
) -> ([FieldWrite; MAX_NODES], u8) {
    let mut out = [FieldWrite {
        seq: 0,
        shape: Shape::Point,
        origin,
        end: origin,
        radius: 0,
        op: WriteOp::AddTemp {
            delta: 0,
            stack: false,
        },
    }; MAX_NODES];
    let count = template.node_count as usize;
    for (i, (slot, node)) in out
        .iter_mut()
        .zip(template.nodes.iter())
        .take(count)
        .enumerate()
    {
        *slot = FieldWrite {
            seq: seq_base.saturating_add(i as u32),
            shape: node.shape,
            origin,
            end: origin,
            radius: node.radius,
            op: node.op,
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
        let radius = node.get("radius").and_then(|v| v.as_integer()).unwrap_or(0) as u8;
        let a = node.get("a").and_then(|v| v.as_integer()).unwrap_or(0) as i16;
        let op = match kind {
            "heat" => WriteOp::AddTemp {
                delta: a,
                stack: false,
            },
            "wind" => WriteOp::SetWind {
                x: a as i8,
                y: 0,
                z: 0,
            },
            "gravity" => WriteOp::SetGravity { index: a as u16 },
            "phase" => WriteOp::SetPhase { phase: a as u8 },
            "volume" => WriteOp::AddVolume { delta: a },
            "density" => WriteOp::AddDensity {
                delta: a.clamp(-128, 127) as i8,
                stack: false,
            },
            _ => return Err(format!("unknown kind {kind}")),
        };
        let shape = if radius == 0 {
            Shape::Point
        } else {
            Shape::Sphere
        };
        template.nodes[i] = Node { shape, radius, op };
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
        assert!(matches!(
            template.nodes[0].op,
            WriteOp::AddDensity { delta: 120, .. }
        ));
    }
}
