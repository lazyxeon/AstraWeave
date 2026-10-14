use egui::{self, Color32, RichText};
use glam::Vec3;

use astraweave_gameplay::quests::QuestLog;
use astraweave_gameplay::stats::Stats;
use astraweave_gameplay::{Inventory, RecipeBook};

use crate::menu::{MenuAction, MenuManager};
use crate::{Accessibility, UiFlags};
use astraweave_cinematics as awc;

#[derive(Default)]
pub struct UiResult {
    pub crafted: Option<String>, // name
    pub menu_action: Option<MenuAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopBarAction {
    Menu,
    Inventory,
    Crafting,
    Map,
    Quests,
    Settings,
}

fn apply_menu_action(menu_manager: &mut MenuManager, action: MenuAction, out: &mut UiResult) {
    if action != MenuAction::None {
        menu_manager.handle_action(action);
        out.menu_action = Some(action);
    }
}

fn apply_top_bar_action(action: TopBarAction, flags: &mut UiFlags, menu_manager: &mut MenuManager) {
    match action {
        TopBarAction::Menu => menu_manager.toggle_pause(),
        TopBarAction::Inventory => flags.show_inventory = !flags.show_inventory,
        TopBarAction::Crafting => flags.show_crafting = !flags.show_crafting,
        TopBarAction::Map => flags.show_map = !flags.show_map,
        TopBarAction::Quests => flags.show_quests = !flags.show_quests,
        TopBarAction::Settings => flags.show_settings = !flags.show_settings,
    }
}

fn colorblind_mode_to_index(mode: Option<&str>) -> usize {
    match mode {
        Some("protanopia") => 1,
        Some("deuteranopia") => 2,
        Some("tritanopia") => 3,
        _ => 0,
    }
}

fn colorblind_mode_from_index(idx: usize) -> Option<String> {
    match idx {
        1 => Some("protanopia".into()),
        2 => Some("deuteranopia".into()),
        3 => Some("tritanopia".into()),
        _ => None,
    }
}

fn recipe_world_seed(name: &str) -> u64 {
    // FNV-1a. The UI has no world clock; the recipe name is the seed so the
    // same button press writes the same item id.
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in name.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

fn craft_and_push(book: &RecipeBook, recipe_name: &str, inventory: &mut Inventory) -> bool {
    let tick = inventory.items.len() as u64;
    if let Some(it) =
        book.craft_with_seed(recipe_name, inventory, recipe_world_seed(recipe_name), tick)
    {
        inventory.items.push(it.clone());
        true
    } else {
        false
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_ui(
    ctx: &egui::Context,
    flags: &mut UiFlags,
    menu_manager: &mut MenuManager,
    acc: &mut Accessibility,
    player_stats: &Stats,
    player_pos: Vec3,
    inventory: &mut Inventory,
    recipes: Option<&RecipeBook>,
    quests: Option<&mut QuestLog>,
) -> UiResult {
    let mut out = UiResult::default();

    // Menu System Integration
    let action = menu_manager.show(ctx);
    apply_menu_action(menu_manager, action, &mut out);

    // Top bar – Menu toggles
    egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            if ui.button("Menu").clicked() {
                apply_top_bar_action(TopBarAction::Menu, flags, menu_manager);
            }
            if ui.button("Inventory (I)").clicked() {
                apply_top_bar_action(TopBarAction::Inventory, flags, menu_manager);
            }
            if ui.button("Crafting (C)").clicked() {
                apply_top_bar_action(TopBarAction::Crafting, flags, menu_manager);
            }
            if ui.button("Map (M)").clicked() {
                apply_top_bar_action(TopBarAction::Map, flags, menu_manager);
            }
            if ui.button("Quests (J)").clicked() {
                apply_top_bar_action(TopBarAction::Quests, flags, menu_manager);
            }
            if ui.button("Settings").clicked() {
                apply_top_bar_action(TopBarAction::Settings, flags, menu_manager);
            }
            ui.separator();
            ui.label(
                RichText::new(format!("HP: {}", player_stats.hp))
                    .color(Color32::from_rgb(220, 80, 80)),
            );
            ui.label(
                RichText::new(format!("STA: {}", player_stats.stamina))
                    .color(Color32::from_rgb(80, 180, 220)),
            );
            ui.label(format!(
                "Pos: {:.1}, {:.1}, {:.1}",
                player_pos.x, player_pos.y, player_pos.z
            ));
        });
    });

    // HUD (bottom-left small)
    egui::TopBottomPanel::bottom("hud").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("Ability1 [Q]  Ability2 [E]");
            if acc.high_contrast_ui {
                ui.colored_label(Color32::YELLOW, "High Contrast");
            }
            if let Some(mode) = &acc.colorblind_mode {
                ui.label(format!("CB: {}", mode));
            }
        });
    });

    if flags.show_inventory {
        egui::Window::new("Inventory")
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Resources");
                for (k, c) in &inventory.resources {
                    ui.label(format!("{:?}: {}", k, c));
                }
                ui.separator();
                ui.heading("Items");
                for it in &inventory.items {
                    ui.label(format!("{} {:?}", it.name, it.kind));
                }
            });
    }

    if flags.show_crafting {
        egui::Window::new("Crafting")
            .resizable(true)
            .show(ctx, |ui| {
                if let Some(book) = recipes {
                    for r in &book.recipes {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} -> {:?}", r.name, r.output_item));
                            if ui.button("Craft").clicked()
                                && craft_and_push(book, &r.name, inventory)
                            {
                                out.crafted = Some(r.name.clone());
                            }
                        });
                    }
                } else {
                    ui.label("No recipes loaded.");
                }
            });
    }

    if flags.show_map {
        egui::Window::new("Map").resizable(true).show(ctx, |ui| {
            ui.label("World Map (placeholder).");
            ui.add(egui::widgets::Label::new(
                "MiniMap: TODO: render navmesh texture or world quads",
            ));
        });
    }

    if flags.show_quests {
        egui::Window::new("Quest Log")
            .resizable(true)
            .show(ctx, |ui| {
                if let Some(q) = quests {
                    for (id, quest) in &q.quests {
                        ui.heading(format!("{} - {}", id, quest.title));
                        for t in &quest.tasks {
                            ui.label(format!(
                                "{:?} {} {}",
                                t.kind,
                                if t.done { "[done]" } else { "[todo]" },
                                t.id
                            ));
                        }
                        ui.separator();
                    }
                } else {
                    ui.label("No quests.");
                }
            });
    }

    if flags.show_settings {
        egui::Window::new("Settings / Accessibility").resizable(true).show(ctx, |ui| {
            ui.checkbox(&mut acc.high_contrast_ui, "High contrast UI");
            ui.checkbox(&mut acc.reduce_motion, "Reduce motion");
            ui.checkbox(&mut acc.subtitles, "Subtitles");
            ui.add(egui::Slider::new(&mut acc.subtitle_scale, 0.6..=1.8).text("Subtitle scale"));
            let mut cb_idx: usize = colorblind_mode_to_index(acc.colorblind_mode.as_deref());
            egui::ComboBox::from_label("Colorblind mode").selected_text(match cb_idx {
                1 => "Protanopia",
                2 => "Deuteranopia",
                3 => "Tritanopia",
                _ => "None",
            }).show_ui(ui, |ui| {
                ui.selectable_value(&mut cb_idx, 0, "None");
                ui.selectable_value(&mut cb_idx, 1, "Protanopia");
                ui.selectable_value(&mut cb_idx, 2, "Deuteranopia");
                ui.selectable_value(&mut cb_idx, 3, "Tritanopia");
            });
            acc.colorblind_mode = colorblind_mode_from_index(cb_idx);

            ui.separator();
            ui.label("Input Remapping (press the desired key/gamepad button after selecting an action):");
            ui.label("This demo shows the UX portion; wire into the input manager's remap call.");
        });
    }

    // Simple Cinematics panel (dev-only)
    egui::Window::new("Cinematics")
        .resizable(true)
        .show(ctx, |ui| {
            use std::sync::{Mutex, OnceLock};
            static TL: OnceLock<Mutex<Option<awc::Timeline>>> = OnceLock::new();
            static SEQ: OnceLock<Mutex<Option<awc::Sequencer>>> = OnceLock::new();
            static FILENAME: OnceLock<Mutex<String>> = OnceLock::new();
            let tl = TL.get_or_init(|| Mutex::new(None));
            let seq = SEQ.get_or_init(|| Mutex::new(None));
            let filename =
                FILENAME.get_or_init(|| Mutex::new("assets/cinematics/cutscene.json".to_string()));
            let mut load_demo = false;
            ui.horizontal(|ui| {
                if ui.button("Load Demo").clicked() {
                    load_demo = true;
                }
                if ui.button("Save JSON").clicked() {
                    if let Some(ref tlv) =
                        *tl.lock().expect("Timeline mutex poisoned - cannot recover")
                    {
                        let s = serde_json::to_string_pretty(tlv).unwrap();
                        ui.ctx().copy_text(s);
                    }
                }
                // Load/Save to assets
                {
                    let mut name = filename
                        .lock()
                        .expect("Filename mutex poisoned - cannot recover");
                    ui.text_edit_singleline(&mut *name);
                    if ui.button("Load File").clicked() {
                        match std::fs::read_to_string(&*name) {
                            Ok(s) => match serde_json::from_str::<awc::Timeline>(&s) {
                                Ok(new_tl) => {
                                    *tl.lock().expect("Timeline mutex poisoned - cannot recover") =
                                        Some(new_tl);
                                    *seq.lock()
                                        .expect("Sequencer mutex poisoned - cannot recover") =
                                        Some(awc::Sequencer::new());
                                }
                                Err(e) => {
                                    ui.label(format!("Parse error: {}", e));
                                }
                            },
                            Err(e) => {
                                ui.label(format!("IO error: {}", e));
                            }
                        }
                    }
                    if ui.button("Save File").clicked() {
                        if let Some(ref tlv) =
                            *tl.lock().expect("Timeline mutex poisoned - cannot recover")
                        {
                            match serde_json::to_string_pretty(tlv) {
                                Ok(s) => {
                                    let _ = std::fs::create_dir_all("assets/cinematics");
                                    let path = (*name).clone();
                                    let _ = std::fs::write(path, s);
                                }
                                Err(e) => {
                                    ui.label(format!("Serialize error: {}", e));
                                }
                            }
                        }
                    }
                }
                if ui.button("Play").clicked() {
                    let mut seq_guard = seq
                        .lock()
                        .expect("Sequencer mutex poisoned - cannot recover");
                    if seq_guard.is_none() {
                        *seq_guard = Some(awc::Sequencer::new());
                    }
                }
                if ui.button("Step 0.5s").clicked() {
                    let mut seq_guard = seq
                        .lock()
                        .expect("Sequencer mutex poisoned - cannot recover");
                    if let (Some(ref mut seqv), Some(tlv)) = (
                        seq_guard.as_mut(),
                        tl.lock()
                            .expect("Timeline mutex poisoned - cannot recover")
                            .as_ref(),
                    ) {
                        if let Ok(evs) = seqv.step(0.5, tlv) {
                            for e in evs {
                                ui.label(format!("{:4.1}s: {:?}", seqv.t.0, e));
                            }
                        }
                    }
                }
            });
            if load_demo {
                let mut new_tl = awc::Timeline::new("cutscene", 5.0);
                new_tl.tracks.push(awc::Track::Camera {
                    keyframes: vec![
                        awc::CameraKey {
                            t: awc::Time(1.0),
                            pos: (0.0, 1.5, 3.0),
                            look_at: (0.0, 1.0, 0.0),
                            fov_deg: 60.0,
                        },
                        awc::CameraKey {
                            t: awc::Time(3.0),
                            pos: (2.0, 2.0, 3.0),
                            look_at: (0.0, 1.0, 0.0),
                            fov_deg: 55.0,
                        },
                    ],
                });
                new_tl.tracks.push(awc::Track::Audio {
                    clip: "music:start".into(),
                    start: awc::Time(0.2),
                    volume: 0.7,
                });
                new_tl.tracks.push(awc::Track::Fx {
                    name: "fade-in".into(),
                    start: awc::Time(0.0),
                    params: serde_json::json!({"duration": 0.5}),
                });
                *tl.lock().expect("Timeline mutex poisoned - cannot recover") = Some(new_tl);
                *seq.lock()
                    .expect("Sequencer mutex poisoned - cannot recover") =
                    Some(awc::Sequencer::new());
            }
        });

    out
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use astraweave_gameplay::crafting::{CraftCost, CraftRecipe};
    use astraweave_gameplay::items::{Item, ItemKind};
    use astraweave_gameplay::quests::{Quest, Task, TaskKind};
    use astraweave_gameplay::DamageType;
    use astraweave_gameplay::ResourceKind;

    fn run_frame<T>(f: impl FnOnce(&egui::Context) -> T) -> T {
        let ctx = egui::Context::default();
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 720.0),
        ));

        ctx.begin_pass(input);
        let out = f(&ctx);
        let _ = ctx.end_pass();
        out
    }

    #[test]
    fn test_apply_menu_action_records_and_handles_non_none() {
        let mut menu_manager = MenuManager::new();
        // Move to in-game so Quit path is deterministic (PauseMenu quit goes to MainMenu)
        menu_manager.handle_action(MenuAction::NewGame);
        menu_manager.toggle_pause();
        assert_eq!(
            menu_manager.current_state(),
            crate::menu::MenuState::PauseMenu
        );

        let mut out = UiResult::default();
        apply_menu_action(&mut menu_manager, MenuAction::Quit, &mut out);
        assert_eq!(out.menu_action, Some(MenuAction::Quit));
        assert_eq!(
            menu_manager.current_state(),
            crate::menu::MenuState::MainMenu
        );
    }

    #[test]
    fn test_apply_top_bar_action_toggles_flags() {
        let mut flags = UiFlags::default();
        let mut menu_manager = MenuManager::new();
        apply_top_bar_action(TopBarAction::Inventory, &mut flags, &mut menu_manager);
        assert!(flags.show_inventory);
        apply_top_bar_action(TopBarAction::Inventory, &mut flags, &mut menu_manager);
        assert!(!flags.show_inventory);
    }

    #[test]
    fn test_colorblind_mode_index_roundtrip() {
        assert_eq!(colorblind_mode_to_index(None), 0);
        assert_eq!(colorblind_mode_to_index(Some("protanopia")), 1);
        assert_eq!(colorblind_mode_to_index(Some("deuteranopia")), 2);
        assert_eq!(colorblind_mode_to_index(Some("tritanopia")), 3);
        assert_eq!(colorblind_mode_to_index(Some("unknown")), 0);

        assert_eq!(colorblind_mode_from_index(0), None);
        assert_eq!(
            colorblind_mode_from_index(1),
            Some("protanopia".to_string())
        );
        assert_eq!(
            colorblind_mode_from_index(2),
            Some("deuteranopia".to_string())
        );
        assert_eq!(
            colorblind_mode_from_index(3),
            Some("tritanopia".to_string())
        );
        assert_eq!(colorblind_mode_from_index(99), None);
    }

    #[test]
    fn test_craft_and_push_success_and_failure() {
        let book = RecipeBook {
            recipes: vec![CraftRecipe {
                name: "Basic Armor".to_string(),
                output_item: ItemKind::Armor { defense: 5 },
                costs: vec![CraftCost {
                    kind: ResourceKind::Ore,
                    count: 2,
                }],
            }],
        };

        let mut inventory = Inventory::default();
        inventory.add_resource(ResourceKind::Ore, 2);
        let before = inventory.items.len();
        assert!(craft_and_push(&book, "Basic Armor", &mut inventory));
        assert_eq!(inventory.items.len(), before + 1);

        // Now we should be out of Ore -> crafting should fail
        let before = inventory.items.len();
        assert!(!craft_and_push(&book, "Basic Armor", &mut inventory));
        assert_eq!(inventory.items.len(), before);
    }

    #[test]
    fn test_draw_ui_runs_with_all_panels_visible_without_input() {
        run_frame(|ctx| {
            let mut flags = UiFlags {
                show_menu: false,
                show_inventory: true,
                show_map: true,
                show_quests: true,
                show_crafting: true,
                show_settings: true,
            };

            let mut menu_manager = MenuManager::new();
            let mut acc = Accessibility::default();
            acc.high_contrast_ui = true;
            acc.colorblind_mode = Some("deuteranopia".to_string());

            let player_stats = Stats::new(100);
            let player_pos = Vec3::new(1.0, 2.0, 3.0);

            let mut inventory = Inventory::default();
            inventory.add_resource(ResourceKind::Ore, 10);
            inventory.items.push(Item {
                id: 1,
                name: "Test Sword".to_string(),
                kind: ItemKind::Weapon {
                    base_damage: 10,
                    dtype: DamageType::Physical,
                },
                echo: None,
            });

            let recipes = RecipeBook {
                recipes: vec![CraftRecipe {
                    name: "Basic Armor".to_string(),
                    output_item: ItemKind::Armor { defense: 5 },
                    costs: vec![CraftCost {
                        kind: ResourceKind::Ore,
                        count: 2,
                    }],
                }],
            };

            let mut quest_log = QuestLog::default();
            quest_log.add(Quest {
                id: "q1".to_string(),
                title: "Test Quest".to_string(),
                tasks: vec![Task {
                    id: "t1".to_string(),
                    kind: TaskKind::Visit {
                        marker: "marker_a".to_string(),
                    },
                    done: false,
                }],
                reward_text: "Reward".to_string(),
                completed: false,
            });

            let out = draw_ui(
                ctx,
                &mut flags,
                &mut menu_manager,
                &mut acc,
                &player_stats,
                player_pos,
                &mut inventory,
                Some(&recipes),
                Some(&mut quest_log),
            );

            assert!(out.crafted.is_none());
            assert!(out.menu_action.is_none());
        });
    }

    #[test]
    fn test_draw_ui_covers_no_recipes_and_done_task_paths() {
        run_frame(|ctx| {
            let mut flags = UiFlags {
                show_menu: false,
                show_inventory: false,
                show_map: false,
                show_quests: true,
                show_crafting: true,
                show_settings: true,
            };

            let mut menu_manager = MenuManager::new();
            let mut acc = Accessibility::default();
            acc.high_contrast_ui = true;
            acc.colorblind_mode = Some("protanopia".to_string());

            let player_stats = Stats::new(100);
            let player_pos = Vec3::new(0.0, 0.0, 0.0);
            let mut inventory = Inventory::default();

            let mut quest_log = QuestLog::default();
            quest_log.add(Quest {
                id: "q_done".to_string(),
                title: "Done Quest".to_string(),
                tasks: vec![Task {
                    id: "t_done".to_string(),
                    kind: TaskKind::Defeat {
                        enemy: "slime".to_string(),
                        count: 1,
                    },
                    done: true,
                }],
                reward_text: "Reward".to_string(),
                completed: false,
            });

            let out = draw_ui(
                ctx,
                &mut flags,
                &mut menu_manager,
                &mut acc,
                &player_stats,
                player_pos,
                &mut inventory,
                None,
                Some(&mut quest_log),
            );

            assert!(out.crafted.is_none());
            assert!(out.menu_action.is_none());
        });
    }
}
