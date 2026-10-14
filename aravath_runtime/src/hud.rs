//! Strings the greybox HUD draws. The window paints them; this module does not
//! touch a GPU.

#[derive(Clone, Debug, PartialEq)]
pub struct HudModel {
    pub health: f32,
    pub health_max: f32,
    pub stamina: f32,
    pub stamina_max: f32,
    pub locked: bool,
    /// Set when lock-on has a foe. The string is a PLACEHOLDER from the brief.
    pub locked_name: Option<&'static str>,
    pub block: &'static str,
    pub notice: Option<String>,
    pub downed: bool,
    pub mouse_hint: &'static str,
}

pub fn mouse_hint(captured: bool) -> &'static str {
    if captured {
        "Esc to release"
    } else {
        "Click to capture mouse / Esc to release"
    }
}

pub fn block_name(material: u16) -> &'static str {
    match material {
        2 => "Dirt",
        3 => "Grass",
        4 => "Sand",
        5 => "Wood",
        _ => "Stone",
    }
}

pub fn health_fraction(model: &HudModel) -> f32 {
    if model.health_max <= 0.0 {
        0.0
    } else {
        (model.health / model.health_max).clamp(0.0, 1.0)
    }
}

pub fn stamina_fraction(model: &HudModel) -> f32 {
    if model.stamina_max <= 0.0 {
        0.0
    } else {
        (model.stamina / model.stamina_max).clamp(0.0, 1.0)
    }
}

pub fn respawn_hint(downed: bool) -> Option<&'static str> {
    if downed {
        Some("Press Enter to respawn")
    } else {
        None
    }
}

/// Spawn order in the greybox window. Index 0 is the first `spawn_enemy`.
pub const FOE_NAMES: [&str; 3] = [
    "PLACEHOLDER seepage-touched beast",
    "PLACEHOLDER hollow-eyed wildlife",
    "PLACEHOLDER roadside brigand",
];

pub fn foe_name(index: usize) -> &'static str {
    FOE_NAMES.get(index).copied().unwrap_or("PLACEHOLDER foe")
}

pub fn reticle_label(locked_name: Option<&str>) -> &str {
    locked_name.unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_and_blocks_cover_the_hud() {
        assert_eq!(block_name(1), "Stone");
        assert_eq!(block_name(5), "Wood");
        let model = HudModel {
            health: 150.0,
            health_max: 100.0,
            stamina: 25.0,
            stamina_max: 100.0,
            locked: true,
            locked_name: Some(foe_name(2)),
            block: block_name(4),
            notice: Some("Saved".into()),
            downed: true,
            mouse_hint: mouse_hint(false),
        };
        assert_eq!(model.mouse_hint, "Click to capture mouse / Esc to release");
        assert_eq!(mouse_hint(true), "Esc to release");
        assert_eq!(respawn_hint(model.downed), Some("Press Enter to respawn"));
        assert!(respawn_hint(false).is_none());
        assert!((health_fraction(&model) - 1.0).abs() < 1e-4);
        assert!((stamina_fraction(&model) - 0.25).abs() < 1e-4);
        assert_eq!(reticle_label(model.locked_name), foe_name(2));
        assert_eq!(foe_name(0), "PLACEHOLDER seepage-touched beast");
        assert_eq!(foe_name(1), "PLACEHOLDER hollow-eyed wildlife");
        assert!(reticle_label(None).is_empty());
        assert_eq!(model.block, "Sand");
        assert_eq!(model.notice.as_deref(), Some("Saved"));
    }
}
