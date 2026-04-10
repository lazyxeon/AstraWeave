//! Localization Panel for the editor
//!
//! Provides localization management:
//! - String table management
//! - Language/locale configuration
//! - Translation workflow
//! - Missing translation detection
//! - Export/import support (CSV, XLIFF, PO)
//! - Real-time preview

use egui::{Color32, RichText, Ui};
use std::collections::HashMap;
use tracing::info;

use crate::panels::Panel;

/// Supported languages/locales
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Language {
    #[default]
    English,
    Spanish,
    French,
    German,
    Italian,
    Portuguese,
    Russian,
    Japanese,
    Korean,
    SimplifiedChinese,
    TraditionalChinese,
    Arabic,
    Custom(u32), // Custom language ID
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.flag_emoji(), self.name())
    }
}

impl Language {
    pub fn all() -> &'static [Language] {
        &[
            Language::English,
            Language::Spanish,
            Language::French,
            Language::German,
            Language::Italian,
            Language::Portuguese,
            Language::Russian,
            Language::Japanese,
            Language::Korean,
            Language::SimplifiedChinese,
            Language::TraditionalChinese,
            Language::Arabic,
        ]
    }

    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::German => "de",
            Language::Italian => "it",
            Language::Portuguese => "pt",
            Language::Russian => "ru",
            Language::Japanese => "ja",
            Language::Korean => "ko",
            Language::SimplifiedChinese => "zh-CN",
            Language::TraditionalChinese => "zh-TW",
            Language::Arabic => "ar",
            Language::Custom(_) => "custom",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Spanish => "Español",
            Language::French => "Français",
            Language::German => "Deutsch",
            Language::Italian => "Italiano",
            Language::Portuguese => "Português",
            Language::Russian => "Русский",
            Language::Japanese => "日本語",
            Language::Korean => "한국어",
            Language::SimplifiedChinese => "简体中文",
            Language::TraditionalChinese => "繁體中文",
            Language::Arabic => "العربية",
            Language::Custom(_) => "Custom",
        }
    }

    pub fn flag_emoji(&self) -> &'static str {
        match self {
            Language::English => "🇬🇧",
            Language::Spanish => "🇪🇸",
            Language::French => "🇫🇷",
            Language::German => "🇩🇪",
            Language::Italian => "🇮🇹",
            Language::Portuguese => "🇧🇷",
            Language::Russian => "🇷🇺",
            Language::Japanese => "🇯🇵",
            Language::Korean => "🇰🇷",
            Language::SimplifiedChinese => "🇨🇳",
            Language::TraditionalChinese => "🇹🇼",
            Language::Arabic => "🇸🇦",
            Language::Custom(_) => "🏳️",
        }
    }
}

/// String entry in localization table
#[derive(Debug, Clone)]
pub struct LocalizedString {
    pub key: String,
    pub category: StringCategory,
    pub context: String,
    pub translations: HashMap<Language, String>,
    pub needs_review: bool,
    pub max_length: Option<u32>,
    pub plural_forms: Option<PluralForms>,
}

impl Default for LocalizedString {
    fn default() -> Self {
        Self {
            key: String::new(),
            category: StringCategory::Ui,
            context: String::new(),
            translations: HashMap::new(),
            needs_review: false,
            max_length: None,
            plural_forms: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StringCategory {
    #[default]
    Ui,
    Dialogue,
    Quest,
    Item,
    Achievement,
    Tutorial,
    System,
    Error,
}

impl std::fmt::Display for StringCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl StringCategory {
    pub fn all() -> &'static [StringCategory] {
        &[
            StringCategory::Ui,
            StringCategory::Dialogue,
            StringCategory::Quest,
            StringCategory::Item,
            StringCategory::Achievement,
            StringCategory::Tutorial,
            StringCategory::System,
            StringCategory::Error,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            StringCategory::Ui => "UI",
            StringCategory::Dialogue => "Dialogue",
            StringCategory::Quest => "Quest",
            StringCategory::Item => "Item",
            StringCategory::Achievement => "Achievement",
            StringCategory::Tutorial => "Tutorial",
            StringCategory::System => "System",
            StringCategory::Error => "Error",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            StringCategory::Ui => "[Mon]",
            StringCategory::Dialogue => "[Chat]",
            StringCategory::Quest => "📜",
            StringCategory::Item => "🎒",
            StringCategory::Achievement => "🏆",
            StringCategory::Tutorial => "📖",
            StringCategory::System => "⚙️",
            StringCategory::Error => "[x]",
        }
    }
}

/// Plural forms for a string
#[derive(Debug, Clone, Default)]
pub struct PluralForms {
    pub zero: String,
    pub one: String,
    pub two: String,
    pub few: String,
    pub many: String,
    pub other: String,
}

/// Export format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExportFormat {
    #[default]
    Csv,
    Xliff,
    Po,
    Json,
    Resx,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl ExportFormat {
    pub fn all() -> &'static [ExportFormat] {
        &[
            ExportFormat::Csv,
            ExportFormat::Xliff,
            ExportFormat::Po,
            ExportFormat::Json,
            ExportFormat::Resx,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "CSV",
            ExportFormat::Xliff => "XLIFF",
            ExportFormat::Po => "PO (Gettext)",
            ExportFormat::Json => "JSON",
            ExportFormat::Resx => "RESX (.NET)",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "[Chart]",
            ExportFormat::Xliff => "[Doc]",
            ExportFormat::Po => "[Dir]",
            ExportFormat::Json => "[Edit]",
            ExportFormat::Resx => "[Pkg]",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Csv => ".csv",
            ExportFormat::Xliff => ".xlf",
            ExportFormat::Po => ".po",
            ExportFormat::Json => ".json",
            ExportFormat::Resx => ".resx",
        }
    }
}

/// Panel tabs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LocalizationTab {
    #[default]
    Strings,
    Languages,
    Statistics,
    ImportExport,
    Settings,
}

impl std::fmt::Display for LocalizationTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl LocalizationTab {
    pub fn all() -> &'static [LocalizationTab] {
        &[
            LocalizationTab::Strings,
            LocalizationTab::Languages,
            LocalizationTab::Statistics,
            LocalizationTab::ImportExport,
            LocalizationTab::Settings,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            LocalizationTab::Strings => "Strings",
            LocalizationTab::Languages => "Languages",
            LocalizationTab::Statistics => "Statistics",
            LocalizationTab::ImportExport => "Import/Export",
            LocalizationTab::Settings => "Settings",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            LocalizationTab::Strings => "[Edit]",
            LocalizationTab::Languages => "[Glb]",
            LocalizationTab::Statistics => "[Chart]",
            LocalizationTab::ImportExport => "📥",
            LocalizationTab::Settings => "⚙️",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════════
// ACTION SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════════

/// Actions that can be triggered from the localization panel
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LocalizationAction {
    // Tab navigation
    SetActiveTab(LocalizationTab),

    // String operations
    AddString,
    RemoveString(u32),
    SelectString(u32),
    DuplicateString(u32),
    SetStringKey(u32, String),
    SetStringCategory(u32, StringCategory),
    SetStringContext(u32, String),
    SetTranslation(u32, Language, String),
    MarkNeedsReview(u32, bool),

    // Language operations
    AddLanguage(Language),
    RemoveLanguage(Language),
    SetSourceLanguage(Language),
    SetPreviewLanguage(Language),
    ToggleLanguageEnabled(Language, bool),

    // Filter operations
    SetFilterText(String),
    SetFilterCategory(Option<StringCategory>),
    ToggleMissingOnly(bool),
    ToggleNeedsReviewOnly(bool),
    ClearFilters,

    // Import/Export
    SetExportFormat(ExportFormat),
    AddExportLanguage(Language),
    RemoveExportLanguage(Language),
    Export,
    Import(String),
    SetImportPath(String),

    // Batch operations
    AutoTranslate(Language),
    CopyFromLanguage(Language, Language),
    ClearLanguage(Language),
    ValidateStrings,
    FindDuplicates,
    RefreshStatistics,
}

impl std::fmt::Display for LocalizationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalizationAction::SetActiveTab(tab) => write!(f, "Set tab: {}", tab),
            LocalizationAction::AddString => write!(f, "Add string"),
            LocalizationAction::RemoveString(id) => write!(f, "Remove string {}", id),
            LocalizationAction::SelectString(id) => write!(f, "Select string {}", id),
            LocalizationAction::DuplicateString(id) => write!(f, "Duplicate string {}", id),
            LocalizationAction::SetStringKey(id, key) => {
                write!(f, "Set string {} key: {}", id, key)
            }
            LocalizationAction::SetStringCategory(id, cat) => {
                write!(f, "Set string {} category: {}", id, cat)
            }
            LocalizationAction::SetStringContext(id, ctx) => {
                write!(f, "Set string {} context: {}", id, ctx)
            }
            LocalizationAction::SetTranslation(id, lang, _) => {
                write!(f, "Set translation {} for {}", id, lang)
            }
            LocalizationAction::MarkNeedsReview(id, b) => {
                write!(f, "Mark string {} review: {}", id, b)
            }
            LocalizationAction::AddLanguage(lang) => write!(f, "Add language: {}", lang),
            LocalizationAction::RemoveLanguage(lang) => write!(f, "Remove language: {}", lang),
            LocalizationAction::SetSourceLanguage(lang) => write!(f, "Set source: {}", lang),
            LocalizationAction::SetPreviewLanguage(lang) => write!(f, "Preview: {}", lang),
            LocalizationAction::ToggleLanguageEnabled(lang, b) => {
                write!(f, "Toggle {} enabled: {}", lang, b)
            }
            LocalizationAction::SetFilterText(text) => write!(f, "Filter: {}", text),
            LocalizationAction::SetFilterCategory(cat) => write!(f, "Filter category: {:?}", cat),
            LocalizationAction::ToggleMissingOnly(b) => write!(f, "Missing only: {}", b),
            LocalizationAction::ToggleNeedsReviewOnly(b) => write!(f, "Needs review only: {}", b),
            LocalizationAction::ClearFilters => write!(f, "Clear filters"),
            LocalizationAction::SetExportFormat(fmt) => write!(f, "Export format: {}", fmt),
            LocalizationAction::AddExportLanguage(lang) => write!(f, "Add export lang: {}", lang),
            LocalizationAction::RemoveExportLanguage(lang) => {
                write!(f, "Remove export lang: {}", lang)
            }
            LocalizationAction::Export => write!(f, "Export"),
            LocalizationAction::Import(path) => write!(f, "Import from: {}", path),
            LocalizationAction::SetImportPath(path) => write!(f, "Set import path: {}", path),
            LocalizationAction::AutoTranslate(lang) => write!(f, "Auto-translate to: {}", lang),
            LocalizationAction::CopyFromLanguage(from, to) => write!(f, "Copy {} to {}", from, to),
            LocalizationAction::ClearLanguage(lang) => write!(f, "Clear language: {}", lang),
            LocalizationAction::ValidateStrings => write!(f, "Validate strings"),
            LocalizationAction::FindDuplicates => write!(f, "Find duplicates"),
            LocalizationAction::RefreshStatistics => write!(f, "Refresh statistics"),
        }
    }
}

/// Main Localization Panel
pub struct LocalizationPanel {
    active_tab: LocalizationTab,

    // Strings
    strings: Vec<LocalizedString>,
    selected_string: Option<usize>,
    current_string: LocalizedString,

    // Languages
    enabled_languages: Vec<Language>,
    source_language: Language,
    active_preview_language: Language,

    // Filters
    filter_text: String,
    filter_category: Option<StringCategory>,
    show_missing_only: bool,
    show_needs_review_only: bool,

    // Import/Export
    export_format: ExportFormat,
    export_languages: Vec<Language>,
    import_path: String,

    // Action system
    actions: Vec<LocalizationAction>,
}

impl Default for LocalizationPanel {
    fn default() -> Self {
        let panel = Self {
            active_tab: LocalizationTab::Strings,

            strings: Vec::new(),
            selected_string: None,
            current_string: LocalizedString::default(),

            enabled_languages: vec![
                Language::English,
                Language::Spanish,
                Language::French,
                Language::German,
                Language::Japanese,
            ],
            source_language: Language::English,
            active_preview_language: Language::English,

            filter_text: String::new(),
            filter_category: None,
            show_missing_only: false,
            show_needs_review_only: false,

            export_format: ExportFormat::Csv,
            export_languages: vec![Language::English],
            import_path: String::new(),

            actions: Vec::new(),
        };

        panel
    }
}

impl LocalizationPanel {
    pub fn new() -> Self {
        Self::default()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Action System
    // ─────────────────────────────────────────────────────────────────────────────

    /// Queue an action for later processing
    pub fn queue_action(&mut self, action: LocalizationAction) {
        self.actions.push(action);
    }

    /// Check if there are pending actions
    pub fn has_pending_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// Get pending actions without consuming them
    pub fn pending_actions(&self) -> &[LocalizationAction] {
        &self.actions
    }

    /// Take all pending actions, clearing the queue
    pub fn take_actions(&mut self) -> Vec<LocalizationAction> {
        std::mem::take(&mut self.actions)
    }

    fn create_sample_data(&mut self) {
        // UI strings
        let mut translations = HashMap::new();
        translations.insert(Language::English, "Play Game".to_string());
        translations.insert(Language::Spanish, "Jugar".to_string());
        translations.insert(Language::French, "Jouer".to_string());
        translations.insert(Language::German, "Spielen".to_string());
        translations.insert(Language::Japanese, "ゲームを始める".to_string());

        self.strings.push(LocalizedString {
            key: "menu.play".to_string(),
            category: StringCategory::Ui,
            context: "Main menu button".to_string(),
            translations,
            ..Default::default()
        });

        // Dialogue
        let mut translations = HashMap::new();
        translations.insert(
            Language::English,
            "Welcome, traveler! What brings you to our village?".to_string(),
        );
        translations.insert(
            Language::Spanish,
            "¡Bienvenido, viajero! ¿Qué te trae a nuestro pueblo?".to_string(),
        );

        self.strings.push(LocalizedString {
            key: "npc.elder.greeting".to_string(),
            category: StringCategory::Dialogue,
            context: "Village elder greeting".to_string(),
            translations,
            needs_review: true,
            ..Default::default()
        });

        // Quest
        let mut translations = HashMap::new();
        translations.insert(Language::English, "Defeat the Dragon".to_string());
        translations.insert(Language::French, "Vaincre le Dragon".to_string());

        self.strings.push(LocalizedString {
            key: "quest.dragon.title".to_string(),
            category: StringCategory::Quest,
            context: "Main quest title".to_string(),
            translations,
            max_length: Some(30),
            ..Default::default()
        });

        // Item
        let mut translations = HashMap::new();
        translations.insert(Language::English, "Legendary Sword of Fire".to_string());

        self.strings.push(LocalizedString {
            key: "item.sword_fire.name".to_string(),
            category: StringCategory::Item,
            context: "Weapon name".to_string(),
            translations,
            ..Default::default()
        });

        // System
        let mut translations = HashMap::new();
        translations.insert(Language::English, "Save game?".to_string());
        translations.insert(Language::Spanish, "¿Guardar partida?".to_string());
        translations.insert(Language::French, "Sauvegarder la partie?".to_string());
        translations.insert(Language::German, "Spiel speichern?".to_string());
        translations.insert(Language::Japanese, "ゲームをセーブしますか?".to_string());

        self.strings.push(LocalizedString {
            key: "system.save_prompt".to_string(),
            category: StringCategory::System,
            context: "Save confirmation dialog".to_string(),
            translations,
            ..Default::default()
        });

        self.current_string = self.strings.first().cloned().unwrap_or_default();
        self.selected_string = Some(0);
    }

    fn show_tab_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let tabs = [
                (LocalizationTab::Strings, "[Edit] Strings"),
                (LocalizationTab::Languages, "[Glb] Languages"),
                (LocalizationTab::Statistics, "Statistics"),
                (LocalizationTab::ImportExport, "Import/Export"),
                (LocalizationTab::Settings, "Settings"),
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

        // Status bar
        let missing = self.count_missing_translations();
        let needs_review = self.strings.iter().filter(|s| s.needs_review).count();

        ui.horizontal(|ui| {
            ui.label(format!("[Edit] {} strings", self.strings.len()));
            ui.separator();
            ui.label(format!("[Glb] {} languages", self.enabled_languages.len()));
            ui.separator();
            if missing > 0 {
                ui.label(RichText::new(format!("{} missing", missing)).color(Color32::YELLOW));
            }
            if needs_review > 0 {
                ui.separator();
                ui.label(
                    RichText::new(format!("{} need review", needs_review))
                        .color(Color32::LIGHT_BLUE),
                );
            }
        });

        ui.separator();
    }

    fn count_missing_translations(&self) -> usize {
        let mut missing = 0;
        for s in &self.strings {
            for lang in &self.enabled_languages {
                if !s.translations.contains_key(lang) {
                    missing += 1;
                }
            }
        }
        missing
    }

    fn show_strings_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Edit] String Table");
        ui.add_space(5.0);

        // Filters
        ui.horizontal(|ui| {
            ui.label("[Srch]");
            ui.add(egui::TextEdit::singleline(&mut self.filter_text).hint_text("Search..."));

            ui.separator();

            egui::ComboBox::from_id_salt("filter_category")
                .selected_text(
                    self.filter_category
                        .map(|c| format!("{} {:?}", c.icon(), c))
                        .unwrap_or_else(|| "All Categories".to_string()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filter_category, None, "All");
                    for cat in [
                        StringCategory::Ui,
                        StringCategory::Dialogue,
                        StringCategory::Quest,
                        StringCategory::Item,
                        StringCategory::Achievement,
                        StringCategory::Tutorial,
                        StringCategory::System,
                        StringCategory::Error,
                    ] {
                        if ui
                            .selectable_value(
                                &mut self.filter_category,
                                Some(cat),
                                format!("{} {:?}", cat.icon(), cat),
                            )
                            .clicked()
                        {}
                    }
                });

            ui.checkbox(&mut self.show_missing_only, "Missing Only");
            ui.checkbox(&mut self.show_needs_review_only, "Needs Review");
        });

        ui.add_space(10.0);

        ui.columns(2, |cols| {
            // Left: String list
            cols[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Strings").strong());
                    if ui.button("+ New").clicked() {
                        let key = format!("new.string_{}", self.strings.len());
                        info!(key = %key, "Localization: added string key");
                        self.strings.push(LocalizedString {
                            key,
                            ..Default::default()
                        });
                    }
                });

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (i, s) in self.strings.iter().enumerate() {
                            // Apply filters
                            if !self.filter_text.is_empty()
                                && !s
                                    .key
                                    .to_lowercase()
                                    .contains(&self.filter_text.to_lowercase())
                            {
                                continue;
                            }

                            if let Some(cat) = self.filter_category {
                                if s.category != cat {
                                    continue;
                                }
                            }

                            if self.show_needs_review_only && !s.needs_review {
                                continue;
                            }

                            let has_missing = self
                                .enabled_languages
                                .iter()
                                .any(|l| !s.translations.contains_key(l));

                            if self.show_missing_only && !has_missing {
                                continue;
                            }

                            let is_selected = self.selected_string == Some(i);

                            let status = if s.needs_review {
                                "👁️"
                            } else if has_missing {
                                "[!]"
                            } else {
                                "[ok]"
                            };

                            let label = format!("{} {} {}", s.category.icon(), s.key, status);

                            if ui.selectable_label(is_selected, label).clicked() {
                                self.selected_string = Some(i);
                                self.current_string = s.clone();
                            }
                        }
                    });
            });

            // Right: String editor
            cols[1].group(|ui| {
                ui.label(RichText::new("✏️ Edit String").strong());

                egui::Grid::new("string_editor")
                    .num_columns(2)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Key:");
                        ui.text_edit_singleline(&mut self.current_string.key);
                        ui.end_row();

                        ui.label("Category:");
                        egui::ComboBox::from_id_salt("string_category")
                            .selected_text(format!(
                                "{} {:?}",
                                self.current_string.category.icon(),
                                self.current_string.category
                            ))
                            .show_ui(ui, |ui| {
                                for cat in [
                                    StringCategory::Ui,
                                    StringCategory::Dialogue,
                                    StringCategory::Quest,
                                    StringCategory::Item,
                                    StringCategory::Achievement,
                                    StringCategory::Tutorial,
                                    StringCategory::System,
                                    StringCategory::Error,
                                ] {
                                    ui.selectable_value(
                                        &mut self.current_string.category,
                                        cat,
                                        format!("{} {:?}", cat.icon(), cat),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Context:");
                        ui.text_edit_singleline(&mut self.current_string.context);
                        ui.end_row();

                        ui.label("Needs Review:");
                        ui.checkbox(&mut self.current_string.needs_review, "");
                        ui.end_row();
                    });

                ui.add_space(10.0);

                // Translations
                ui.label(RichText::new("[Glb] Translations").strong());

                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for lang in &self.enabled_languages.clone() {
                            let flag = lang.flag_emoji();
                            let name = lang.name();

                            let has_translation =
                                self.current_string.translations.contains_key(lang);
                            let status = if has_translation { "[ok]" } else { "[!]" };

                            ui.horizontal(|ui| {
                                ui.label(format!("{} {} {}", flag, name, status));
                            });

                            let translation =
                                self.current_string.translations.entry(*lang).or_default();

                            ui.add(egui::TextEdit::multiline(translation).desired_rows(2));
                            ui.add_space(5.0);
                        }
                    });
            });
        });
    }

    fn show_languages_tab(&mut self, ui: &mut Ui) {
        ui.heading("[Glb] Languages");
        ui.add_space(10.0);

        ui.columns(2, |cols| {
            // Left: Enabled languages
            cols[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Enabled Languages").strong());
                    if ui.button("+ Add").clicked() {
                        // Show language picker
                    }
                });

                for lang in &self.enabled_languages.clone() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} {}", lang.flag_emoji(), lang.name()));
                        ui.label(format!("({})", lang.code()));

                        if *lang == self.source_language {
                            ui.label(RichText::new("[Source]").color(Color32::LIGHT_GREEN));
                        }

                        if ui.button("[Del]").clicked() {
                            info!(language = %lang.name(), "Localization: removed language");
                            self.enabled_languages.retain(|l| l != lang);
                        }
                    });
                }
            });

            // Right: Source language & settings
            cols[1].group(|ui| {
                ui.label(RichText::new("Language Settings").strong());

                ui.horizontal(|ui| {
                    ui.label("Source Language:");
                    egui::ComboBox::from_id_salt("source_lang")
                        .selected_text(format!(
                            "{} {}",
                            self.source_language.flag_emoji(),
                            self.source_language.name()
                        ))
                        .show_ui(ui, |ui| {
                            for lang in &self.enabled_languages.clone() {
                                ui.selectable_value(
                                    &mut self.source_language,
                                    *lang,
                                    format!("{} {}", lang.flag_emoji(), lang.name()),
                                );
                            }
                        });
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label("Preview Language:");
                    egui::ComboBox::from_id_salt("preview_lang")
                        .selected_text(format!(
                            "{} {}",
                            self.active_preview_language.flag_emoji(),
                            self.active_preview_language.name()
                        ))
                        .show_ui(ui, |ui| {
                            for lang in &self.enabled_languages.clone() {
                                ui.selectable_value(
                                    &mut self.active_preview_language,
                                    *lang,
                                    format!("{} {}", lang.flag_emoji(), lang.name()),
                                );
                            }
                        });
                });

                ui.add_space(20.0);

                // Available languages to add
                ui.label(RichText::new("Available Languages").strong());

                let available = [
                    Language::English,
                    Language::Spanish,
                    Language::French,
                    Language::German,
                    Language::Italian,
                    Language::Portuguese,
                    Language::Russian,
                    Language::Japanese,
                    Language::Korean,
                    Language::SimplifiedChinese,
                    Language::TraditionalChinese,
                    Language::Arabic,
                ];

                egui::ScrollArea::vertical()
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for lang in available {
                            if !self.enabled_languages.contains(&lang) {
                                ui.horizontal(|ui| {
                                    ui.label(format!("{} {}", lang.flag_emoji(), lang.name()));
                                    if ui.button("+ Add").clicked() {
                                        self.enabled_languages.push(lang);
                                    }
                                });
                            }
                        }
                    });
            });
        });
    }

    fn show_statistics_tab(&mut self, ui: &mut Ui) {
        ui.heading("Localization Statistics");
        ui.add_space(10.0);

        // Overall stats
        ui.group(|ui| {
            ui.label(RichText::new("📈 Overview").strong());

            egui::Grid::new("overview")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Total Strings:");
                    ui.label(RichText::new(format!("{}", self.strings.len())).strong());
                    ui.end_row();

                    ui.label("Languages:");
                    ui.label(RichText::new(format!("{}", self.enabled_languages.len())).strong());
                    ui.end_row();

                    ui.label("Missing Translations:");
                    let missing = self.count_missing_translations();
                    ui.label(RichText::new(format!("{}", missing)).strong().color(
                        if missing > 0 {
                            Color32::YELLOW
                        } else {
                            Color32::GREEN
                        },
                    ));
                    ui.end_row();

                    ui.label("Needs Review:");
                    let needs_review = self.strings.iter().filter(|s| s.needs_review).count();
                    ui.label(RichText::new(format!("{}", needs_review)).strong());
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Per-language completion
        ui.group(|ui| {
            ui.label(RichText::new("[Glb] Per-Language Completion").strong());

            for lang in &self.enabled_languages {
                let total = self.strings.len();
                let translated = self
                    .strings
                    .iter()
                    .filter(|s| s.translations.contains_key(lang))
                    .count();
                let percent = if total > 0 {
                    100.0 * translated as f32 / total as f32
                } else {
                    0.0
                };

                ui.horizontal(|ui| {
                    ui.label(format!("{} {}: ", lang.flag_emoji(), lang.name()));
                    ui.add(
                        egui::ProgressBar::new(percent / 100.0)
                            .text(format!("{}/{} ({:.0}%)", translated, total, percent)),
                    );
                });
            }
        });

        ui.add_space(10.0);

        // Per-category breakdown
        ui.group(|ui| {
            ui.label(RichText::new("Per-Category").strong());

            let categories = [
                StringCategory::Ui,
                StringCategory::Dialogue,
                StringCategory::Quest,
                StringCategory::Item,
                StringCategory::Achievement,
                StringCategory::Tutorial,
                StringCategory::System,
                StringCategory::Error,
            ];

            for cat in categories {
                let count = self.strings.iter().filter(|s| s.category == cat).count();
                if count > 0 {
                    ui.label(format!("{} {:?}: {} strings", cat.icon(), cat, count));
                }
            }
        });
    }

    fn show_import_export_tab(&mut self, ui: &mut Ui) {
        ui.heading("Import/Export");
        ui.add_space(10.0);

        // Export
        ui.group(|ui| {
            ui.label(RichText::new("📤 Export").strong());

            ui.horizontal(|ui| {
                ui.label("Format:");
                egui::ComboBox::from_id_salt("export_format")
                    .selected_text(format!("{:?}", self.export_format))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.export_format, ExportFormat::Csv, "CSV");
                        ui.selectable_value(&mut self.export_format, ExportFormat::Xliff, "XLIFF");
                        ui.selectable_value(
                            &mut self.export_format,
                            ExportFormat::Po,
                            "PO (gettext)",
                        );
                        ui.selectable_value(&mut self.export_format, ExportFormat::Json, "JSON");
                        ui.selectable_value(
                            &mut self.export_format,
                            ExportFormat::Resx,
                            "RESX (.NET)",
                        );
                    });
            });

            ui.add_space(5.0);

            ui.label("Languages to export:");
            for lang in &self.enabled_languages.clone() {
                let mut selected = self.export_languages.contains(lang);
                if ui
                    .checkbox(
                        &mut selected,
                        format!("{} {}", lang.flag_emoji(), lang.name()),
                    )
                    .clicked()
                {
                    if selected {
                        self.export_languages.push(*lang);
                    } else {
                        self.export_languages.retain(|l| l != lang);
                    }
                }
            }

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("📤 Export").clicked() {
                    info!(format = ?self.export_format, "Localization: exporting strings");
                    // Export strings
                }
                if ui.button("📤 Export Missing Only").clicked() {
                    info!(format = ?self.export_format, "Localization: exporting missing translations");
                    // Export only missing translations
                }
            });
        });

        ui.add_space(10.0);

        // Import
        ui.group(|ui| {
            ui.label(RichText::new("Import").strong());

            ui.horizontal(|ui| {
                ui.label("File:");
                ui.text_edit_singleline(&mut self.import_path);
                if ui.button("Browse").clicked() {
                    // Open file dialog
                }
            });

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                if ui.button("Import").clicked() {
                    info!(path = %self.import_path, "Localization: importing strings");
                    // Import strings
                }
                if ui.button("Merge").clicked() {
                    info!(path = %self.import_path, "Localization: merging strings");
                    // Merge with existing strings
                }
            });
        });
    }

    fn show_settings_tab(&mut self, ui: &mut Ui) {
        ui.heading("Localization Settings");
        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("General").strong());

            ui.checkbox(&mut false, "Auto-detect text overflow");
            ui.checkbox(&mut false, "Warn on missing translations");
            ui.checkbox(&mut false, "Fallback to source language");
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.label(RichText::new("[Sq] Text Validation").strong());

            ui.checkbox(&mut false, "Check max length");
            ui.checkbox(&mut false, "Validate placeholders (e.g., {0}, {name})");
            ui.checkbox(&mut false, "Check for HTML/BBCode tags");
        });
    }

    // Getters for testing
    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    pub fn language_count(&self) -> usize {
        self.enabled_languages.len()
    }

    pub fn add_string(&mut self, key: &str, category: StringCategory) {
        self.strings.push(LocalizedString {
            key: key.to_string(),
            category,
            ..Default::default()
        });
    }

    pub fn add_language(&mut self, lang: Language) {
        if !self.enabled_languages.contains(&lang) {
            info!(language = %lang.name(), "Localization: added language");
            self.enabled_languages.push(lang);
        }
    }
}

impl Panel for LocalizationPanel {
    fn name(&self) -> &'static str {
        "Localization"
    }

    fn show(&mut self, ui: &mut Ui) {
        self.show_tab_bar(ui);

        match self.active_tab {
            LocalizationTab::Strings => self.show_strings_tab(ui),
            LocalizationTab::Languages => self.show_languages_tab(ui),
            LocalizationTab::Statistics => self.show_statistics_tab(ui),
            LocalizationTab::ImportExport => self.show_import_export_tab(ui),
            LocalizationTab::Settings => self.show_settings_tab(ui),
        }
    }

    fn update(&mut self) {
        // Sync current string back to list
        if let Some(idx) = self.selected_string {
            if idx < self.strings.len() {
                self.strings[idx] = self.current_string.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // LANGUAGE TESTS - DEFAULT AND VARIANTS
    // ============================================================

    #[test]
    fn test_language_default() {
        let lang = Language::default();
        assert_eq!(lang, Language::English);
    }

    #[test]
    fn test_language_all_variants() {
        let variants = [
            Language::English,
            Language::Spanish,
            Language::French,
            Language::German,
            Language::Italian,
            Language::Portuguese,
            Language::Russian,
            Language::Japanese,
            Language::Korean,
            Language::SimplifiedChinese,
            Language::TraditionalChinese,
            Language::Arabic,
            Language::Custom(0),
        ];
        assert_eq!(variants.len(), 13);
    }

    #[test]
    fn test_language_custom_ids() {
        let custom1 = Language::Custom(0);
        let custom2 = Language::Custom(100);
        let custom3 = Language::Custom(999);
        assert_ne!(custom1, custom2);
        assert_ne!(custom2, custom3);
    }

    // ============================================================
    // LANGUAGE CODE TESTS
    // ============================================================

    #[test]
    fn test_language_codes() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::SimplifiedChinese.code(), "zh-CN");
    }

    #[test]
    fn test_language_code_all() {
        assert_eq!(Language::English.code(), "en");
        assert_eq!(Language::Spanish.code(), "es");
        assert_eq!(Language::French.code(), "fr");
        assert_eq!(Language::German.code(), "de");
        assert_eq!(Language::Italian.code(), "it");
        assert_eq!(Language::Portuguese.code(), "pt");
        assert_eq!(Language::Russian.code(), "ru");
        assert_eq!(Language::Japanese.code(), "ja");
        assert_eq!(Language::Korean.code(), "ko");
        assert_eq!(Language::SimplifiedChinese.code(), "zh-CN");
        assert_eq!(Language::TraditionalChinese.code(), "zh-TW");
        assert_eq!(Language::Arabic.code(), "ar");
        assert_eq!(Language::Custom(0).code(), "custom");
        assert_eq!(Language::Custom(999).code(), "custom");
    }

    // ============================================================
    // LANGUAGE NAME TESTS
    // ============================================================

    #[test]
    fn test_language_name_all() {
        assert_eq!(Language::English.name(), "English");
        assert_eq!(Language::Spanish.name(), "Español");
        assert_eq!(Language::French.name(), "Français");
        assert_eq!(Language::German.name(), "Deutsch");
        assert_eq!(Language::Italian.name(), "Italiano");
        assert_eq!(Language::Portuguese.name(), "Português");
        assert_eq!(Language::Russian.name(), "Русский");
        assert_eq!(Language::Japanese.name(), "日本語");
        assert_eq!(Language::Korean.name(), "한국어");
        assert_eq!(Language::SimplifiedChinese.name(), "简体中文");
        assert_eq!(Language::TraditionalChinese.name(), "繁體中文");
        assert_eq!(Language::Arabic.name(), "العربية");
        assert_eq!(Language::Custom(0).name(), "Custom");
    }

    // ============================================================
    // LANGUAGE FLAG TESTS
    // ============================================================

    #[test]
    fn test_language_flags() {
        assert_eq!(Language::English.flag_emoji(), "🇬🇧");
        assert_eq!(Language::Japanese.flag_emoji(), "🇯🇵");
    }

    #[test]
    fn test_language_flag_all() {
        assert_eq!(Language::English.flag_emoji(), "🇬🇧");
        assert_eq!(Language::Spanish.flag_emoji(), "🇪🇸");
        assert_eq!(Language::French.flag_emoji(), "🇫🇷");
        assert_eq!(Language::German.flag_emoji(), "🇩🇪");
        assert_eq!(Language::Italian.flag_emoji(), "🇮🇹");
        assert_eq!(Language::Portuguese.flag_emoji(), "🇧🇷");
        assert_eq!(Language::Russian.flag_emoji(), "🇷🇺");
        assert_eq!(Language::Japanese.flag_emoji(), "🇯🇵");
        assert_eq!(Language::Korean.flag_emoji(), "🇰🇷");
        assert_eq!(Language::SimplifiedChinese.flag_emoji(), "🇨🇳");
        assert_eq!(Language::TraditionalChinese.flag_emoji(), "🇹🇼");
        assert_eq!(Language::Arabic.flag_emoji(), "🇸🇦");
        assert_eq!(Language::Custom(0).flag_emoji(), "🏳️");
    }

    // ============================================================
    // LANGUAGE CLONE AND COPY TESTS
    // ============================================================

    #[test]
    fn test_language_clone() {
        let lang = Language::Japanese;
        let cloned = lang;
        assert_eq!(cloned, Language::Japanese);
    }

    #[test]
    fn test_language_hash() {
        let mut map = HashMap::new();
        map.insert(Language::English, "Hello");
        map.insert(Language::Japanese, "こんにちは");
        assert_eq!(map.get(&Language::English), Some(&"Hello"));
        assert_eq!(map.get(&Language::Japanese), Some(&"こんにちは"));
    }

    // ============================================================
    // STRING CATEGORY TESTS
    // ============================================================

    #[test]
    fn test_string_category_default() {
        let cat = StringCategory::default();
        assert_eq!(cat, StringCategory::Ui);
    }

    #[test]
    fn test_string_category_all_variants() {
        let variants = [
            StringCategory::Ui,
            StringCategory::Dialogue,
            StringCategory::Quest,
            StringCategory::Item,
            StringCategory::Achievement,
            StringCategory::Tutorial,
            StringCategory::System,
            StringCategory::Error,
        ];
        assert_eq!(variants.len(), 8);
    }

    #[test]
    fn test_category_icons() {
        assert_eq!(StringCategory::Dialogue.icon(), "[Chat]");
        assert_eq!(StringCategory::Quest.icon(), "📜");
    }

    #[test]
    fn test_category_icon_all() {
        assert_eq!(StringCategory::Ui.icon(), "[Mon]");
        assert_eq!(StringCategory::Dialogue.icon(), "[Chat]");
        assert_eq!(StringCategory::Quest.icon(), "📜");
        assert_eq!(StringCategory::Item.icon(), "🎒");
        assert_eq!(StringCategory::Achievement.icon(), "🏆");
        assert_eq!(StringCategory::Tutorial.icon(), "📖");
        assert_eq!(StringCategory::System.icon(), "⚙️");
        assert_eq!(StringCategory::Error.icon(), "[x]");
    }

    #[test]
    fn test_string_category_clone() {
        let cat = StringCategory::Quest;
        let cloned = cat;
        assert_eq!(cloned, StringCategory::Quest);
    }

    // ============================================================
    // LOCALIZED STRING TESTS
    // ============================================================

    #[test]
    fn test_localized_string_default() {
        let s = LocalizedString::default();
        assert!(s.key.is_empty());
        assert_eq!(s.category, StringCategory::Ui);
        assert!(s.context.is_empty());
        assert!(s.translations.is_empty());
        assert!(!s.needs_review);
        assert!(s.max_length.is_none());
        assert!(s.plural_forms.is_none());
    }

    #[test]
    fn test_localized_string_custom() {
        let mut translations = HashMap::new();
        translations.insert(Language::English, "Hello".to_string());
        translations.insert(Language::Spanish, "Hola".to_string());

        let s = LocalizedString {
            key: "greeting.hello".to_string(),
            category: StringCategory::Dialogue,
            context: "NPC greeting".to_string(),
            translations,
            needs_review: true,
            max_length: Some(50),
            plural_forms: None,
        };
        assert_eq!(s.key, "greeting.hello");
        assert_eq!(s.category, StringCategory::Dialogue);
        assert_eq!(s.context, "NPC greeting");
        assert_eq!(s.translations.len(), 2);
        assert!(s.needs_review);
        assert_eq!(s.max_length, Some(50));
    }

    #[test]
    fn test_localized_string_clone() {
        let mut translations = HashMap::new();
        translations.insert(Language::English, "Test".to_string());

        let s = LocalizedString {
            key: "test.key".to_string(),
            translations,
            ..Default::default()
        };
        let cloned = s.clone();
        assert_eq!(cloned.key, "test.key");
        assert_eq!(cloned.translations.len(), 1);
    }

    // ============================================================
    // PLURAL FORMS TESTS
    // ============================================================

    #[test]
    fn test_plural_forms_default() {
        let p = PluralForms::default();
        assert!(p.zero.is_empty());
        assert!(p.one.is_empty());
        assert!(p.two.is_empty());
        assert!(p.few.is_empty());
        assert!(p.many.is_empty());
        assert!(p.other.is_empty());
    }

    #[test]
    fn test_plural_forms_custom() {
        let p = PluralForms {
            zero: "no items".to_string(),
            one: "1 item".to_string(),
            two: "2 items".to_string(),
            few: "a few items".to_string(),
            many: "many items".to_string(),
            other: "{n} items".to_string(),
        };
        assert_eq!(p.zero, "no items");
        assert_eq!(p.one, "1 item");
        assert_eq!(p.other, "{n} items");
    }

    #[test]
    fn test_plural_forms_clone() {
        let p = PluralForms {
            one: "1 coin".to_string(),
            other: "{n} coins".to_string(),
            ..Default::default()
        };
        let cloned = p.clone();
        assert_eq!(cloned.one, "1 coin");
        assert_eq!(cloned.other, "{n} coins");
    }

    // ============================================================
    // EXPORT FORMAT TESTS
    // ============================================================

    #[test]
    fn test_export_format_default() {
        let fmt = ExportFormat::default();
        assert_eq!(fmt, ExportFormat::Csv);
    }

    #[test]
    fn test_export_format_all_variants() {
        let variants = [
            ExportFormat::Csv,
            ExportFormat::Xliff,
            ExportFormat::Po,
            ExportFormat::Json,
            ExportFormat::Resx,
        ];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn test_export_format_clone() {
        let fmt = ExportFormat::Xliff;
        let cloned = fmt;
        assert_eq!(cloned, ExportFormat::Xliff);
    }

    // ============================================================
    // LOCALIZATION TAB TESTS
    // ============================================================

    #[test]
    fn test_localization_tab_default() {
        let tab = LocalizationTab::default();
        assert_eq!(tab, LocalizationTab::Strings);
    }

    #[test]
    fn test_localization_tab_all_variants() {
        let variants = [
            LocalizationTab::Strings,
            LocalizationTab::Languages,
            LocalizationTab::Statistics,
            LocalizationTab::ImportExport,
            LocalizationTab::Settings,
        ];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn test_localization_tab_clone() {
        let tab = LocalizationTab::Statistics;
        let cloned = tab;
        assert_eq!(cloned, LocalizationTab::Statistics);
    }

    // ============================================================
    // LOCALIZATION PANEL CREATION TESTS
    // ============================================================

    #[test]
    fn test_localization_panel_creation() {
        let mut panel = LocalizationPanel::new();
        panel.create_sample_data();
        assert!(panel.string_count() >= 5);
    }

    #[test]
    fn test_localization_panel_default() {
        let mut panel = LocalizationPanel::default();
        panel.create_sample_data();
        assert!(panel.string_count() >= 5);
        assert!(panel.language_count() >= 5);
    }

    #[test]
    fn test_panel_trait() {
        let panel = LocalizationPanel::new();
        assert_eq!(panel.name(), "Localization");
    }

    #[test]
    fn test_default_languages() {
        let panel = LocalizationPanel::new();
        assert!(panel.language_count() >= 5);
    }

    // ============================================================
    // STRING MANAGEMENT TESTS
    // ============================================================

    #[test]
    fn test_add_string() {
        let mut panel = LocalizationPanel::new();
        let initial = panel.string_count();
        panel.add_string("test.key", StringCategory::Ui);
        assert_eq!(panel.string_count(), initial + 1);
    }

    #[test]
    fn test_add_multiple_strings() {
        let mut panel = LocalizationPanel::new();
        let initial = panel.string_count();
        panel.add_string("test.key1", StringCategory::Ui);
        panel.add_string("test.key2", StringCategory::Dialogue);
        panel.add_string("test.key3", StringCategory::Quest);
        assert_eq!(panel.string_count(), initial + 3);
    }

    #[test]
    fn test_add_string_different_categories() {
        let mut panel = LocalizationPanel::new();
        let initial = panel.string_count();
        panel.add_string("ui.button", StringCategory::Ui);
        panel.add_string("npc.greeting", StringCategory::Dialogue);
        panel.add_string("quest.title", StringCategory::Quest);
        panel.add_string("item.name", StringCategory::Item);
        panel.add_string("achievement.name", StringCategory::Achievement);
        panel.add_string("tutorial.step", StringCategory::Tutorial);
        panel.add_string("system.error", StringCategory::System);
        panel.add_string("error.msg", StringCategory::Error);
        assert_eq!(panel.string_count(), initial + 8);
    }

    // ============================================================
    // LANGUAGE MANAGEMENT TESTS
    // ============================================================

    #[test]
    fn test_add_language() {
        let mut panel = LocalizationPanel::new();
        let initial = panel.language_count();
        panel.add_language(Language::Italian);
        assert_eq!(panel.language_count(), initial + 1);
    }

    #[test]
    fn test_add_duplicate_language() {
        let mut panel = LocalizationPanel::new();
        panel.add_language(Language::Arabic);
        let count_after_first = panel.language_count();
        panel.add_language(Language::Arabic);
        assert_eq!(panel.language_count(), count_after_first); // No duplicate
    }

    #[test]
    fn test_add_multiple_languages() {
        let mut panel = LocalizationPanel::new();
        let initial = panel.language_count();
        panel.add_language(Language::Arabic);
        panel.add_language(Language::Korean);
        panel.add_language(Language::Italian);
        assert_eq!(panel.language_count(), initial + 3);
    }

    #[test]
    fn test_add_custom_language() {
        let mut panel = LocalizationPanel::new();
        let initial = panel.language_count();
        panel.add_language(Language::Custom(1));
        panel.add_language(Language::Custom(2));
        assert_eq!(panel.language_count(), initial + 2);
    }

    // ============================================================
    // SAMPLE DATA TESTS
    // ============================================================

    #[test]
    fn test_sample_data_exists() {
        let mut panel = LocalizationPanel::new();
        panel.create_sample_data();
        assert!(panel.string_count() >= 5); // Sample data creates at least 5 strings
    }

    #[test]
    fn test_default_enabled_languages() {
        let panel = LocalizationPanel::new();
        // Default includes English, Spanish, French, German, Japanese
        assert!(panel.language_count() >= 5);
    }

    // ============================================================
    // EDGE CASE TESTS
    // ============================================================

    #[test]
    fn test_empty_key() {
        let mut panel = LocalizationPanel::new();
        let initial = panel.string_count();
        panel.add_string("", StringCategory::Ui);
        assert_eq!(panel.string_count(), initial + 1);
    }

    #[test]
    fn test_long_key() {
        let mut panel = LocalizationPanel::new();
        let initial = panel.string_count();
        let long_key = "very.long.key.path.that.goes.on.and.on.for.testing";
        panel.add_string(long_key, StringCategory::Ui);
        assert_eq!(panel.string_count(), initial + 1);
    }

    #[test]
    fn test_unicode_in_translations() {
        let mut translations = HashMap::new();
        translations.insert(Language::English, "Hello".to_string());
        translations.insert(Language::Japanese, "こんにちは".to_string());
        translations.insert(Language::Arabic, "مرحبا".to_string());
        translations.insert(Language::Russian, "Привет".to_string());

        let s = LocalizedString {
            key: "greeting".to_string(),
            translations,
            ..Default::default()
        };
        assert_eq!(s.translations.len(), 4);
        assert_eq!(
            s.translations.get(&Language::Japanese),
            Some(&"こんにちは".to_string())
        );
    }

    // ===== Language Enum Tests =====

    #[test]
    fn test_language_display_format() {
        for lang in Language::all() {
            let display = format!("{}", lang);
            assert!(!display.is_empty(), "Language display should not be empty");
        }
    }

    #[test]
    fn test_language_all_contains_expected() {
        let variants = Language::all();
        assert_eq!(variants.len(), 12, "Expected 12 standard language variants");
        assert!(variants.contains(&Language::English));
        assert!(variants.contains(&Language::Spanish));
        assert!(variants.contains(&Language::French));
        assert!(variants.contains(&Language::German));
        assert!(variants.contains(&Language::Japanese));
        assert!(variants.contains(&Language::SimplifiedChinese));
    }

    #[test]
    fn test_language_hash_in_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for lang in Language::all() {
            set.insert(*lang);
        }
        assert_eq!(set.len(), Language::all().len());
    }

    // ===== StringCategory Enum Tests =====

    #[test]
    fn test_string_category_display_format() {
        for cat in StringCategory::all() {
            let display = format!("{}", cat);
            assert!(display.contains(cat.name()), "Display should contain name");
        }
    }

    #[test]
    fn test_string_category_all_contains_expected() {
        let variants = StringCategory::all();
        assert_eq!(variants.len(), 8, "Expected 8 string category variants");
        assert!(variants.contains(&StringCategory::Ui));
        assert!(variants.contains(&StringCategory::Dialogue));
        assert!(variants.contains(&StringCategory::Quest));
    }

    #[test]
    fn test_string_category_hash_in_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for cat in StringCategory::all() {
            set.insert(*cat);
        }
        assert_eq!(set.len(), StringCategory::all().len());
    }

    #[test]
    fn test_string_category_name_method() {
        assert_eq!(StringCategory::Ui.name(), "UI");
        assert_eq!(StringCategory::Dialogue.name(), "Dialogue");
        assert_eq!(StringCategory::Quest.name(), "Quest");
    }

    // ===== ExportFormat Enum Tests =====

    #[test]
    fn test_export_format_display_format() {
        for format in ExportFormat::all() {
            let display = format!("{}", format);
            assert!(
                display.contains(format.name()),
                "Display should contain name"
            );
        }
    }

    #[test]
    fn test_export_format_hash_in_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for format in ExportFormat::all() {
            set.insert(*format);
        }
        assert_eq!(set.len(), ExportFormat::all().len());
    }

    #[test]
    fn test_export_format_extension_method() {
        assert_eq!(ExportFormat::Json.extension(), ".json");
        assert_eq!(ExportFormat::Csv.extension(), ".csv");
        assert_eq!(ExportFormat::Xliff.extension(), ".xlf");
        assert_eq!(ExportFormat::Po.extension(), ".po");
        assert_eq!(ExportFormat::Resx.extension(), ".resx");
    }

    // ===== LocalizationTab Enum Tests =====

    #[test]
    fn test_localization_tab_display_format() {
        for tab in LocalizationTab::all() {
            let display = format!("{}", tab);
            assert!(display.contains(tab.name()), "Display should contain name");
        }
    }

    #[test]
    fn test_localization_tab_hash_in_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for tab in LocalizationTab::all() {
            set.insert(*tab);
        }
        assert_eq!(set.len(), LocalizationTab::all().len());
    }

    #[test]
    fn test_localization_tab_icon_method() {
        for tab in LocalizationTab::all() {
            let icon = tab.icon();
            assert!(!icon.is_empty(), "Tab icon should not be empty");
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // LocalizationAction Tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_localization_action_display() {
        let action = LocalizationAction::SetActiveTab(LocalizationTab::Strings);
        let display = format!("{}", action);
        assert!(display.contains("tab"));
    }

    #[test]
    fn test_localization_action_display_all_variants() {
        let actions = vec![
            LocalizationAction::SetActiveTab(LocalizationTab::Languages),
            LocalizationAction::AddString,
            LocalizationAction::RemoveString(0),
            LocalizationAction::SelectString(1),
            LocalizationAction::AddLanguage(Language::French),
            LocalizationAction::SetSourceLanguage(Language::English),
            LocalizationAction::Export,
            LocalizationAction::ValidateStrings,
        ];

        for action in actions {
            let display = format!("{}", action);
            assert!(
                !display.is_empty(),
                "Display should not be empty for {:?}",
                action
            );
        }
    }

    #[test]
    fn test_localization_action_equality() {
        let action1 = LocalizationAction::SelectString(5);
        let action2 = LocalizationAction::SelectString(5);
        let action3 = LocalizationAction::SelectString(10);

        assert_eq!(action1, action2);
        assert_ne!(action1, action3);
    }

    #[test]
    fn test_localization_action_clone() {
        let action = LocalizationAction::SetFilterText("test".to_string());
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_localization_panel_pending_actions_empty_by_default() {
        let panel = LocalizationPanel::new();
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_localization_panel_queue_action() {
        let mut panel = LocalizationPanel::new();
        panel.queue_action(LocalizationAction::AddString);
        assert!(panel.has_pending_actions());
        assert_eq!(panel.pending_actions().len(), 1);
    }

    #[test]
    fn test_localization_panel_take_actions() {
        let mut panel = LocalizationPanel::new();
        panel.queue_action(LocalizationAction::AddString);
        panel.queue_action(LocalizationAction::SetActiveTab(LocalizationTab::Languages));

        let actions = panel.take_actions();
        assert_eq!(actions.len(), 2);
        assert!(!panel.has_pending_actions());
        assert!(panel.pending_actions().is_empty());
    }

    #[test]
    fn test_localization_panel_action_order_preserved() {
        let mut panel = LocalizationPanel::new();
        panel.queue_action(LocalizationAction::AddString);
        panel.queue_action(LocalizationAction::SelectString(0));
        panel.queue_action(LocalizationAction::RemoveString(0));

        let actions = panel.take_actions();
        assert!(matches!(actions[0], LocalizationAction::AddString));
        assert!(matches!(actions[1], LocalizationAction::SelectString(_)));
        assert!(matches!(actions[2], LocalizationAction::RemoveString(_)));
    }

    #[test]
    fn test_localization_action_string_operations() {
        let actions = vec![
            LocalizationAction::AddString,
            LocalizationAction::RemoveString(0),
            LocalizationAction::DuplicateString(1),
            LocalizationAction::SelectString(2),
            LocalizationAction::SetStringKey(0, "key.test".to_string()),
            LocalizationAction::SetStringCategory(0, StringCategory::Ui),
            LocalizationAction::SetStringContext(0, "Test context".to_string()),
            LocalizationAction::MarkNeedsReview(0, true),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_localization_action_language_operations() {
        let actions = vec![
            LocalizationAction::AddLanguage(Language::German),
            LocalizationAction::RemoveLanguage(Language::French),
            LocalizationAction::SetSourceLanguage(Language::English),
            LocalizationAction::SetPreviewLanguage(Language::Spanish),
            LocalizationAction::ToggleLanguageEnabled(Language::Japanese, true),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_localization_action_filter_operations() {
        let actions = vec![
            LocalizationAction::SetFilterText("search".to_string()),
            LocalizationAction::SetFilterCategory(Some(StringCategory::Dialogue)),
            LocalizationAction::ToggleMissingOnly(true),
            LocalizationAction::ToggleNeedsReviewOnly(false),
            LocalizationAction::ClearFilters,
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("search"));
        assert!(displays[4].contains("Clear"));
    }

    #[test]
    fn test_localization_action_import_export() {
        let actions = vec![
            LocalizationAction::SetExportFormat(ExportFormat::Json),
            LocalizationAction::AddExportLanguage(Language::French),
            LocalizationAction::RemoveExportLanguage(Language::German),
            LocalizationAction::Export,
            LocalizationAction::Import("path/to/file.json".to_string()),
            LocalizationAction::SetImportPath("path/to/folder".to_string()),
        ];

        for action in &actions {
            let display = format!("{}", action);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_localization_action_batch_operations() {
        let actions = vec![
            LocalizationAction::AutoTranslate(Language::French),
            LocalizationAction::CopyFromLanguage(Language::English, Language::Spanish),
            LocalizationAction::ClearLanguage(Language::German),
            LocalizationAction::ValidateStrings,
            LocalizationAction::FindDuplicates,
            LocalizationAction::RefreshStatistics,
        ];

        let displays: Vec<_> = actions.iter().map(|a| format!("{}", a)).collect();
        assert!(displays[0].contains("Auto-translate"));
        assert!(displays[3].contains("Validate"));
    }
}
