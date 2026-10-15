//! Mod-facing weave queue.
//!
//! A cast is data: a template id and an origin. The tick copies those fields
//! into a fixed array. It does not allocate and it does not evaluate a field.
//! Engine template ids 1 through 7 stay reserved for the engine tables.

use std::fmt;

pub const MAX_WEAVE_COMMANDS: usize = 64;
pub const ENGINE_TEMPLATE_MAX: u16 = 7;
/// Greybox island half-extent in cells. `queue_weave` rejects anything outside
/// this cube so `(origin ± radius) * CELL_MM` cannot overflow `i32`.
pub const ISLAND_HALF_CELLS: i32 = 2_048;
pub const MAX_TEMPLATE_NODES: usize = 8;
pub const MAX_LOADED_TEMPLATES: usize = 16;

pub const KIND_HEAT: u8 = 1;
pub const KIND_DENSITY: u8 = 2;
pub const KIND_GRAVITY: u8 = 3;
pub const KIND_PHASE: u8 = 4;
pub const KIND_VOLUME: u8 = 5;
pub const KIND_WIND: u8 = 6;
pub const KIND_DECAY: u8 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaveCommand {
    pub start: u64,
    pub until: u64,
    pub seq: u32,
    pub caster: u64,
    pub template: u16,
    pub origin: [i32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueError {
    UnknownTemplate,
    Full,
    OutsideIsland,
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueError::UnknownTemplate => f.write_str("unknown weave template"),
            QueueError::Full => f.write_str("weave queue is full"),
            QueueError::OutsideIsland => f.write_str("weave origin is outside the island"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaveNode {
    pub kind: u8,
    pub radius: u8,
    pub a: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaveTemplate {
    pub id: u16,
    pub duration: u32,
    pub nodes: [WeaveNode; MAX_TEMPLATE_NODES],
    pub node_count: u8,
}

impl WeaveTemplate {
    const fn empty(id: u16) -> Self {
        Self {
            id,
            duration: 1,
            nodes: [WeaveNode {
                kind: 0,
                radius: 0,
                a: 0,
            }; MAX_TEMPLATE_NODES],
            node_count: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WeaveScratch {
    commands: [WeaveCommand; MAX_WEAVE_COMMANDS],
    command_count: u8,
    loaded: [WeaveTemplate; MAX_LOADED_TEMPLATES],
    loaded_count: u8,
}

impl Default for WeaveScratch {
    fn default() -> Self {
        Self::new()
    }
}

impl WeaveScratch {
    pub fn new() -> Self {
        Self {
            commands: [WeaveCommand {
                start: 0,
                until: 0,
                seq: 0,
                caster: 0,
                template: 0,
                origin: [0, 0, 0],
            }; MAX_WEAVE_COMMANDS],
            command_count: 0,
            loaded: [WeaveTemplate::empty(0); MAX_LOADED_TEMPLATES],
            loaded_count: 0,
        }
    }

    pub fn command_count(&self) -> usize {
        self.command_count as usize
    }

    pub fn command(&self, index: usize) -> Option<WeaveCommand> {
        if index < self.command_count as usize {
            Some(self.commands[index])
        } else {
            None
        }
    }

    pub fn copy_loaded_ids(&self, out: &mut [u16]) -> usize {
        let count = (self.loaded_count as usize).min(out.len());
        for (slot, template) in out.iter_mut().zip(self.loaded.iter()).take(count) {
            *slot = template.id;
        }
        count
    }

    pub fn duration_of(&self, id: u16) -> u32 {
        if (1..=ENGINE_TEMPLATE_MAX).contains(&id) {
            return 1;
        }
        let count = self.loaded_count as usize;
        self.loaded[..count]
            .iter()
            .find(|template| template.id == id)
            .map(|template| template.duration.max(1))
            .unwrap_or(1)
    }

    pub fn load_template(&mut self, template: WeaveTemplate) -> Result<(), QueueError> {
        if template.id == 0 || template.id <= ENGINE_TEMPLATE_MAX {
            return Err(QueueError::UnknownTemplate);
        }
        let count = self.loaded_count as usize;
        if self.loaded[..count]
            .iter()
            .any(|slot| slot.id == template.id)
        {
            return Err(QueueError::UnknownTemplate);
        }
        if count >= self.loaded.len() {
            return Err(QueueError::Full);
        }
        self.loaded[count] = template;
        self.loaded_count = self.loaded_count.saturating_add(1);
        Ok(())
    }

    fn known(&self, id: u16) -> bool {
        if (1..=ENGINE_TEMPLATE_MAX).contains(&id) {
            return true;
        }
        let count = self.loaded_count as usize;
        self.loaded[..count]
            .iter()
            .any(|template| template.id == id)
    }

    fn enqueue(
        &mut self,
        start: u64,
        until: u64,
        seq: u32,
        caster: u64,
        template: u16,
        origin: [i32; 3],
    ) -> Result<(), QueueError> {
        if !origin_on_island(origin) {
            return Err(QueueError::OutsideIsland);
        }
        if !self.known(template) {
            return Err(QueueError::UnknownTemplate);
        }
        let index = self.command_count as usize;
        if index >= MAX_WEAVE_COMMANDS {
            return Err(QueueError::Full);
        }
        self.commands[index] = WeaveCommand {
            start,
            until,
            seq,
            caster,
            template,
            origin,
        };
        self.command_count = self.command_count.saturating_add(1);
        Ok(())
    }

    pub(crate) fn loaded(&self, id: u16) -> Option<WeaveTemplate> {
        let count = self.loaded_count as usize;
        self.loaded[..count]
            .iter()
            .copied()
            .find(|template| template.id == id)
    }
}

/// Mod command entry. The sandbox calls this with host-filled tick fields.
pub fn queue_weave(
    scratch: &mut WeaveScratch,
    start: u64,
    until: u64,
    seq: u32,
    caster: u64,
    template: u16,
    origin: [i32; 3],
) -> Result<(), QueueError> {
    scratch.enqueue(start, until, seq, caster, template, origin)
}

/// Parse one template. Load time only. Ids 1 through 7 are engine ids.
pub fn parse_template_toml(text: &str) -> Result<WeaveTemplate, String> {
    let value: toml::Value = toml::from_str(text).map_err(|err| err.to_string())?;
    let raw_id = integer_field(&value, "id")?;
    if raw_id < 0 || raw_id > i64::from(u16::MAX) {
        return Err(format!("template id {raw_id} does not fit u16"));
    }
    let id = raw_id as u16;
    if id == 0 || id <= ENGINE_TEMPLATE_MAX {
        return Err(format!("template {id} is an engine id"));
    }
    let duration = match value.get("duration") {
        None => 1u32,
        Some(slot) => {
            let ticks = integer_value(slot, "duration")?;
            if ticks <= 0 || ticks > 10_000 {
                return Err("duration out of range".to_string());
            }
            ticks as u32
        }
    };
    let nodes = value
        .get("nodes")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "nodes".to_string())?;
    if nodes.is_empty() || nodes.len() > MAX_TEMPLATE_NODES {
        return Err("node count out of range".to_string());
    }
    let mut template = WeaveTemplate::empty(id);
    template.duration = duration;
    for (index, node) in nodes.iter().enumerate() {
        let kind_name = node
            .get("kind")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| "kind".to_string())?;
        let kind = match kind_name {
            "heat" => KIND_HEAT,
            "density" => KIND_DENSITY,
            "gravity" => KIND_GRAVITY,
            "phase" => KIND_PHASE,
            "volume" => KIND_VOLUME,
            "wind" => KIND_WIND,
            "decay" => KIND_DECAY,
            _ => return Err(format!("unknown kind {kind_name}")),
        };
        let radius = match node.get("radius") {
            None => 0u8,
            Some(slot) => {
                let value = integer_value(slot, "radius")?;
                if !(0..=255).contains(&value) {
                    return Err("radius out of range".to_string());
                }
                value as u8
            }
        };
        let a = match node.get("a") {
            None => 0i16,
            Some(slot) => {
                let value = integer_value(slot, "a")?;
                if value < i64::from(i16::MIN) || value > i64::from(i16::MAX) {
                    return Err("a out of range".to_string());
                }
                value as i16
            }
        };
        template.nodes[index] = WeaveNode { kind, radius, a };
    }
    template.node_count = nodes.len() as u8;
    Ok(template)
}

fn integer_field(value: &toml::Value, key: &str) -> Result<i64, String> {
    let slot = value.get(key).ok_or_else(|| key.to_string())?;
    integer_value(slot, key)
}

fn origin_on_island(origin: [i32; 3]) -> bool {
    origin
        .iter()
        .all(|cell| (-ISLAND_HALF_CELLS..=ISLAND_HALF_CELLS).contains(cell))
}

fn integer_value(slot: &toml::Value, key: &str) -> Result<i64, String> {
    slot.as_integer()
        .ok_or_else(|| format!("{key} must be an integer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_ids_queue_and_unknown_ids_do_not() {
        let mut scratch = WeaveScratch::new();
        queue_weave(&mut scratch, 4, 5, 0, 9, 1, [2, 3, 4]).unwrap();
        assert_eq!(scratch.command_count(), 1);
        let command = scratch.command(0).unwrap();
        assert_eq!(command.template, 1);
        assert_eq!(command.origin, [2, 3, 4]);
        assert_eq!(
            queue_weave(&mut scratch, 0, 1, 1, 9, 99, [0, 0, 0]),
            Err(QueueError::UnknownTemplate)
        );
        assert_eq!(scratch.command_count(), 1);
    }

    #[test]
    fn parse_rejects_engine_ids_and_accepts_modifier_data() {
        let err = parse_template_toml("id = 1\nnodes = [{ kind = \"heat\" }]\n").unwrap_err();
        assert!(err.contains("engine id"));
        let err = parse_template_toml("id = 8.5\nnodes = [{ kind = \"heat\" }]\n").unwrap_err();
        assert!(err.contains("integer"));
        let err = parse_template_toml("id = 65536\nnodes = [{ kind = \"heat\" }]\n").unwrap_err();
        assert!(err.contains("does not fit u16"), "{err}");
        let err = parse_template_toml("id = -1\nnodes = [{ kind = \"heat\" }]\n").unwrap_err();
        assert!(err.contains("does not fit u16"), "{err}");
        let wide = parse_template_toml("id = 65535\nnodes = [{ kind = \"heat\" }]\n").unwrap();
        assert_eq!(wide.id, 65535);
        let template = parse_template_toml(
            "id = 8\nduration = 4\n[[nodes]]\nkind = \"density\"\nradius = 1\na = 40\n",
        )
        .unwrap();
        assert_eq!(template.id, 8);
        assert_eq!(template.duration, 4);
        assert_eq!(template.nodes[0].kind, KIND_DENSITY);
        let mut scratch = WeaveScratch::new();
        scratch.load_template(template).unwrap();
        queue_weave(&mut scratch, 0, 4, 0, 1, 8, [0, 1, 0]).unwrap();
        assert_eq!(scratch.command(0).unwrap().until, 4);
    }

    #[test]
    fn queue_stops_at_the_fixed_cap() {
        let mut scratch = WeaveScratch::new();
        for seq in 0..MAX_WEAVE_COMMANDS {
            queue_weave(&mut scratch, 0, 1, seq as u32, 1, 1, [0, 0, 0]).unwrap();
        }
        assert_eq!(
            queue_weave(&mut scratch, 0, 1, 64, 1, 1, [0, 0, 0]),
            Err(QueueError::Full)
        );
        assert_eq!(scratch.command_count(), MAX_WEAVE_COMMANDS);
    }

    #[test]
    fn origins_outside_the_island_are_rejected_without_saturation() {
        let mut scratch = WeaveScratch::new();
        queue_weave(
            &mut scratch,
            1,
            2,
            0,
            1,
            1,
            [ISLAND_HALF_CELLS, 0, -ISLAND_HALF_CELLS],
        )
        .unwrap();
        assert_eq!(scratch.command_count(), 1);
        for origin in [
            [ISLAND_HALF_CELLS + 1, 0, 0],
            [0, -ISLAND_HALF_CELLS - 1, 0],
            [i32::MAX, 0, 0],
            [i32::MIN, 0, 0],
        ] {
            assert_eq!(
                queue_weave(&mut scratch, 1, 2, 1, 1, 1, origin),
                Err(QueueError::OutsideIsland)
            );
        }
        assert_eq!(scratch.command_count(), 1);
        assert_eq!(
            scratch.command(0).unwrap().origin,
            [ISLAND_HALF_CELLS, 0, -ISLAND_HALF_CELLS]
        );
    }
}
