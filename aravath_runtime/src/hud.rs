//! Strings the greybox HUD draws. The window paints them; this module does not
//! touch a GPU.

#[derive(Clone, Debug, PartialEq)]
pub struct HudModel {
    pub health: f32,
    pub health_max: f32,
    pub stamina: f32,
    pub stamina_max: f32,
    pub locked: bool,
    pub block: &'static str,
    pub notice: Option<String>,
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

pub fn reticle_label(locked: bool) -> &'static str {
    if locked {
        "Locked"
    } else {
        ""
    }
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
            block: block_name(4),
            notice: Some("Saved".into()),
        };
        assert!((health_fraction(&model) - 1.0).abs() < 1e-4);
        assert!((stamina_fraction(&model) - 0.25).abs() < 1e-4);
        assert_eq!(reticle_label(model.locked), "Locked");
        assert_eq!(model.block, "Sand");
        assert_eq!(model.notice.as_deref(), Some("Saved"));
    }
}
