//! Bridge between the `tracing` ecosystem and the in-editor [`ConsolePanel`].
//!
//! [`EditorConsoleLayer`] is a `tracing_subscriber::Layer` that captures every
//! tracing event, converts it to a [`LogEntry`], and pushes it into a shared
//! ring-buffer.  The editor drains this buffer each frame and forwards the
//! entries to `ConsolePanel::push_entry`.

use crate::panels::console_panel::{LogEntry, LogLevel};
use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Maximum entries stored in the shared sink before old entries are dropped.
const MAX_SINK_ENTRIES: usize = 2000;

/// Shared log sink: the bridge writes, the editor drains each frame.
pub type LogSink = Arc<Mutex<VecDeque<LogEntry>>>;

/// Create a new shared log sink.
pub fn new_log_sink() -> LogSink {
    Arc::new(Mutex::new(VecDeque::with_capacity(MAX_SINK_ENTRIES)))
}

/// Global log sink, shared between the tracing layer (set up in `main()`)
/// and `EditorApp` (which drains it each frame).
static GLOBAL_LOG_SINK: std::sync::OnceLock<LogSink> = std::sync::OnceLock::new();

/// Initialize the global log sink and tracing subscriber with the editor
/// console layer. Call once in `main()`, before the eframe app starts.
///
/// Returns `Ok(())` on success or if already initialized.
pub fn init_editor_tracing() -> anyhow::Result<()> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    let sink = new_log_sink();
    // Store in the global — ignore error if already set (idempotent).
    let _ = GLOBAL_LOG_SINK.set(Arc::clone(&sink));

    let console_layer = EditorConsoleLayer::new(sink);

    let subscriber = tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,aw_editor=debug")),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(true),
        )
        .with(console_layer);

    subscriber.try_init().ok();
    Ok(())
}

/// Drain all pending log entries from the global sink.
/// Call this once per frame from `EditorApp::update()`.
pub fn drain_log_sink(console: &mut super::panels::console_panel::ConsolePanel) {
    if let Some(sink) = GLOBAL_LOG_SINK.get() {
        let mut guard = match sink.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                // Recover from poisoned lock — logs are too important to lose
                eprintln!("WARN: Log sink mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
        for entry in guard.drain(..) {
            console.push_entry(entry);
        }
    }
}

/// A `tracing_subscriber::Layer` that captures events and pushes them into a
/// shared `VecDeque<LogEntry>` for the editor console to drain.
pub struct EditorConsoleLayer {
    sink: LogSink,
}

impl EditorConsoleLayer {
    pub fn new(sink: LogSink) -> Self {
        Self { sink }
    }
}

impl<S: Subscriber> Layer<S> for EditorConsoleLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        // Map tracing level to LogLevel
        let level = match *metadata.level() {
            Level::ERROR => LogLevel::Error,
            Level::WARN => LogLevel::Warning,
            Level::INFO => LogLevel::Info,
            Level::DEBUG | Level::TRACE => LogLevel::Debug,
        };

        // Extract message via visitor
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.finish();

        // Build the LogEntry
        let mut entry = LogEntry::new(message, level);

        // Derive category from tracing target
        let target = metadata.target();
        entry = entry.with_category(category_from_target(target));

        // Attach source location if available
        if let (Some(file), Some(line)) = (metadata.file(), metadata.line()) {
            entry = entry.with_source(file, line);
        }

        // Push into the shared sink
        if let Ok(mut sink) = self.sink.lock() {
            if sink.len() >= MAX_SINK_ENTRIES {
                sink.pop_front();
            }
            sink.push_back(entry);
        }
    }
}

// ---------------------------------------------------------------------------
// Category mapping
// ---------------------------------------------------------------------------

/// Maps a tracing target string to a human-readable console category.
///
/// Tracing targets follow Rust module paths, e.g.
/// `aw_editor::panels::lighting_panel`.  We match prefixes to produce
/// short display categories like `"Lighting"`.
fn category_from_target(target: &str) -> &'static str {
    // Panel-specific categories (longest prefix first for correct matching)
    if target.contains("animation_panel") || target.contains("animation_bridge") {
        return "Animation";
    }
    if target.contains("material_editor") || target.contains("material_inspector") {
        return "Material";
    }
    if target.contains("physics_panel") {
        return "Physics";
    }
    if target.contains("lighting_panel") {
        return "Lighting";
    }
    if target.contains("audio_panel") || target.contains("audio_bridge") {
        return "Audio";
    }
    if target.contains("terrain_panel") || target.contains("terrain_integration") {
        return "Terrain";
    }
    if target.contains("navigation_panel") {
        return "Navigation";
    }
    if target.contains("particle_system") {
        return "Particles";
    }
    if target.contains("cinematics") {
        return "Cinematics";
    }
    if target.contains("dialogue_editor") {
        return "Dialogue";
    }
    if target.contains("foliage_panel") {
        return "Foliage";
    }
    if target.contains("pcg_panel") || target.contains("procedural_filler") {
        return "PCG";
    }
    if target.contains("post_process") {
        return "PostProcess";
    }
    if target.contains("networking") {
        return "Network";
    }
    if target.contains("build_manager") {
        return "Build";
    }
    if target.contains("ui_editor") {
        return "UI";
    }
    if target.contains("asset_browser") || target.contains("blend_import") {
        return "Assets";
    }
    if target.contains("project_settings") {
        return "Settings";
    }
    if target.contains("console_panel") {
        return "Console";
    }
    if target.contains("entity_panel") || target.contains("entity_catalog") {
        return "Entity";
    }
    if target.contains("hierarchy_panel") {
        return "Hierarchy";
    }
    if target.contains("transform_panel") {
        return "Transform";
    }
    if target.contains("world_panel") || target.contains("world_wizard") {
        return "World";
    }
    if target.contains("theme_manager") {
        return "Theme";
    }
    if target.contains("lod_config") {
        return "LOD";
    }
    if target.contains("localization") {
        return "Localization";
    }
    if target.contains("input_bindings") {
        return "Input";
    }
    if target.contains("distribution") {
        return "Distribution";
    }
    if target.contains("import_doctor") {
        return "ImportDoctor";
    }
    if target.contains("blueprint_panel") {
        return "Blueprint";
    }
    if target.contains("spline_editor") {
        return "Spline";
    }
    if target.contains("graph_panel") {
        return "Graph";
    }
    if target.contains("environment_preset") {
        return "Environment";
    }
    if target.contains("gameplay_presets") {
        return "Gameplay";
    }
    if target.contains("ready_asset_store") {
        return "AssetStore";
    }
    if target.contains("charts_panel") || target.contains("advanced_widgets") {
        return "Widgets";
    }
    if target.contains("scene_stats") {
        return "SceneStats";
    }
    if target.contains("profiler_panel") {
        return "Profiler";
    }
    if target.contains("performance_panel") {
        return "Performance";
    }
    if target.contains("frame_debugger") {
        return "FrameDebugger";
    }

    // Subsystem categories
    if target.contains("viewport") {
        return "Viewport";
    }
    if target.contains("command") {
        return "Command";
    }
    if target.contains("scene_state") || target.contains("scene_serialization") {
        return "Scene";
    }
    if target.contains("telemetry") {
        return "Telemetry";
    }
    if target.contains("interaction") {
        return "Interaction";
    }
    if target.contains("gizmo") {
        return "Gizmo";
    }
    if target.contains("file_watcher") {
        return "FileWatcher";
    }
    if target.contains("plugin") {
        return "Plugin";
    }

    // Fallback for anything from the editor crate
    if target.contains("aw_editor") {
        return "Editor";
    }

    // External crates
    "External"
}

// ---------------------------------------------------------------------------
// Visitor: extracts `message` (and other fields) from a tracing event.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl MessageVisitor {
    /// Produce the final log message string.
    fn finish(self) -> String {
        if let Some(msg) = self.message {
            if self.fields.is_empty() {
                msg
            } else {
                let extras: Vec<String> = self
                    .fields
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect();
                format!("{msg} {{ {} }}", extras.join(", "))
            }
        } else if !self.fields.is_empty() {
            let parts: Vec<String> = self
                .fields
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            parts.join(", ")
        } else {
            "(empty event)".to_string()
        }
    }
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let val = format!("{:?}", value);
        if field.name() == "message" {
            self.message = Some(val);
        } else {
            self.fields.push((field.name().to_string(), val));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .push((field.name().to_string(), format!("{value:.4}")));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .push((field.name().to_string(), value.to_string()));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    #[test]
    fn test_category_from_target() {
        assert_eq!(
            category_from_target("aw_editor::panels::lighting_panel"),
            "Lighting"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::terrain_panel"),
            "Terrain"
        );
        assert_eq!(
            category_from_target("aw_editor::viewport::renderer"),
            "Viewport"
        );
        assert_eq!(category_from_target("aw_editor::command"), "Command");
        assert_eq!(category_from_target("aw_editor::scene_state"), "Scene");
        assert_eq!(
            category_from_target("aw_editor::scene_serialization"),
            "Scene"
        );
        assert_eq!(
            category_from_target("aw_editor::terrain_integration"),
            "Terrain"
        );
        assert_eq!(category_from_target("aw_editor::telemetry"), "Telemetry");
        assert_eq!(
            category_from_target("aw_editor::panels::physics_panel"),
            "Physics"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::audio_panel"),
            "Audio"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::animation_panel"),
            "Animation"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::material_editor_panel"),
            "Material"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::particle_system_panel"),
            "Particles"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::navigation_panel"),
            "Navigation"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::cinematics_panel"),
            "Cinematics"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::dialogue_editor_panel"),
            "Dialogue"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::foliage_panel"),
            "Foliage"
        );
        assert_eq!(category_from_target("aw_editor::panels::pcg_panel"), "PCG");
        assert_eq!(
            category_from_target("aw_editor::panels::post_process_panel"),
            "PostProcess"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::networking_panel"),
            "Network"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::build_manager"),
            "Build"
        );
        assert_eq!(
            category_from_target("aw_editor::panels::asset_browser"),
            "Assets"
        );
        assert_eq!(category_from_target("aw_editor"), "Editor");
        assert_eq!(category_from_target("some_external_crate"), "External");
    }

    #[test]
    fn test_level_mapping() {
        // Verify level mappings are correct by constructing LogEntry directly
        let debug_entry = LogEntry::new("test", LogLevel::Debug);
        assert_eq!(debug_entry.level, LogLevel::Debug);

        let info_entry = LogEntry::new("test", LogLevel::Info);
        assert_eq!(info_entry.level, LogLevel::Info);

        let warn_entry = LogEntry::new("test", LogLevel::Warning);
        assert_eq!(warn_entry.level, LogLevel::Warning);

        let error_entry = LogEntry::new("test", LogLevel::Error);
        assert_eq!(error_entry.level, LogLevel::Error);
    }

    #[test]
    fn test_message_visitor_simple_message() {
        let mut visitor = MessageVisitor::default();
        // Simulate recording a message
        visitor.message = Some("Hello world".to_string());
        assert_eq!(visitor.finish(), "Hello world");
    }

    #[test]
    fn test_message_visitor_with_fields() {
        let mut visitor = MessageVisitor::default();
        visitor.message = Some("Operation complete".to_string());
        visitor.fields.push(("count".to_string(), "42".to_string()));
        visitor
            .fields
            .push(("status".to_string(), "ok".to_string()));
        let result = visitor.finish();
        assert!(result.starts_with("Operation complete"));
        assert!(result.contains("count=42"));
        assert!(result.contains("status=ok"));
    }

    #[test]
    fn test_message_visitor_no_message() {
        let mut visitor = MessageVisitor::default();
        visitor.fields.push(("key".to_string(), "val".to_string()));
        assert_eq!(visitor.finish(), "key=val");
    }

    #[test]
    fn test_message_visitor_empty() {
        let visitor = MessageVisitor::default();
        assert_eq!(visitor.finish(), "(empty event)");
    }

    #[test]
    fn test_log_sink_capacity() {
        let sink = new_log_sink();
        {
            let mut guard = sink.lock().unwrap();
            for i in 0..MAX_SINK_ENTRIES + 100 {
                if guard.len() >= MAX_SINK_ENTRIES {
                    guard.pop_front();
                }
                guard.push_back(LogEntry::new(format!("msg {i}"), LogLevel::Info));
            }
            assert_eq!(guard.len(), MAX_SINK_ENTRIES);
            // The oldest should have been dropped
            assert!(guard.front().unwrap().message.contains("100"));
        }
    }

    #[test]
    fn test_roundtrip_tracing_to_log_entry() {
        let sink = new_log_sink();
        let layer = EditorConsoleLayer::new(Arc::clone(&sink));

        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "aw_editor::panels::lighting_panel", "Light created");
            tracing::warn!(target: "aw_editor::command", "Stack near capacity");
            tracing::error!(target: "aw_editor::viewport::renderer", "Render failed");
            tracing::debug!(target: "aw_editor::scene_state", "Cache synced");
        });

        let guard = sink.lock().unwrap();
        assert_eq!(guard.len(), 4);

        // Check first entry (info, Lighting)
        assert_eq!(guard[0].level, LogLevel::Info);
        assert_eq!(guard[0].category.as_deref(), Some("Lighting"));
        assert!(guard[0].message.contains("Light created"));

        // Check second entry (warn, Command)
        assert_eq!(guard[1].level, LogLevel::Warning);
        assert_eq!(guard[1].category.as_deref(), Some("Command"));

        // Check third entry (error, Viewport)
        assert_eq!(guard[2].level, LogLevel::Error);
        assert_eq!(guard[2].category.as_deref(), Some("Viewport"));

        // Check fourth entry (debug, Scene)
        assert_eq!(guard[3].level, LogLevel::Debug);
        assert_eq!(guard[3].category.as_deref(), Some("Scene"));
    }
}
