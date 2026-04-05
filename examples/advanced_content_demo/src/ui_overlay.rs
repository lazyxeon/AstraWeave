/// UI Overlay Module for Advanced Content Demo
///
/// This module provides console-based UI overlays for demonstrating
/// ability cooldowns, quest progress, and Echo currency HUD.
///
/// In a real game engine, this would be replaced with actual egui/imgui rendering,
/// but for this demo we use ANSI escape codes and console formatting.
use astraweave_weaving::*;
use glam::Vec3;

/// Console UI colors using ANSI escape codes
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";

    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    #[allow(dead_code)]
    pub const BG_BLACK: &str = "\x1b[40m";
    pub const BG_RED: &str = "\x1b[41m";
    pub const BG_GREEN: &str = "\x1b[42m";
    pub const BG_BLUE: &str = "\x1b[44m";
}

/// Renders a progress bar for ability cooldowns
pub fn render_cooldown_bar(ability_name: &str, current: f32, max: f32, width: usize) -> String {
    let percentage = (current / max).clamp(0.0, 1.0);
    let filled = (percentage * width as f32) as usize;
    let empty = width.saturating_sub(filled);

    let bar_color = if percentage < 0.3 {
        colors::RED
    } else if percentage < 0.7 {
        colors::YELLOW
    } else {
        colors::GREEN
    };

    format!(
        "{}{:<12}{} [{}{}{}{}] {:.1}s / {:.1}s{}",
        colors::BOLD,
        ability_name,
        colors::RESET,
        bar_color,
        "█".repeat(filled),
        colors::DIM,
        "░".repeat(empty),
        current,
        max,
        colors::RESET
    )
}

/// Renders Echo currency HUD (top-right style)
pub fn render_echo_hud(echo_currency: i32, max_width: usize) -> String {
    let padding = max_width.saturating_sub(20);
    format!(
        "{}{}{}{} Echo: {}{}{} {}{}",
        " ".repeat(padding),
        colors::BG_BLUE,
        colors::BOLD,
        colors::CYAN,
        colors::YELLOW,
        echo_currency,
        colors::CYAN,
        "⚡",
        colors::RESET
    )
}

/// Renders quest progress UI with checkboxes
pub fn render_quest_progress(quest: &Quest) -> Vec<String> {
    let mut lines = Vec::new();

    // Quest title
    lines.push(format!(
        "{}{}{} {} {}{}",
        colors::BOLD,
        colors::CYAN,
        "📋",
        quest.title,
        get_quest_state_symbol(quest.state),
        colors::RESET
    ));

    // Quest description
    lines.push(format!(
        "   {}{}{}",
        colors::DIM,
        quest.description,
        colors::RESET
    ));

    // Objectives with checkboxes
    for (i, obj) in quest.objectives.iter().enumerate() {
        let checkbox = if obj.is_complete() {
            format!("{}✓{}", colors::GREEN, colors::RESET)
        } else {
            format!("{}☐{}", colors::DIM, colors::RESET)
        };

        let progress_text = obj.progress();
        let desc = obj.description();

        lines.push(format!(
            "   {} {}. {} {}({}){}",
            checkbox,
            i + 1,
            desc,
            colors::DIM,
            progress_text,
            colors::RESET
        ));
    }

    // Rewards
    if !quest.rewards.is_empty() {
        let reward_text = quest
            .rewards
            .iter()
            .map(|r| format!("{:?}", r))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "   {}Reward: {}{}{}",
            colors::DIM,
            colors::YELLOW,
            reward_text,
            colors::RESET
        ));
    }

    lines
}

/// Renders ability panel with cooldowns and Echo costs
pub fn render_ability_panel(player: &Player) -> Vec<String> {
    let mut lines = Vec::new();

    lines.push(format!(
        "{}{} Abilities {}{}",
        colors::BOLD,
        colors::MAGENTA,
        "⚔",
        colors::RESET
    ));

    // Dash ability
    let dash = &player.ability_manager.echo_dash;
    let dash_ready = dash.can_use(player.echo_currency as u32);
    let dash_status = if dash_ready {
        format!("{}READY{}", colors::GREEN, colors::RESET)
    } else {
        format!("{}COOLDOWN{}", colors::RED, colors::RESET)
    };

    lines.push(format!(
        "  {}[D]{} Dash ({}⚡) - {}",
        colors::CYAN,
        colors::RESET,
        dash.state.echo_cost,
        dash_status
    ));

    if !dash_ready {
        lines.push(format!(
            "     {}",
            render_cooldown_bar(
                "Cooldown",
                dash.state.remaining_cooldown(),
                dash.state.cooldown_seconds,
                20
            )
        ));
    }

    // Shield ability
    let shield = &player.ability_manager.echo_shield;
    let shield_ready = shield.can_use(player.echo_currency as u32);
    let shield_status = if shield_ready {
        format!("{}READY{}", colors::GREEN, colors::RESET)
    } else {
        format!("{}COOLDOWN{}", colors::RED, colors::RESET)
    };

    lines.push(format!(
        "  {}[S]{} Shield ({}⚡) - {}",
        colors::CYAN,
        colors::RESET,
        shield.state.echo_cost,
        shield_status
    ));

    if !shield_ready {
        lines.push(format!(
            "     {}",
            render_cooldown_bar(
                "Cooldown",
                shield.state.remaining_cooldown(),
                shield.state.cooldown_seconds,
                20
            )
        ));
    }

    lines
}

/// Renders full HUD overlay (combines all UI elements)
pub fn render_full_hud(player: &Player, quest: &Quest, frame_width: usize) -> String {
    let mut output = String::new();

    // Top bar: Echo HUD
    output.push_str(&format!(
        "{}\n",
        render_echo_hud(player.echo_currency, frame_width)
    ));
    output.push_str(&format!("{}\n", "─".repeat(frame_width)));

    // Left panel: Abilities
    let ability_lines = render_ability_panel(player);
    output.push_str(&format!("\n{}\n", ability_lines.join("\n")));

    // Right panel: Quest progress (would be overlaid in real UI)
    let quest_lines = render_quest_progress(quest);
    output.push_str(&format!("\n{}\n", quest_lines.join("\n")));

    output.push_str(&format!("\n{}\n", "─".repeat(frame_width)));

    output
}

/// Renders a notification popup (simulated)
pub fn render_notification(title: &str, message: &str, icon: &str) -> String {
    format!(
        "\n{}{}╔═══════════════════════════════════════════╗{}\n\
         {}║ {} {}{}{}                              {}║{}\n\
         {}║ {}{}                                      {}║{}\n\
         {}╚═══════════════════════════════════════════╝{}\n",
        colors::BG_GREEN,
        colors::BOLD,
        colors::RESET,
        colors::BG_GREEN,
        icon,
        colors::BOLD,
        colors::WHITE,
        title,
        colors::BG_GREEN,
        colors::RESET,
        colors::BG_GREEN,
        colors::WHITE,
        message,
        colors::BG_GREEN,
        colors::RESET,
        colors::BG_GREEN,
        colors::RESET
    )
}

/// Helper: QuestState symbol for UI display
pub fn get_quest_state_symbol(state: QuestState) -> &'static str {
    match state {
        QuestState::Inactive => "⭕",
        QuestState::Active => "🔄",
        QuestState::Completed => "✅",
        QuestState::Failed => "❌",
        _ => "❓",
    }
}

/// Helper: Simulate particle effects with ASCII art
pub fn render_particle_effect(effect_type: &str, position: Vec3) -> String {
    match effect_type {
        "dash_trail" => format!(
            "{}💨 Dash Trail at ({:.1}, {:.1}, {:.1}){}",
            colors::CYAN,
            position.x,
            position.y,
            position.z,
            colors::RESET
        ),
        "shield_bubble" => format!(
            "{}🛡️  Shield Bubble at ({:.1}, {:.1}, {:.1}){}",
            colors::BLUE,
            position.x,
            position.y,
            position.z,
            colors::RESET
        ),
        "spawn_portal" => format!(
            "{}🌀 Spawn Portal at ({:.1}, {:.1}, {:.1}){}",
            colors::MAGENTA,
            position.x,
            position.y,
            position.z,
            colors::RESET
        ),
        "damage_numbers" => format!(
            "{}💥 -25 HP at ({:.1}, {:.1}, {:.1}){}",
            colors::RED,
            position.x,
            position.y,
            position.z,
            colors::RESET
        ),
        _ => format!("✨ Effect: {} at {:?}", effect_type, position),
    }
}

/// Helper: Simulate audio effects
pub fn play_audio_effect(effect_type: &str) -> String {
    match effect_type {
        "dash_whoosh" => format!("{}🔊 Audio: Dash Whoosh{}", colors::YELLOW, colors::RESET),
        "shield_activate" => format!(
            "{}🔊 Audio: Shield Activate{}",
            colors::YELLOW,
            colors::RESET
        ),
        "quest_complete" => format!(
            "{}🔊 Audio: Quest Complete Jingle{}",
            colors::YELLOW,
            colors::RESET
        ),
        "spawn_portal" => format!("{}🔊 Audio: Portal Sound{}", colors::YELLOW, colors::RESET),
        "objective_complete" => format!(
            "{}🔊 Audio: Objective Complete{}",
            colors::YELLOW,
            colors::RESET
        ),
        _ => format!("🔊 Audio: {}", effect_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_cooldown_bar() {
        let bar = render_cooldown_bar("Test", 5.0, 10.0, 20);
        assert!(bar.contains("Test"));
        assert!(bar.contains("5.0"));
        assert!(bar.contains("10.0"));
    }

    #[test]
    fn test_render_echo_hud() {
        let hud = render_echo_hud(100, 60);
        assert!(hud.contains("100"));
        assert!(hud.contains("Echo"));
    }

    #[test]
    fn test_render_notification() {
        let notif = render_notification("Quest Complete!", "You earned 50 Echo", "✅");
        assert!(notif.contains("Quest Complete!"));
        assert!(notif.contains("50 Echo"));
    }

    #[test]
    fn test_render_particle_effect() {
        let effect = render_particle_effect("dash_trail", Vec3::new(10.0, 0.0, 5.0));
        assert!(effect.contains("Dash Trail"));
        assert!(effect.contains("10.0"));
    }

    #[test]
    fn test_play_audio_effect() {
        let audio = play_audio_effect("dash_whoosh");
        assert!(audio.contains("Dash Whoosh"));
    }
}
