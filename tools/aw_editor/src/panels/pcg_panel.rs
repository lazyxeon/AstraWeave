//! Procedural Content Generation Panel for the editor UI
//!
//! Provides comprehensive PCG tools:
//! - Seed-based random generation
//! - Encounter generation and configuration
//! - Dungeon/level layout generation
//! - Loot table configuration
//! - Biome and terrain procedural rules

#![allow(clippy::upper_case_acronyms)] // NPC is industry-standard acronym

use egui::{Color32, RichText, Ui, Vec2};
use tracing::{info, warn};

use crate::panels::Panel;

/// PCG generation type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GenerationType {
    #[default]
    Encounter,
    Dungeon,
    Loot,
    Terrain,
    Vegetation,
    Props,
    NPC,
}

impl std::fmt::Display for GenerationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl GenerationType {
    pub fn name(&self) -> &'static str {
        match self {
            GenerationType::Encounter => "Encounter",
            GenerationType::Dungeon => "Dungeon",
            GenerationType::Loot => "Loot",
            GenerationType::Terrain => "Terrain",
            GenerationType::Vegetation => "Vegetation",
            GenerationType::Props => "Props",
            GenerationType::NPC => "NPC",
        }
    }

    pub fn all() -> &'static [GenerationType] {
        &[
            GenerationType::Encounter,
            GenerationType::Dungeon,
            GenerationType::Loot,
            GenerationType::Terrain,
            GenerationType::Vegetation,
            GenerationType::Props,
            GenerationType::NPC,
        ]
    }

    pub fn icon(&self) -> &'static str {
        match self {
            GenerationType::Encounter => "⚔️",
            GenerationType::Dungeon => "🏰",
            GenerationType::Loot => "[Gem]",
            GenerationType::Terrain => "🏔️",
            GenerationType::Vegetation => "🌲",
            GenerationType::Props => "🪑",
            GenerationType::NPC => "👤",
        }
    }
}

/// Encounter difficulty
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EncounterDifficulty {
    Trivial,
    Easy,
    #[default]
    Medium,
    Hard,
    Deadly,
    Boss,
}

impl std::fmt::Display for EncounterDifficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl EncounterDifficulty {
    pub fn all() -> &'static [EncounterDifficulty] {
        &[
            EncounterDifficulty::Trivial,
            EncounterDifficulty::Easy,
            EncounterDifficulty::Medium,
            EncounterDifficulty::Hard,
            EncounterDifficulty::Deadly,
            EncounterDifficulty::Boss,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            EncounterDifficulty::Trivial => "Trivial",
            EncounterDifficulty::Easy => "Easy",
            EncounterDifficulty::Medium => "Medium",
            EncounterDifficulty::Hard => "Hard",
            EncounterDifficulty::Deadly => "Deadly",
            EncounterDifficulty::Boss => "Boss",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            EncounterDifficulty::Trivial => "-",
            EncounterDifficulty::Easy => "[G]",
            EncounterDifficulty::Medium => "[Y]",
            EncounterDifficulty::Hard => "🟠",
            EncounterDifficulty::Deadly => "[R]",
            EncounterDifficulty::Boss => "[Skull]",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            EncounterDifficulty::Trivial => Color32::from_rgb(150, 150, 150),
            EncounterDifficulty::Easy => Color32::from_rgb(100, 200, 100),
            EncounterDifficulty::Medium => Color32::from_rgb(255, 200, 100),
            EncounterDifficulty::Hard => Color32::from_rgb(255, 150, 50),
            EncounterDifficulty::Deadly => Color32::from_rgb(255, 80, 80),
            EncounterDifficulty::Boss => Color32::from_rgb(200, 50, 200),
        }
    }
}

/// Enemy type for encounters
#[derive(Debug, Clone)]
pub struct EnemyType {
    pub id: String,
    pub name: String,
    pub threat_level: f32,
    pub min_count: u32,
    pub max_count: u32,
    pub spawn_weight: f32,
}

impl Default for EnemyType {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Enemy".to_string(),
            threat_level: 1.0,
            min_count: 1,
            max_count: 3,
            spawn_weight: 1.0,
        }
    }
}

/// Encounter configuration
#[derive(Debug, Clone)]
pub struct EncounterConfig {
    pub id: u32,
    pub name: String,
    pub difficulty: EncounterDifficulty,
    pub enemy_types: Vec<String>,
    pub min_enemies: u32,
    pub max_enemies: u32,
    pub spawn_radius: f32,
    pub reinforcement_chance: f32,
}

impl Default for EncounterConfig {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Encounter".to_string(),
            difficulty: EncounterDifficulty::Medium,
            enemy_types: Vec::new(),
            min_enemies: 2,
            max_enemies: 5,
            spawn_radius: 10.0,
            reinforcement_chance: 0.0,
        }
    }
}

/// Room type for dungeon generation
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RoomType {
    #[default]
    Normal,
    Entrance,
    Exit,
    Treasure,
    Boss,
    Shop,
    Secret,
    Corridor,
}

impl std::fmt::Display for RoomType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl RoomType {
    pub fn all() -> &'static [RoomType] {
        &[
            RoomType::Normal,
            RoomType::Entrance,
            RoomType::Exit,
            RoomType::Treasure,
            RoomType::Boss,
            RoomType::Shop,
            RoomType::Secret,
            RoomType::Corridor,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            RoomType::Normal => "Normal",
            RoomType::Entrance => "Entrance",
            RoomType::Exit => "Exit",
            RoomType::Treasure => "Treasure",
            RoomType::Boss => "Boss",
            RoomType::Shop => "Shop",
            RoomType::Secret => "Secret",
            RoomType::Corridor => "Corridor",
        }
    }

    /// Returns true if this room type is special (not a regular room or corridor)
    pub fn is_special(&self) -> bool {
        !matches!(self, RoomType::Normal | RoomType::Corridor)
    }

    pub fn icon(&self) -> &'static str {
        match self {
            RoomType::Normal => "🚪",
            RoomType::Entrance => "🚶",
            RoomType::Exit => "🏁",
            RoomType::Treasure => "[Gold]",
            RoomType::Boss => "👹",
            RoomType::Shop => "🛒",
            RoomType::Secret => "❓",
            RoomType::Corridor => "[Rt]",
        }
    }
}

/// Room configuration
#[derive(Debug, Clone)]
pub struct RoomConfig {
    pub room_type: RoomType,
    pub min_size: (u32, u32),
    pub max_size: (u32, u32),
    pub spawn_chance: f32,
    pub max_count: u32,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            room_type: RoomType::Normal,
            min_size: (5, 5),
            max_size: (15, 15),
            spawn_chance: 1.0,
            max_count: 10,
        }
    }
}

/// Dungeon layout settings
#[derive(Debug, Clone)]
pub struct DungeonSettings {
    pub width: u32,
    pub height: u32,
    pub room_count: (u32, u32),
    pub corridor_width: u32,
    pub branching_factor: f32,
    pub loop_chance: f32,
    pub room_configs: Vec<RoomConfig>,
}

impl Default for DungeonSettings {
    fn default() -> Self {
        Self {
            width: 100,
            height: 100,
            room_count: (8, 15),
            corridor_width: 3,
            branching_factor: 0.3,
            loop_chance: 0.2,
            room_configs: vec![
                RoomConfig {
                    room_type: RoomType::Entrance,
                    spawn_chance: 1.0,
                    max_count: 1,
                    ..Default::default()
                },
                RoomConfig {
                    room_type: RoomType::Exit,
                    spawn_chance: 1.0,
                    max_count: 1,
                    ..Default::default()
                },
                RoomConfig {
                    room_type: RoomType::Normal,
                    spawn_chance: 1.0,
                    max_count: 10,
                    ..Default::default()
                },
                RoomConfig {
                    room_type: RoomType::Treasure,
                    spawn_chance: 0.3,
                    max_count: 2,
                    ..Default::default()
                },
                RoomConfig {
                    room_type: RoomType::Boss,
                    spawn_chance: 0.5,
                    max_count: 1,
                    ..Default::default()
                },
            ],
        }
    }
}

/// Loot rarity
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LootRarity {
    #[default]
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl std::fmt::Display for LootRarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl LootRarity {
    pub fn all() -> &'static [LootRarity] {
        &[
            LootRarity::Common,
            LootRarity::Uncommon,
            LootRarity::Rare,
            LootRarity::Epic,
            LootRarity::Legendary,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LootRarity::Common => "Common",
            LootRarity::Uncommon => "Uncommon",
            LootRarity::Rare => "Rare",
            LootRarity::Epic => "Epic",
            LootRarity::Legendary => "Legendary",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LootRarity::Common => "⚪",
            LootRarity::Uncommon => "[G]",
            LootRarity::Rare => "[B]",
            LootRarity::Epic => "🟣",
            LootRarity::Legendary => "🟠",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            LootRarity::Common => Color32::from_rgb(180, 180, 180),
            LootRarity::Uncommon => Color32::from_rgb(100, 200, 100),
            LootRarity::Rare => Color32::from_rgb(100, 150, 255),
            LootRarity::Epic => Color32::from_rgb(200, 100, 255),
            LootRarity::Legendary => Color32::from_rgb(255, 180, 50),
        }
    }
}

/// Loot item entry
#[derive(Debug, Clone)]
pub struct LootEntry {
    pub item_id: String,
    pub name: String,
    pub rarity: LootRarity,
    pub drop_weight: f32,
    pub min_quantity: u32,
    pub max_quantity: u32,
}

impl Default for LootEntry {
    fn default() -> Self {
        Self {
            item_id: String::new(),
            name: "Item".to_string(),
            rarity: LootRarity::Common,
            drop_weight: 1.0,
            min_quantity: 1,
            max_quantity: 1,
        }
    }
}

/// Loot table
#[derive(Debug, Clone)]
pub struct LootTable {
    pub id: u32,
    pub name: String,
    pub entries: Vec<LootEntry>,
    pub guaranteed_drops: u32,
    pub bonus_rolls: u32,
}

impl Default for LootTable {
    fn default() -> Self {
        Self {
            id: 0,
            name: "New Loot Table".to_string(),
            entries: Vec::new(),
            guaranteed_drops: 1,
            bonus_rolls: 2,
        }
    }
}

/// Generated preview data
#[derive(Debug, Clone)]
pub struct GenerationPreview {
    pub seed: u64,
    pub generation_type: GenerationType,
    pub result_summary: String,
    pub generation_time_ms: f32,
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PcgTab {
    #[default]
    Seeds,
    Encounters,
    Dungeons,
    Loot,
    Preview,
    History,
}

impl std::fmt::Display for PcgTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl PcgTab {
    pub fn all() -> &'static [PcgTab] {
        &[
            PcgTab::Seeds,
            PcgTab::Encounters,
            PcgTab::Dungeons,
            PcgTab::Loot,
            PcgTab::Preview,
            PcgTab::History,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            PcgTab::Seeds => "Seeds",
            PcgTab::Encounters => "Encounters",
            PcgTab::Dungeons => "Dungeons",
            PcgTab::Loot => "Loot",
            PcgTab::Preview => "Preview",
            PcgTab::History => "History",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PcgTab::Seeds => "🌱",
            PcgTab::Encounters => "⚔️",
            PcgTab::Dungeons => "🏰",
            PcgTab::Loot => "[Gem]",
            PcgTab::Preview => "👁️",
            PcgTab::History => "📜",
        }
    }
}

/// Main PCG Panel
pub struct PcgPanel {
    // Tab state
    active_tab: PcgTab,

    // Seed management
    current_seed: u64,
    seed_input: String,
    use_random_seed: bool,
    saved_seeds: Vec<(String, u64)>,

    // Encounters
    enemy_types: Vec<EnemyType>,
    encounters: Vec<EncounterConfig>,
    selected_encounter: Option<u32>,

    // Dungeons
    dungeon_settings: DungeonSettings,

    // Loot
    loot_tables: Vec<LootTable>,
    selected_loot_table: Option<u32>,

    // Preview
    preview: Option<GenerationPreview>,
    generation_history: Vec<GenerationPreview>,

    // ID counter
    next_id: u32,
}

impl Default for PcgPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: PcgTab::Seeds,

            current_seed: 12345,
            seed_input: "12345".to_string(),
            use_random_seed: false,
            saved_seeds: Vec::new(),

            enemy_types: Vec::new(),
            encounters: Vec::new(),
            selected_encounter: None,

            dungeon_settings: DungeonSettings::default(),

            loot_tables: Vec::new(),
            selected_loot_table: None,

            preview: None,
            generation_history: Vec::new(),

            next_id: 1,
        };

        panel
    }
}

impl PcgPanel {
    pub fn new() -> Self {
        Self::default()
    }

    fn create_sample_data(&mut self) {
        // Sample enemy types
        self.enemy_types.push(EnemyType {
            id: "goblin".to_string(),
            name: "Goblin".to_string(),
            threat_level: 0.5,
            min_count: 2,
            max_count: 5,
            spawn_weight: 2.0,
        });

        self.enemy_types.push(EnemyType {
            id: "orc".to_string(),
            name: "Orc".to_string(),
            threat_level: 1.0,
            min_count: 1,
            max_count: 3,
            spawn_weight: 1.0,
        });

        self.enemy_types.push(EnemyType {
            id: "troll".to_string(),
            name: "Troll".to_string(),
            threat_level: 2.0,
            min_count: 1,
            max_count: 1,
            spawn_weight: 0.3,
        });

        // Sample encounters
        let encounter_id = self.next_id();
        self.encounters.push(EncounterConfig {
            id: encounter_id,
            name: "Goblin Ambush".to_string(),
            difficulty: EncounterDifficulty::Easy,
            enemy_types: vec!["goblin".to_string()],
            min_enemies: 3,
            max_enemies: 6,
            spawn_radius: 8.0,
            reinforcement_chance: 0.2,
        });

        let encounter_id = self.next_id();
        self.encounters.push(EncounterConfig {
            id: encounter_id,
            name: "Orc Warband".to_string(),
            difficulty: EncounterDifficulty::Hard,
            enemy_types: vec!["orc".to_string(), "goblin".to_string()],
            min_enemies: 4,
            max_enemies: 8,
            spawn_radius: 12.0,
            reinforcement_chance: 0.4,
        });

        // Sample loot table
        let loot_id = self.next_id();
        self.loot_tables.push(LootTable {
            id: loot_id,
            name: "Common Chest".to_string(),
            entries: vec![
                LootEntry {
                    item_id: "gold".to_string(),
                    name: "Gold Coins".to_string(),
                    rarity: LootRarity::Common,
                    drop_weight: 5.0,
                    min_quantity: 10,
                    max_quantity: 50,
                },
                LootEntry {
                    item_id: "potion_health".to_string(),
                    name: "Health Potion".to_string(),
                    rarity: LootRarity::Common,
                    drop_weight: 2.0,
                    min_quantity: 1,
                    max_quantity: 2,
                },
                LootEntry {
                    item_id: "sword_iron".to_string(),
                    name: "Iron Sword".to_string(),
                    rarity: LootRarity::Uncommon,
                    drop_weight: 0.5,
                    min_quantity: 1,
                    max_quantity: 1,
                },
            ],
            guaranteed_drops: 1,
            bonus_rolls: 2,
        });

        // Saved seeds
        self.saved_seeds.push(("Tutorial Seed".to_string(), 42));
        self.saved_seeds.push(("Challenge Mode".to_string(), 666));
        self.saved_seeds.push(("Random Fun".to_string(), 31415));
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (PcgTab::Seeds, "[Dice] Seeds"),
                (PcgTab::Encounters, "⚔️ Encounters"),
                (PcgTab::Dungeons, "🏰 Dungeons"),
                (PcgTab::Loot, "[Gem] Loot"),
                (PcgTab::Preview, "Preview"),
                (PcgTab::History, "History"),
            ];

            for (tab, label) in tabs {
                let is_selected = self.active_tab == tab;
                let button = egui::Button::new(label).fill(if is_selected {
                    Color32::from_rgb(60, 100, 160)
                } else {
                    Color32::from_rgb(50, 50, 55)
                });

                if ui.add(button).clicked() {
                    self.active_tab = tab;
                }
            }
        });

        // Current seed display
        ui.horizontal(|ui| {
            ui.label(format!("[Dice] Current Seed: {}", self.current_seed));
            ui.separator();
            ui.label(format!("{} encounters", self.encounters.len()));
            ui.label(format!("[Gem] {} loot tables", self.loot_tables.len()));
        });

        ui.separator();
    }

    fn show_seeds_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Dice] Seed Management");
        ui.add_space(10.0);

        // Seed input
        ui.group(|ui| {
            ui.label(RichText::new("Current Seed").strong());

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.use_random_seed, "Random");

                if !self.use_random_seed {
                    ui.text_edit_singleline(&mut self.seed_input);
                    if ui.button("Apply").clicked() {
                        if let Ok(seed) = self.seed_input.parse::<u64>() {
                            info!(seed = seed, "PCG: seed changed");
                            self.current_seed = seed;
                        } else {
                            warn!(input = %self.seed_input, "PCG: invalid seed input");
                        }
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui.button("[Dice] New Random Seed").clicked() {
                    self.current_seed = rand_seed();
                    self.seed_input = self.current_seed.to_string();
                    info!(seed = self.current_seed, "PCG: generated random seed");
                }

                if ui.button("Copy").clicked() {
                    // Copy to clipboard
                }

                if ui.button("Save Seed").clicked() {
                    self.saved_seeds.push((
                        format!("Seed {}", self.saved_seeds.len() + 1),
                        self.current_seed,
                    ));
                }
            });
        });

        ui.add_space(10.0);

        // Saved seeds
        ui.group(|ui| {
            ui.label(RichText::new("Saved Seeds").strong());

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for (name, seed) in &self.saved_seeds.clone() {
                        ui.horizontal(|ui| {
                            ui.label(name);
                            ui.label(
                                RichText::new(format!("{}", seed))
                                    .monospace()
                                    .color(Color32::GRAY),
                            );
                            if ui.button("Load").clicked() {
                                self.current_seed = *seed;
                                self.seed_input = seed.to_string();
                            }
                        });
                    }
                });
        });

        ui.add_space(10.0);

        // Seed utilities
        ui.group(|ui| {
            ui.label(RichText::new("Seed Utilities").strong());

            ui.horizontal(|ui| {
                if ui.button("+ 1").clicked() {
                    self.current_seed = self.current_seed.wrapping_add(1);
                    self.seed_input = self.current_seed.to_string();
                }
                if ui.button("× 2").clicked() {
                    self.current_seed = self.current_seed.wrapping_mul(2);
                    self.seed_input = self.current_seed.to_string();
                }
                if ui.button("Hash").clicked() {
                    self.current_seed = simple_hash(self.current_seed);
                    self.seed_input = self.current_seed.to_string();
                }
            });
        });
    }

    fn show_encounters_tab(&mut self, ui: &mut Ui) {
        ui.heading("⚔️ Encounter Configuration");
        ui.add_space(10.0);

        // Add encounter button
        ui.horizontal(|ui| {
            if ui.button("+ New Encounter").clicked() {
                let id = self.next_id();
                info!(encounter_id = id, "PCG: added new encounter rule");
                self.encounters.push(EncounterConfig {
                    id,
                    name: format!("Encounter {}", id),
                    ..Default::default()
                });
            }

            if ui.button("[Dice] Generate Random").clicked() {
                info!("PCG: triggered random encounter generation");
                self.generate_random_encounter();
            }
        });

        ui.add_space(10.0);

        // Enemy types panel
        ui.collapsing("👹 Enemy Types", |ui| {
            for enemy in &self.enemy_types {
                ui.horizontal(|ui| {
                    ui.label(&enemy.name);
                    ui.label(
                        RichText::new(format!("[Zap]{:.1}", enemy.threat_level))
                            .small()
                            .color(Color32::YELLOW),
                    );
                    ui.label(
                        RichText::new(format!("({}-{})", enemy.min_count, enemy.max_count))
                            .small()
                            .color(Color32::GRAY),
                    );
                });
            }
        });

        ui.add_space(10.0);

        // Encounters list
        egui::ScrollArea::vertical()
            .max_height(250.0)
            .show(ui, |ui| {
                for encounter in &mut self.encounters {
                    let is_selected = self.selected_encounter == Some(encounter.id);

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let diff_color = encounter.difficulty.color();
                            let color_rect = ui
                                .allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover())
                                .0;
                            ui.painter().rect_filled(color_rect, 2.0, diff_color);

                            if ui.selectable_label(is_selected, &encounter.name).clicked() {
                                self.selected_encounter = Some(encounter.id);
                            }

                            ui.label(
                                RichText::new(format!("{:?}", encounter.difficulty))
                                    .small()
                                    .color(diff_color),
                            );
                        });

                        if is_selected {
                            egui::Grid::new(format!("encounter_{}", encounter.id))
                                .num_columns(2)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("Name:");
                                    ui.text_edit_singleline(&mut encounter.name);
                                    ui.end_row();

                                    ui.label("Difficulty:");
                                    egui::ComboBox::from_id_salt(format!("diff_{}", encounter.id))
                                        .selected_text(format!("{:?}", encounter.difficulty))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(
                                                &mut encounter.difficulty,
                                                EncounterDifficulty::Trivial,
                                                "Trivial",
                                            );
                                            ui.selectable_value(
                                                &mut encounter.difficulty,
                                                EncounterDifficulty::Easy,
                                                "Easy",
                                            );
                                            ui.selectable_value(
                                                &mut encounter.difficulty,
                                                EncounterDifficulty::Medium,
                                                "Medium",
                                            );
                                            ui.selectable_value(
                                                &mut encounter.difficulty,
                                                EncounterDifficulty::Hard,
                                                "Hard",
                                            );
                                            ui.selectable_value(
                                                &mut encounter.difficulty,
                                                EncounterDifficulty::Deadly,
                                                "Deadly",
                                            );
                                            ui.selectable_value(
                                                &mut encounter.difficulty,
                                                EncounterDifficulty::Boss,
                                                "Boss",
                                            );
                                        });
                                    ui.end_row();

                                    ui.label("Enemies:");
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut encounter.min_enemies)
                                                .prefix("Min:")
                                                .range(1..=20),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut encounter.max_enemies)
                                                .prefix("Max:")
                                                .range(1..=50),
                                        );
                                    });
                                    ui.end_row();

                                    ui.label("Spawn Radius:");
                                    ui.add(
                                        egui::DragValue::new(&mut encounter.spawn_radius)
                                            .speed(0.5)
                                            .range(1.0..=50.0),
                                    );
                                    ui.end_row();

                                    ui.label("Reinforcement:");
                                    ui.add(
                                        egui::Slider::new(
                                            &mut encounter.reinforcement_chance,
                                            0.0..=1.0,
                                        )
                                        .show_value(true),
                                    );
                                    ui.end_row();
                                });
                        }
                    });
                }
            });
    }

    fn show_dungeons_tab(&mut self, ui: &mut Ui) {
        ui.heading("🏰 Dungeon Generation");
        ui.add_space(10.0);

        // Generation controls
        ui.horizontal(|ui| {
            if ui.button("[Dice] Generate Dungeon").clicked() {
                info!(
                    seed = self.current_seed,
                    "PCG: triggered dungeon generation"
                );
                self.generate_dungeon_preview();
            }
            if ui.button("Reset Settings").clicked() {
                self.dungeon_settings = DungeonSettings::default();
            }
        });

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                // Size settings
                ui.group(|ui| {
                    ui.label(RichText::new("[Sq] Dungeon Size").strong());

                    egui::Grid::new("dungeon_size")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Width:");
                            ui.add(
                                egui::DragValue::new(&mut self.dungeon_settings.width)
                                    .range(20..=500),
                            );
                            ui.end_row();

                            ui.label("Height:");
                            ui.add(
                                egui::DragValue::new(&mut self.dungeon_settings.height)
                                    .range(20..=500),
                            );
                            ui.end_row();

                            ui.label("Room Count:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut self.dungeon_settings.room_count.0)
                                        .prefix("Min:")
                                        .range(1..=50),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut self.dungeon_settings.room_count.1)
                                        .prefix("Max:")
                                        .range(1..=100),
                                );
                            });
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Corridor settings
                ui.group(|ui| {
                    ui.label(RichText::new("[Rt] Corridors").strong());

                    egui::Grid::new("corridor_settings")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Corridor Width:");
                            ui.add(
                                egui::DragValue::new(&mut self.dungeon_settings.corridor_width)
                                    .range(1..=5),
                            );
                            ui.end_row();

                            ui.label("Branching:");
                            ui.add(egui::Slider::new(
                                &mut self.dungeon_settings.branching_factor,
                                0.0..=1.0,
                            ));
                            ui.end_row();

                            ui.label("Loop Chance:");
                            ui.add(egui::Slider::new(
                                &mut self.dungeon_settings.loop_chance,
                                0.0..=1.0,
                            ));
                            ui.end_row();
                        });
                });

                ui.add_space(10.0);

                // Room types
                ui.group(|ui| {
                    ui.label(RichText::new("🚪 Room Types").strong());

                    for config in &mut self.dungeon_settings.room_configs {
                        ui.horizontal(|ui| {
                            ui.label(config.room_type.icon());
                            ui.label(format!("{:?}", config.room_type));
                            ui.add(
                                egui::Slider::new(&mut config.spawn_chance, 0.0..=1.0)
                                    .show_value(false),
                            );
                            ui.label(format!("max: {}", config.max_count));
                        });
                    }
                });
            });
    }

    fn show_loot_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Gem] Loot Tables");
        ui.add_space(10.0);

        // Add loot table button
        if ui.button("+ New Loot Table").clicked() {
            let id = self.next_id();
            info!(table_id = id, "PCG: added new loot table");
            self.loot_tables.push(LootTable {
                id,
                name: format!("Loot Table {}", id),
                ..Default::default()
            });
        }

        ui.add_space(10.0);

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for table in &mut self.loot_tables {
                    let is_selected = self.selected_loot_table == Some(table.id);

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(is_selected, format!("[Gem] {}", table.name))
                                .clicked()
                            {
                                self.selected_loot_table = Some(table.id);
                            }
                            ui.label(
                                RichText::new(format!("({} items)", table.entries.len()))
                                    .small()
                                    .color(Color32::GRAY),
                            );
                        });

                        if is_selected {
                            egui::Grid::new(format!("loot_{}", table.id))
                                .num_columns(2)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("Name:");
                                    ui.text_edit_singleline(&mut table.name);
                                    ui.end_row();

                                    ui.label("Guaranteed Drops:");
                                    ui.add(
                                        egui::DragValue::new(&mut table.guaranteed_drops)
                                            .range(0..=10),
                                    );
                                    ui.end_row();

                                    ui.label("Bonus Rolls:");
                                    ui.add(
                                        egui::DragValue::new(&mut table.bonus_rolls).range(0..=10),
                                    );
                                    ui.end_row();
                                });

                            ui.add_space(5.0);
                            ui.label(RichText::new("Items:").strong());

                            for entry in &table.entries {
                                ui.horizontal(|ui| {
                                    let color_rect = ui
                                        .allocate_exact_size(
                                            Vec2::new(8.0, 8.0),
                                            egui::Sense::hover(),
                                        )
                                        .0;
                                    ui.painter()
                                        .rect_filled(color_rect, 2.0, entry.rarity.color());

                                    ui.label(&entry.name);
                                    ui.label(
                                        RichText::new(format!("w:{:.1}", entry.drop_weight))
                                            .small()
                                            .color(Color32::GRAY),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "({}-{})",
                                            entry.min_quantity, entry.max_quantity
                                        ))
                                        .small()
                                        .color(Color32::GRAY),
                                    );
                                });
                            }
                        }
                    });
                }
            });
    }

    fn show_preview_tab(&mut self, ui: &mut Ui) {
        ui.heading("Generation Preview");
        ui.add_space(10.0);

        // Generation type selector
        ui.horizontal(|ui| {
            ui.label("Type:");
            for gen_type in GenerationType::all() {
                if ui
                    .button(format!("{} {:?}", gen_type.icon(), gen_type))
                    .clicked()
                {
                    self.generate_preview(*gen_type);
                }
            }
        });

        ui.add_space(10.0);

        // Preview display
        if let Some(ref preview) = self.preview {
            ui.group(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} {:?} Preview",
                        preview.generation_type.icon(),
                        preview.generation_type
                    ))
                    .strong(),
                );
                ui.label(format!("Seed: {}", preview.seed));
                ui.label(format!(
                    "Generation Time: {:.2}ms",
                    preview.generation_time_ms
                ));

                ui.add_space(10.0);
                ui.label(&preview.result_summary);
            });
        } else {
            ui.label("Click a generation type above to preview results.");
        }
    }

    fn show_history_tab(&mut self, ui: &mut Ui) {
        ui.heading("Generation History");
        ui.add_space(10.0);

        if self.generation_history.is_empty() {
            ui.label("No generation history yet. Generate some content to see it here.");
        } else {
            if ui.button("Clear History").clicked() {
                self.generation_history.clear();
            }

            ui.add_space(10.0);

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for (i, preview) in self.generation_history.iter().enumerate().rev() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("#{}", i + 1));
                                ui.label(format!(
                                    "{} {:?}",
                                    preview.generation_type.icon(),
                                    preview.generation_type
                                ));
                                ui.label(
                                    RichText::new(format!("Seed: {}", preview.seed))
                                        .small()
                                        .color(Color32::GRAY),
                                );
                            });
                        });
                    }
                });
        }
    }

    fn generate_random_encounter(&mut self) {
        let id = self.next_id();
        let difficulty = match self.current_seed % 6 {
            0 => EncounterDifficulty::Trivial,
            1 => EncounterDifficulty::Easy,
            2 => EncounterDifficulty::Medium,
            3 => EncounterDifficulty::Hard,
            4 => EncounterDifficulty::Deadly,
            _ => EncounterDifficulty::Boss,
        };

        self.encounters.push(EncounterConfig {
            id,
            name: format!("Random Encounter {}", id),
            difficulty,
            min_enemies: 2 + (self.current_seed % 4) as u32,
            max_enemies: 5 + (self.current_seed % 8) as u32,
            spawn_radius: 8.0 + (self.current_seed % 10) as f32,
            ..Default::default()
        });

        self.current_seed = simple_hash(self.current_seed);
    }

    fn generate_dungeon_preview(&mut self) {
        let room_count = self.dungeon_settings.room_count.0
            + (self.current_seed
                % (self.dungeon_settings.room_count.1 - self.dungeon_settings.room_count.0 + 1)
                    as u64) as u32;

        let summary = format!(
            "Generated dungeon: {}x{} tiles, {} rooms, corridor width {}",
            self.dungeon_settings.width,
            self.dungeon_settings.height,
            room_count,
            self.dungeon_settings.corridor_width
        );

        self.preview = Some(GenerationPreview {
            seed: self.current_seed,
            generation_type: GenerationType::Dungeon,
            result_summary: summary,
            generation_time_ms: 12.5,
        });

        if let Some(ref preview) = self.preview {
            self.generation_history.push(preview.clone());
        }
    }

    fn generate_preview(&mut self, gen_type: GenerationType) {
        let summary = match gen_type {
            GenerationType::Encounter => format!(
                "Generated encounter with {} enemies",
                3 + self.current_seed % 5
            ),
            GenerationType::Dungeon => {
                format!("Generated dungeon with {} rooms", 8 + self.current_seed % 7)
            }
            GenerationType::Loot => format!("Generated loot: {} items", 2 + self.current_seed % 4),
            GenerationType::Terrain => "Generated terrain chunk 64x64".to_string(),
            GenerationType::Vegetation => format!(
                "Placed {} trees and {} bushes",
                50 + self.current_seed % 30,
                100 + self.current_seed % 50
            ),
            GenerationType::Props => format!("Placed {} props", 20 + self.current_seed % 15),
            GenerationType::NPC => format!(
                "Generated NPC with {} dialogue lines",
                5 + self.current_seed % 10
            ),
        };

        self.preview = Some(GenerationPreview {
            seed: self.current_seed,
            generation_type: gen_type,
            result_summary: summary,
            generation_time_ms: 5.0 + (self.current_seed % 20) as f32,
        });

        if let Some(ref preview) = self.preview {
            self.generation_history.push(preview.clone());
        }
    }

    // Getters for testing
    pub fn current_seed(&self) -> u64 {
        self.current_seed
    }

    pub fn encounter_count(&self) -> usize {
        self.encounters.len()
    }

    pub fn loot_table_count(&self) -> usize {
        self.loot_tables.len()
    }

    pub fn enemy_type_count(&self) -> usize {
        self.enemy_types.len()
    }

    pub fn saved_seed_count(&self) -> usize {
        self.saved_seeds.len()
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.current_seed = seed;
        self.seed_input = seed.to_string();
    }

    pub fn add_encounter(&mut self, name: &str, difficulty: EncounterDifficulty) -> u32 {
        let id = self.next_id();
        self.encounters.push(EncounterConfig {
            id,
            name: name.to_string(),
            difficulty,
            ..Default::default()
        });
        id
    }

    pub fn add_loot_table(&mut self, name: &str) -> u32 {
        let id = self.next_id();
        self.loot_tables.push(LootTable {
            id,
            name: name.to_string(),
            ..Default::default()
        });
        id
    }
}

fn rand_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn simple_hash(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

impl Panel for PcgPanel {
    fn name(&self) -> &'static str {
        "PCG"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            PcgTab::Seeds => self.show_seeds_tab(ui),
            PcgTab::Encounters => self.show_encounters_tab(ui),
            PcgTab::Dungeons => self.show_dungeons_tab(ui),
            PcgTab::Loot => self.show_loot_tab(ui),
            PcgTab::Preview => self.show_preview_tab(ui),
            PcgTab::History => self.show_history_tab(ui),
        }
    }

    fn update(&mut self) {
        if self.use_random_seed {
            self.current_seed = rand_seed();
            self.seed_input = self.current_seed.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcg_panel_creation() {
        let panel = PcgPanel::new();
        assert!(panel.current_seed() > 0);
    }

    #[test]
    fn test_default_sample_data() {
        let mut panel = PcgPanel::new();
        panel.create_sample_data();
        assert!(panel.enemy_type_count() >= 3);
        assert!(panel.encounter_count() >= 2);
        assert!(panel.loot_table_count() >= 1);
        assert!(panel.saved_seed_count() >= 3);
    }

    #[test]
    fn test_set_seed() {
        let mut panel = PcgPanel::new();
        panel.set_seed(999);
        assert_eq!(panel.current_seed(), 999);
    }

    #[test]
    fn test_add_encounter() {
        let mut panel = PcgPanel::new();
        let initial_count = panel.encounter_count();

        let id = panel.add_encounter("Test Encounter", EncounterDifficulty::Hard);
        assert!(id > 0);
        assert_eq!(panel.encounter_count(), initial_count + 1);
    }

    #[test]
    fn test_add_loot_table() {
        let mut panel = PcgPanel::new();
        let initial_count = panel.loot_table_count();

        let id = panel.add_loot_table("Boss Loot");
        assert!(id > 0);
        assert_eq!(panel.loot_table_count(), initial_count + 1);
    }

    #[test]
    fn test_encounter_difficulty_colors() {
        assert_eq!(
            EncounterDifficulty::Easy.color(),
            Color32::from_rgb(100, 200, 100)
        );
        assert_eq!(
            EncounterDifficulty::Deadly.color(),
            Color32::from_rgb(255, 80, 80)
        );
    }

    #[test]
    fn test_loot_rarity_colors() {
        assert_eq!(LootRarity::Common.color(), Color32::from_rgb(180, 180, 180));
        assert_eq!(
            LootRarity::Legendary.color(),
            Color32::from_rgb(255, 180, 50)
        );
    }

    #[test]
    fn test_generation_type_icons() {
        assert_eq!(GenerationType::Encounter.icon(), "⚔️");
        assert_eq!(GenerationType::Dungeon.icon(), "🏰");
        assert_eq!(GenerationType::Loot.icon(), "[Gem]");
    }

    #[test]
    fn test_room_type_icons() {
        assert_eq!(RoomType::Entrance.icon(), "🚶");
        assert_eq!(RoomType::Boss.icon(), "👹");
        assert_eq!(RoomType::Treasure.icon(), "[Gold]");
    }

    #[test]
    fn test_panel_trait_implementation() {
        let panel = PcgPanel::new();
        assert_eq!(panel.name(), "PCG");
    }

    // ========== GenerationType Tests ==========

    #[test]
    fn test_generation_type_display() {
        for gen in GenerationType::all() {
            let display = format!("{}", gen);
            assert!(display.contains(gen.name()));
        }
    }

    #[test]
    fn test_generation_type_all_variants() {
        let all = GenerationType::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_generation_type_hash() {
        use std::collections::HashSet;
        let set: HashSet<GenerationType> = GenerationType::all().iter().copied().collect();
        assert_eq!(set.len(), 7);
    }

    #[test]
    fn test_generation_type_name() {
        assert_eq!(GenerationType::Encounter.name(), "Encounter");
        assert_eq!(GenerationType::Dungeon.name(), "Dungeon");
        assert_eq!(GenerationType::NPC.name(), "NPC");
    }

    // ========== EncounterDifficulty Tests ==========

    #[test]
    fn test_encounter_difficulty_display() {
        for diff in EncounterDifficulty::all() {
            let display = format!("{}", diff);
            assert!(display.contains(diff.name()));
        }
    }

    #[test]
    fn test_encounter_difficulty_all_variants() {
        let all = EncounterDifficulty::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_encounter_difficulty_hash() {
        use std::collections::HashSet;
        let set: HashSet<EncounterDifficulty> =
            EncounterDifficulty::all().iter().copied().collect();
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn test_encounter_difficulty_name() {
        assert_eq!(EncounterDifficulty::Trivial.name(), "Trivial");
        assert_eq!(EncounterDifficulty::Boss.name(), "Boss");
    }

    // ========== RoomType Tests ==========

    #[test]
    fn test_room_type_display() {
        for room in RoomType::all() {
            let display = format!("{}", room);
            assert!(display.contains(room.name()));
        }
    }

    #[test]
    fn test_room_type_all_variants() {
        let all = RoomType::all();
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn test_room_type_hash() {
        use std::collections::HashSet;
        let set: HashSet<RoomType> = RoomType::all().iter().copied().collect();
        assert_eq!(set.len(), 8);
    }

    #[test]
    fn test_room_type_is_special() {
        assert!(!RoomType::Normal.is_special());
        assert!(!RoomType::Corridor.is_special());
        assert!(RoomType::Entrance.is_special());
        assert!(RoomType::Boss.is_special());
        assert!(RoomType::Treasure.is_special());
        assert!(RoomType::Secret.is_special());
    }

    // ========== LootRarity Tests ==========

    #[test]
    fn test_loot_rarity_display() {
        for rarity in LootRarity::all() {
            let display = format!("{}", rarity);
            assert!(display.contains(rarity.name()));
        }
    }

    #[test]
    fn test_loot_rarity_all_variants() {
        let all = LootRarity::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn test_loot_rarity_hash() {
        use std::collections::HashSet;
        let set: HashSet<LootRarity> = LootRarity::all().iter().copied().collect();
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn test_loot_rarity_name() {
        assert_eq!(LootRarity::Common.name(), "Common");
        assert_eq!(LootRarity::Legendary.name(), "Legendary");
    }

    // ========== PcgTab Tests ==========

    #[test]
    fn test_pcg_tab_display() {
        for tab in PcgTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()));
        }
    }

    #[test]
    fn test_pcg_tab_all_variants() {
        let all = PcgTab::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_pcg_tab_hash() {
        use std::collections::HashSet;
        let set: HashSet<PcgTab> = PcgTab::all().iter().copied().collect();
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn test_pcg_tab_default() {
        assert_eq!(PcgTab::default(), PcgTab::Seeds);
    }
}
