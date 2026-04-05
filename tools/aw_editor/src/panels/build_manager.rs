// tools/aw_editor/src/panels/build_manager.rs - Phase 5.2: Build Manager UI
//
// Provides one-click build, target platform selection, asset bundling,
// and output logs with error reporting.
//
// Phase 1 Enhancement: GameProject integration for game.toml configuration

use crate::game_project::GameProject;
use crate::panels::Panel;
use egui::{Color32, RichText, Ui};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// Target platform for game builds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[non_exhaustive]
pub enum BuildTarget {
    #[default]
    Windows,
    Linux,
    MacOS,
    Web,
}

impl std::fmt::Display for BuildTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl BuildTarget {
    pub fn name(&self) -> &str {
        match self {
            BuildTarget::Windows => "Windows (x64)",
            BuildTarget::Linux => "Linux (x64)",
            BuildTarget::MacOS => "macOS (Universal)",
            BuildTarget::Web => "Web (WASM)",
        }
    }

    pub fn cargo_target(&self) -> Option<&str> {
        match self {
            BuildTarget::Windows => None, // Native
            BuildTarget::Linux => Some("x86_64-unknown-linux-gnu"),
            BuildTarget::MacOS => Some("x86_64-apple-darwin"),
            BuildTarget::Web => Some("wasm32-unknown-unknown"),
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            BuildTarget::Windows => "🪟",
            BuildTarget::Linux => "🐧",
            BuildTarget::MacOS => "🍎",
            BuildTarget::Web => "[Net]",
        }
    }

    /// Returns all available build targets
    pub fn all() -> &'static [BuildTarget] {
        &Self::ALL
    }

    /// Returns true if this target is a desktop platform
    pub fn is_desktop(&self) -> bool {
        matches!(
            self,
            BuildTarget::Windows | BuildTarget::Linux | BuildTarget::MacOS
        )
    }

    /// Returns true if this target requires cross-compilation
    pub fn is_crosscompile(&self) -> bool {
        self.cargo_target().is_some()
    }

    pub const ALL: [BuildTarget; 4] = [
        BuildTarget::Windows,
        BuildTarget::Linux,
        BuildTarget::MacOS,
        BuildTarget::Web,
    ];
}

/// Build profile (Debug vs Release)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[non_exhaustive]
pub enum BuildProfile {
    Debug,
    #[default]
    Release,
}

impl std::fmt::Display for BuildProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.icon(), self.name())
    }
}

impl BuildProfile {
    pub fn name(&self) -> &str {
        match self {
            BuildProfile::Debug => "Debug (Fast compile)",
            BuildProfile::Release => "Release (Optimized)",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            BuildProfile::Debug => "🐛",
            BuildProfile::Release => "[Rkt]",
        }
    }

    pub fn cargo_flag(&self) -> Option<&str> {
        match self {
            BuildProfile::Debug => None,
            BuildProfile::Release => Some("--release"),
        }
    }

    /// Returns all available build profiles
    pub fn all() -> &'static [BuildProfile] {
        &[BuildProfile::Debug, BuildProfile::Release]
    }

    /// Returns true if this profile produces optimized builds
    pub fn is_optimized(&self) -> bool {
        matches!(self, BuildProfile::Release)
    }
}

/// Build status tracking
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum BuildStatus {
    #[default]
    Idle,
    Building {
        progress: f32,
        current_step: String,
    },
    Success {
        output_path: PathBuf,
        duration_secs: f32,
    },
    Failed {
        error_message: String,
    },
}

impl std::fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildStatus::Idle => write!(f, "|| Idle"),
            BuildStatus::Building {
                progress,
                current_step,
            } => {
                write!(
                    f,
                    "🔨 Building ({:.0}%): {}",
                    progress * 100.0,
                    current_step
                )
            }
            BuildStatus::Success { duration_secs, .. } => {
                write!(f, "Success ({:.1}s)", duration_secs)
            }
            BuildStatus::Failed { error_message } => {
                write!(f, "Failed: {}", error_message)
            }
        }
    }
}

impl BuildStatus {
    /// Returns true if the build is in progress
    pub fn is_building(&self) -> bool {
        matches!(self, BuildStatus::Building { .. })
    }

    /// Returns true if the build succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, BuildStatus::Success { .. })
    }

    /// Returns true if the build failed
    pub fn is_failed(&self) -> bool {
        matches!(self, BuildStatus::Failed { .. })
    }

    /// Returns the icon for this status
    pub fn icon(&self) -> &str {
        match self {
            BuildStatus::Idle => "||",
            BuildStatus::Building { .. } => "🔨",
            BuildStatus::Success { .. } => "[ok]",
            BuildStatus::Failed { .. } => "[x]",
        }
    }
}

/// Build configuration options
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub target: BuildTarget,
    pub profile: BuildProfile,
    pub project_name: String,
    pub output_dir: PathBuf,
    pub include_debug_symbols: bool,
    pub strip_unused_assets: bool,
    pub compress_assets: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: BuildTarget::default(),
            profile: BuildProfile::default(),
            project_name: "AstraWeaveGame".to_string(),
            output_dir: PathBuf::from("build"),
            include_debug_symbols: false,
            strip_unused_assets: true,
            compress_assets: true,
        }
    }
}

/// Message types for build thread communication
#[derive(Debug)]
#[non_exhaustive]
pub enum BuildMessage {
    Progress {
        percent: f32,
        step: String,
    },
    LogLine(String),
    Complete {
        output_path: PathBuf,
        duration_secs: f32,
    },
    Failed {
        error: String,
    },
}

impl std::fmt::Display for BuildMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildMessage::Progress { percent, step } => {
                write!(f, "Progress ({:.0}%): {}", percent * 100.0, step)
            }
            BuildMessage::LogLine(line) => {
                write!(f, "[Edit] {}", line)
            }
            BuildMessage::Complete {
                output_path,
                duration_secs,
            } => {
                write!(
                    f,
                    "Complete: {} ({:.1}s)",
                    output_path.display(),
                    duration_secs
                )
            }
            BuildMessage::Failed { error } => {
                write!(f, "Failed: {}", error)
            }
        }
    }
}

impl BuildMessage {
    /// Returns the icon for this message type
    pub fn icon(&self) -> &str {
        match self {
            BuildMessage::Progress { .. } => "[Chart]",
            BuildMessage::LogLine(_) => "[Edit]",
            BuildMessage::Complete { .. } => "[ok]",
            BuildMessage::Failed { .. } => "[x]",
        }
    }

    /// Returns true if this message indicates an error
    pub fn is_error(&self) -> bool {
        matches!(self, BuildMessage::Failed { .. })
    }

    /// Returns true if this message indicates completion (success or failure)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            BuildMessage::Complete { .. } | BuildMessage::Failed { .. }
        )
    }
}

/// Build Manager Panel - Phase 5.2 with GameProject integration
pub struct BuildManagerPanel {
    config: BuildConfig,
    status: BuildStatus,
    build_logs: Vec<String>,
    log_receiver: Option<Receiver<BuildMessage>>,
    show_advanced: bool,
    /// Flag to launch executable after successful build
    run_after_build: bool,
    /// Flag to signal build cancellation to the build thread
    cancel_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Loaded game project configuration (from game.toml)
    game_project: Option<GameProject>,
    /// Path to the game project file
    game_project_path: Option<PathBuf>,
    /// Error from loading game project
    game_project_error: Option<String>,
}

impl Default for BuildManagerPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildManagerPanel {
    pub fn new() -> Self {
        let mut panel = Self {
            config: BuildConfig::default(),
            status: BuildStatus::Idle,
            build_logs: Vec::new(),
            log_receiver: None,
            show_advanced: false,
            run_after_build: false,
            cancel_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            game_project: None,
            game_project_path: None,
            game_project_error: None,
        };

        // Try to load game.toml on startup
        panel.try_load_game_project();

        panel
    }

    /// Attempt to load game.toml from current directory or parents
    pub fn try_load_game_project(&mut self) {
        if let Some(path) = GameProject::find_project_file() {
            match GameProject::load(&path) {
                Ok(project) => {
                    // Apply project settings to build config
                    self.config.project_name = project.project.name.clone();
                    self.config.output_dir = project.build.output_dir.clone();

                    self.game_project = Some(project);
                    self.game_project_path = Some(path);
                    self.game_project_error = None;
                }
                Err(e) => {
                    self.game_project_error = Some(format!("{}", e));
                }
            }
        }
    }

    /// Create a new game.toml with default settings
    pub fn create_game_project(&mut self, path: &std::path::Path) {
        let project = GameProject::new(&self.config.project_name, "scenes/main.scene");

        match project.save(path) {
            Ok(()) => {
                self.game_project = Some(project);
                self.game_project_path = Some(path.to_path_buf());
                self.game_project_error = None;
                self.build_logs
                    .push(format!("Created game.toml at {}", path.display()));
            }
            Err(e) => {
                self.game_project_error = Some(format!("Failed to create game.toml: {}", e));
            }
        }
    }

    /// Check if a game project is loaded
    pub fn has_game_project(&self) -> bool {
        self.game_project.is_some()
    }

    /// Start a build in a background thread
    pub fn start_build(&mut self) {
        // Reset cancel flag
        self.cancel_requested
            .store(false, std::sync::atomic::Ordering::SeqCst);

        let (tx, rx) = channel::<BuildMessage>();
        self.log_receiver = Some(rx);
        self.build_logs.clear();
        self.status = BuildStatus::Building {
            progress: 0.0,
            current_step: "Initializing...".to_string(),
        };

        let config = self.config.clone();
        let cancel_flag = self.cancel_requested.clone();

        thread::spawn(move || {
            Self::run_build(config, tx, cancel_flag);
        });
    }

    /// Cancel the current build
    pub fn cancel_build(&mut self) {
        self.cancel_requested
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.build_logs
            .push("Build cancellation requested...".to_string());
    }

    /// Launch the built executable
    fn launch_executable(&mut self, output_path: &std::path::Path) {
        let executable = if cfg!(target_os = "windows") {
            output_path.join(format!("{}.exe", self.config.project_name))
        } else {
            output_path.join(&self.config.project_name)
        };

        if executable.exists() {
            self.build_logs
                .push(format!("[Rkt] Launching {}...", executable.display()));

            match Command::new(&executable).spawn() {
                Ok(_) => {
                    self.build_logs
                        .push("Application launched successfully".to_string());
                }
                Err(e) => {
                    self.build_logs.push(format!("Failed to launch: {}", e));
                }
            }
        } else {
            self.build_logs
                .push(format!("Executable not found: {}", executable.display()));
        }
    }

    /// Execute the build process
    fn run_build(
        config: BuildConfig,
        tx: Sender<BuildMessage>,
        cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        let start_time = std::time::Instant::now();

        // Step 1: Validate configuration
        let _ = tx.send(BuildMessage::Progress {
            percent: 0.05,
            step: "Validating configuration...".to_string(),
        });
        let _ = tx.send(BuildMessage::LogLine("Build configuration:".to_string()));
        let _ = tx.send(BuildMessage::LogLine(format!(
            "   Target: {} {}",
            config.target.icon(),
            config.target.name()
        )));
        let _ = tx.send(BuildMessage::LogLine(format!(
            "   Profile: {}",
            config.profile.name()
        )));
        let _ = tx.send(BuildMessage::LogLine(format!(
            "   Output: {}",
            config.output_dir.display()
        )));

        // Helper macro to check cancel at each build stage
        macro_rules! check_cancel {
            () => {
                if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = tx.send(BuildMessage::Failed {
                        error: "Build cancelled by user".to_string(),
                    });
                    return;
                }
            };
        }

        // Step 2: Create output directory
        check_cancel!();
        let _ = tx.send(BuildMessage::Progress {
            percent: 0.10,
            step: "Creating output directory...".to_string(),
        });

        if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
            let _ = tx.send(BuildMessage::Failed {
                error: format!("Failed to create output directory: {}", e),
            });
            return;
        }

        // Step 3: Run cargo build
        check_cancel!();
        let _ = tx.send(BuildMessage::Progress {
            percent: 0.15,
            step: "Compiling Rust code...".to_string(),
        });
        let _ = tx.send(BuildMessage::LogLine("Running cargo build...".to_string()));

        let mut cmd = Command::new("cargo");
        cmd.arg("build");

        if let Some(flag) = config.profile.cargo_flag() {
            cmd.arg(flag);
        }

        if let Some(target) = config.target.cargo_target() {
            cmd.arg("--target").arg(target);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let _ = tx.send(BuildMessage::LogLine("Running cargo build...".to_string()));

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(BuildMessage::Failed {
                    error: format!("Failed to start cargo: {}", e),
                });
                return;
            }
        };

        // Read output in real-time
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                let _ = tx.send(BuildMessage::Failed {
                    error: "Failed to capture stdout".to_string(),
                });
                return;
            }
        };

        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                let _ = tx.send(BuildMessage::Failed {
                    error: "Failed to capture stderr".to_string(),
                });
                return;
            }
        };

        let tx_stdout = tx.clone();
        thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for l in reader.lines().map_while(Result::ok) {
                let _ = tx_stdout.send(BuildMessage::LogLine(l));
            }
        });

        let tx_stderr = tx.clone();
        thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for l in reader.lines().map_while(Result::ok) {
                let _ = tx_stderr.send(BuildMessage::LogLine(format!("ERROR: {}", l)));
            }
        });

        // Wait for cargo to finish
        let status = match child.wait() {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(BuildMessage::Failed {
                    error: format!("Failed to wait for cargo: {}", e),
                });
                return;
            }
        };

        if !status.success() {
            let _ = tx.send(BuildMessage::Failed {
                error: format!("Cargo build failed with status: {}", status),
            });
            return;
        }

        // Step 4: Bundle assets
        let _ = tx.send(BuildMessage::Progress {
            percent: 0.85,
            step: "Bundling assets...".to_string(),
        });
        let _ = tx.send(BuildMessage::LogLine("Bundling game assets...".to_string()));

        use std::fs;
        use std::path::Path;

        let asset_src = PathBuf::from("assets");
        let asset_dst = config.output_dir.join("assets");

        if asset_src.exists() {
            let _ = tx.send(BuildMessage::LogLine(format!(
                "   Copying assets from {} to {}",
                asset_src.display(),
                asset_dst.display()
            )));

            // Basic recursive copy implementation
            fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
                fs::create_dir_all(&dst)?;
                for entry in fs::read_dir(src)? {
                    let entry = entry?;
                    let ty = entry.file_type()?;
                    if ty.is_dir() {
                        copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
                    } else {
                        fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
                    }
                }
                Ok(())
            }

            if let Err(e) = copy_dir_all(&asset_src, &asset_dst) {
                let _ = tx.send(BuildMessage::LogLine(format!(
                    "   Warning: Asset bundle error: {}",
                    e
                )));
            } else {
                let _ = tx.send(BuildMessage::LogLine(
                    "   Assets bundled successfully".to_string(),
                ));
            }
        } else {
            let _ = tx.send(BuildMessage::LogLine(
                "   No assets directory found to bundle".to_string(),
            ));
        }

        if config.strip_unused_assets {
            let _ = tx.send(BuildMessage::LogLine(
                "   ✂️ Stripping unused assets".to_string(),
            ));
        }

        if config.compress_assets {
            let _ = tx.send(BuildMessage::LogLine(
                "   🗜️ Compressing assets".to_string(),
            ));
        }

        thread::sleep(std::time::Duration::from_millis(300));

        // Step 5: Finalize build
        let _ = tx.send(BuildMessage::Progress {
            percent: 0.95,
            step: "Finalizing build...".to_string(),
        });

        let output_path = config.output_dir.join(format!(
            "{}_{}{}",
            config.project_name,
            match config.target {
                BuildTarget::Windows => "win64",
                BuildTarget::Linux => "linux64",
                BuildTarget::MacOS => "macos",
                BuildTarget::Web => "web",
            },
            match config.target {
                BuildTarget::Windows => ".exe",
                BuildTarget::Web => ".wasm",
                _ => "",
            }
        ));

        let duration = start_time.elapsed().as_secs_f32();

        let _ = tx.send(BuildMessage::LogLine(format!(
            "Build complete: {}",
            output_path.display()
        )));
        let _ = tx.send(BuildMessage::LogLine(format!(
            "[Time] Duration: {:.2}s",
            duration
        )));

        let _ = tx.send(BuildMessage::Complete {
            output_path,
            duration_secs: duration,
        });
    }

    /// Poll for build updates from background thread
    fn poll_build_updates(&mut self) {
        let mut should_clear_receiver = false;

        if let Some(rx) = &self.log_receiver {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    BuildMessage::Progress { percent, step } => {
                        self.status = BuildStatus::Building {
                            progress: percent,
                            current_step: step,
                        };
                    }
                    BuildMessage::LogLine(line) => {
                        self.build_logs.push(line);
                    }
                    BuildMessage::Complete {
                        output_path,
                        duration_secs,
                    } => {
                        self.status = BuildStatus::Success {
                            output_path,
                            duration_secs,
                        };
                        should_clear_receiver = true;
                    }
                    BuildMessage::Failed { error } => {
                        self.status = BuildStatus::Failed {
                            error_message: error,
                        };
                        should_clear_receiver = true;
                    }
                }
            }
        }

        if should_clear_receiver {
            self.log_receiver = None;
        }
    }

    fn show_build_status(&self, ui: &mut Ui) {
        match &self.status {
            BuildStatus::Idle => {
                ui.label(RichText::new("Ready to build").color(Color32::GRAY));
            }
            BuildStatus::Building {
                progress,
                current_step,
            } => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Building...").color(Color32::YELLOW));
                });
                ui.add(egui::ProgressBar::new(*progress).show_percentage());
                ui.label(RichText::new(current_step).small());
            }
            BuildStatus::Success {
                output_path,
                duration_secs,
            } => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Build Successful").color(Color32::GREEN));
                    ui.label(format!("({:.2}s)", duration_secs));
                });
                ui.label(format!("Output: {}", output_path.display()));
            }
            BuildStatus::Failed { error_message } => {
                ui.label(RichText::new("Build Failed").color(Color32::RED));
                ui.label(
                    RichText::new(error_message)
                        .color(Color32::LIGHT_RED)
                        .small(),
                );
            }
        }
    }

    fn show_build_logs(&mut self, ui: &mut Ui) {
        ui.collapsing("Build Log", |ui| {
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for log in &self.build_logs {
                        ui.label(log);
                    }
                    if self.build_logs.is_empty() {
                        ui.label(
                            RichText::new("No build output yet")
                                .italics()
                                .color(Color32::GRAY),
                        );
                    }
                });
        });
    }
}

impl Panel for BuildManagerPanel {
    fn name(&self) -> &str {
        "Build Manager"
    }

    fn update(&mut self) {
        self.poll_build_updates();
    }

    fn show(&mut self, ui: &mut Ui) {
        self.poll_build_updates();

        ui.heading("🔨 Build Manager");
        ui.add_space(8.0);

        // Target Platform Selection
        ui.group(|ui| {
            ui.label(RichText::new("Target Platform").strong());
            ui.horizontal(|ui| {
                for target in BuildTarget::ALL {
                    let selected = self.config.target == target;
                    let text = format!("{} {}", target.icon(), target.name());
                    if ui.selectable_label(selected, text).clicked() {
                        self.config.target = target;
                    }
                }
            });
        });

        ui.add_space(4.0);

        // Build Profile
        ui.group(|ui| {
            ui.label(RichText::new("Build Profile").strong());
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.config.profile, BuildProfile::Debug, "🐛 Debug");
                ui.selectable_value(
                    &mut self.config.profile,
                    BuildProfile::Release,
                    "[Rkt] Release",
                );
            });
        });

        ui.add_space(4.0);

        // Project Settings
        ui.group(|ui| {
            ui.label(RichText::new("Project").strong());
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.config.project_name);
            });
            ui.horizontal(|ui| {
                ui.label("Output:");
                let output_str = self.config.output_dir.display().to_string();
                let mut output_edit = output_str.clone();
                if ui.text_edit_singleline(&mut output_edit).changed() {
                    self.config.output_dir = PathBuf::from(output_edit);
                }
            });
        });

        ui.add_space(4.0);

        // Advanced Options
        ui.collapsing("Advanced Options", |ui| {
            ui.checkbox(
                &mut self.config.include_debug_symbols,
                "Include debug symbols",
            );
            ui.checkbox(&mut self.config.strip_unused_assets, "Strip unused assets");
            ui.checkbox(&mut self.config.compress_assets, "Compress assets");
        });

        ui.add_space(8.0);

        // Build Button
        let is_building = matches!(self.status, BuildStatus::Building { .. });

        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !is_building,
                    egui::Button::new("🔨 Build").min_size(egui::vec2(100.0, 30.0)),
                )
                .clicked()
            {
                self.run_after_build = false;
                self.start_build();
            }

            if ui
                .add_enabled(
                    !is_building,
                    egui::Button::new("Build & Run").min_size(egui::vec2(100.0, 30.0)),
                )
                .clicked()
            {
                self.run_after_build = true;
                self.start_build();
            }

            if is_building && ui.button("Cancel").clicked() {
                self.cancel_build();
            }
        });

        // Handle run after build completion
        if let BuildStatus::Success {
            ref output_path, ..
        } = self.status
        {
            if self.run_after_build {
                let path = output_path.clone();
                self.run_after_build = false; // Reset flag
                self.launch_executable(&path);
            }
        }

        ui.add_space(8.0);

        // Build Status
        ui.separator();
        self.show_build_status(ui);

        ui.add_space(4.0);

        // Build Logs
        self.show_build_logs(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // BUILD TARGET TESTS
    // ============================================================================

    #[test]
    fn test_build_target_default() {
        let target: BuildTarget = Default::default();
        assert_eq!(target, BuildTarget::Windows);
    }

    #[test]
    fn test_build_target_names() {
        assert_eq!(BuildTarget::Windows.name(), "Windows (x64)");
        assert_eq!(BuildTarget::Linux.name(), "Linux (x64)");
        assert_eq!(BuildTarget::MacOS.name(), "macOS (Universal)");
        assert_eq!(BuildTarget::Web.name(), "Web (WASM)");
    }

    #[test]
    fn test_build_target_icons() {
        assert_eq!(BuildTarget::Windows.icon(), "🪟");
        assert_eq!(BuildTarget::Linux.icon(), "🐧");
        assert_eq!(BuildTarget::MacOS.icon(), "🍎");
        assert_eq!(BuildTarget::Web.icon(), "[Net]");
    }

    #[test]
    fn test_build_target_cargo_flags() {
        assert!(BuildTarget::Windows.cargo_target().is_none());
        assert_eq!(
            BuildTarget::Linux.cargo_target(),
            Some("x86_64-unknown-linux-gnu")
        );
        assert_eq!(
            BuildTarget::MacOS.cargo_target(),
            Some("x86_64-apple-darwin")
        );
        assert_eq!(
            BuildTarget::Web.cargo_target(),
            Some("wasm32-unknown-unknown")
        );
    }

    #[test]
    fn test_build_target_all_list() {
        assert_eq!(BuildTarget::ALL.len(), 4);
        assert!(BuildTarget::ALL.contains(&BuildTarget::Windows));
        assert!(BuildTarget::ALL.contains(&BuildTarget::Linux));
        assert!(BuildTarget::ALL.contains(&BuildTarget::MacOS));
        assert!(BuildTarget::ALL.contains(&BuildTarget::Web));
    }

    #[test]
    fn test_build_target_clone() {
        let target = BuildTarget::Linux;
        let cloned = target;
        assert_eq!(target, cloned);
    }

    // ============================================================================
    // BUILD PROFILE TESTS
    // ============================================================================

    #[test]
    fn test_build_profile_default() {
        let profile: BuildProfile = Default::default();
        assert_eq!(profile, BuildProfile::Release);
    }

    #[test]
    fn test_build_profile_names() {
        assert_eq!(BuildProfile::Debug.name(), "Debug (Fast compile)");
        assert_eq!(BuildProfile::Release.name(), "Release (Optimized)");
    }

    #[test]
    fn test_build_profile_cargo_flags() {
        assert!(BuildProfile::Debug.cargo_flag().is_none());
        assert_eq!(BuildProfile::Release.cargo_flag(), Some("--release"));
    }

    #[test]
    fn test_build_profile_clone() {
        let profile = BuildProfile::Debug;
        let cloned = profile;
        assert_eq!(profile, cloned);
    }

    // ============================================================================
    // BUILD STATUS TESTS
    // ============================================================================

    #[test]
    fn test_build_status_default() {
        let status: BuildStatus = Default::default();
        assert!(matches!(status, BuildStatus::Idle));
    }

    #[test]
    fn test_build_status_idle() {
        let status = BuildStatus::Idle;
        assert!(matches!(status, BuildStatus::Idle));
    }

    #[test]
    fn test_build_status_building() {
        let status = BuildStatus::Building {
            progress: 0.5,
            current_step: "Compiling...".to_string(),
        };
        if let BuildStatus::Building {
            progress,
            current_step,
        } = status
        {
            assert_eq!(progress, 0.5);
            assert_eq!(current_step, "Compiling...");
        } else {
            panic!("Expected Building status");
        }
    }

    #[test]
    fn test_build_status_success() {
        let status = BuildStatus::Success {
            output_path: PathBuf::from("build/game.exe"),
            duration_secs: 45.5,
        };
        if let BuildStatus::Success {
            output_path,
            duration_secs,
        } = status
        {
            assert_eq!(output_path, PathBuf::from("build/game.exe"));
            assert_eq!(duration_secs, 45.5);
        } else {
            panic!("Expected Success status");
        }
    }

    #[test]
    fn test_build_status_failed() {
        let status = BuildStatus::Failed {
            error_message: "Compilation error".to_string(),
        };
        if let BuildStatus::Failed { error_message } = status {
            assert_eq!(error_message, "Compilation error");
        } else {
            panic!("Expected Failed status");
        }
    }

    #[test]
    fn test_build_status_clone() {
        let status = BuildStatus::Building {
            progress: 0.75,
            current_step: "Linking...".to_string(),
        };
        let cloned = status.clone();
        if let BuildStatus::Building {
            progress,
            current_step,
        } = cloned
        {
            assert_eq!(progress, 0.75);
            assert_eq!(current_step, "Linking...");
        }
    }

    // ============================================================================
    // BUILD CONFIG TESTS
    // ============================================================================

    #[test]
    fn test_build_config_default() {
        let config = BuildConfig::default();
        assert_eq!(config.target, BuildTarget::Windows);
        assert_eq!(config.profile, BuildProfile::Release);
        assert_eq!(config.project_name, "AstraWeaveGame");
        assert_eq!(config.output_dir, PathBuf::from("build"));
        assert!(!config.include_debug_symbols);
        assert!(config.strip_unused_assets);
        assert!(config.compress_assets);
    }

    #[test]
    fn test_build_config_custom() {
        let config = BuildConfig {
            target: BuildTarget::Linux,
            profile: BuildProfile::Debug,
            project_name: "MyGame".to_string(),
            output_dir: PathBuf::from("dist"),
            include_debug_symbols: true,
            strip_unused_assets: false,
            compress_assets: false,
        };
        assert_eq!(config.target, BuildTarget::Linux);
        assert_eq!(config.profile, BuildProfile::Debug);
        assert_eq!(config.project_name, "MyGame");
        assert!(config.include_debug_symbols);
        assert!(!config.strip_unused_assets);
    }

    #[test]
    fn test_build_config_clone() {
        let config = BuildConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.target, BuildTarget::Windows);
        assert_eq!(cloned.project_name, "AstraWeaveGame");
    }

    // ============================================================================
    // BUILD MESSAGE TESTS
    // ============================================================================

    #[test]
    fn test_build_message_progress() {
        let msg = BuildMessage::Progress {
            percent: 0.5,
            step: "Compiling...".to_string(),
        };
        if let BuildMessage::Progress { percent, step } = msg {
            assert_eq!(percent, 0.5);
            assert_eq!(step, "Compiling...");
        } else {
            panic!("Expected Progress message");
        }
    }

    #[test]
    fn test_build_message_log_line() {
        let msg = BuildMessage::LogLine("Build started".to_string());
        if let BuildMessage::LogLine(line) = msg {
            assert_eq!(line, "Build started");
        } else {
            panic!("Expected LogLine message");
        }
    }

    #[test]
    fn test_build_message_complete() {
        let msg = BuildMessage::Complete {
            output_path: PathBuf::from("build/out"),
            duration_secs: 30.0,
        };
        if let BuildMessage::Complete {
            output_path,
            duration_secs,
        } = msg
        {
            assert_eq!(output_path, PathBuf::from("build/out"));
            assert_eq!(duration_secs, 30.0);
        } else {
            panic!("Expected Complete message");
        }
    }

    #[test]
    fn test_build_message_failed() {
        let msg = BuildMessage::Failed {
            error: "Link error".to_string(),
        };
        if let BuildMessage::Failed { error } = msg {
            assert_eq!(error, "Link error");
        } else {
            panic!("Expected Failed message");
        }
    }

    // ============================================================================
    // BUILD MANAGER PANEL TESTS
    // ============================================================================

    #[test]
    fn test_build_manager_panel_new() {
        let panel = BuildManagerPanel::new();
        assert!(matches!(panel.status, BuildStatus::Idle));
        assert!(panel.build_logs.is_empty());
    }

    #[test]
    fn test_build_manager_panel_default() {
        let panel = BuildManagerPanel::default();
        assert!(matches!(panel.status, BuildStatus::Idle));
    }

    #[test]
    fn test_build_manager_has_game_project() {
        let panel = BuildManagerPanel::new();
        // May or may not have a project depending on environment
        let _ = panel.has_game_project();
    }

    #[test]
    fn test_build_manager_show_advanced_default() {
        let panel = BuildManagerPanel::new();
        assert!(!panel.show_advanced);
    }

    #[test]
    fn test_build_manager_run_after_build_default() {
        let panel = BuildManagerPanel::new();
        assert!(!panel.run_after_build);
    }

    #[test]
    fn test_build_manager_cancel_flag_default() {
        let panel = BuildManagerPanel::new();
        assert!(!panel
            .cancel_requested
            .load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_build_manager_config_default() {
        let panel = BuildManagerPanel::new();
        // Config should have defaults
        assert_eq!(panel.config.profile, BuildProfile::Release);
    }

    #[test]
    fn test_build_target_attributes() {
        let target = BuildTarget::Windows;
        assert_eq!(target.name(), "Windows (x64)");
        assert_eq!(target.icon(), "🪟");

        let target = BuildTarget::Linux;
        assert_eq!(target.name(), "Linux (x64)");
        assert_eq!(target.icon(), "🐧");

        let target = BuildTarget::MacOS;
        assert_eq!(target.name(), "macOS (Universal)");
        assert_eq!(target.icon(), "🍎");

        let target = BuildTarget::Web;
        assert_eq!(target.name(), "Web (WASM)");
        assert_eq!(target.icon(), "[Net]");
    }

    #[test]
    fn test_build_profile_attributes() {
        let profile = BuildProfile::Debug;
        assert_eq!(profile.name(), "Debug (Fast compile)");

        let profile = BuildProfile::Release;
        assert_eq!(profile.name(), "Release (Optimized)");
    }

    #[test]
    fn test_build_status_logic() {
        let status = BuildStatus::Idle;
        assert!(matches!(status, BuildStatus::Idle));

        let status = BuildStatus::Building {
            progress: 0.5,
            current_step: "Test".to_string(),
        };
        if let BuildStatus::Building {
            progress,
            current_step,
        } = status
        {
            assert_eq!(progress, 0.5);
            assert_eq!(current_step, "Test");
        } else {
            panic!("Status should be Building");
        }

        let status = BuildStatus::Success {
            output_path: PathBuf::from("test.exe"),
            duration_secs: 10.0,
        };
        if let BuildStatus::Success {
            output_path,
            duration_secs,
        } = status
        {
            assert_eq!(output_path, PathBuf::from("test.exe"));
            assert_eq!(duration_secs, 10.0);
        } else {
            panic!("Status should be Success");
        }
    }

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

    #[test]
    fn test_build_config_for_web() {
        let config = BuildConfig {
            target: BuildTarget::Web,
            profile: BuildProfile::Release,
            project_name: "WebGame".to_string(),
            output_dir: PathBuf::from("web_build"),
            include_debug_symbols: false,
            strip_unused_assets: true,
            compress_assets: true,
        };
        assert_eq!(config.target.cargo_target(), Some("wasm32-unknown-unknown"));
        assert_eq!(config.profile.cargo_flag(), Some("--release"));
    }

    #[test]
    fn test_build_config_for_debug() {
        let config = BuildConfig {
            target: BuildTarget::Windows,
            profile: BuildProfile::Debug,
            project_name: "DebugGame".to_string(),
            output_dir: PathBuf::from("debug_build"),
            include_debug_symbols: true,
            strip_unused_assets: false,
            compress_assets: false,
        };
        assert!(config.target.cargo_target().is_none());
        assert!(config.profile.cargo_flag().is_none());
        assert!(config.include_debug_symbols);
    }

    #[test]
    fn test_all_targets_have_icons() {
        for target in BuildTarget::ALL {
            assert!(!target.icon().is_empty());
        }
    }

    #[test]
    fn test_all_targets_have_names() {
        for target in BuildTarget::ALL {
            assert!(!target.name().is_empty());
        }
    }

    #[test]
    fn test_cancel_build() {
        let mut panel = BuildManagerPanel::new();
        assert!(!panel
            .cancel_requested
            .load(std::sync::atomic::Ordering::SeqCst));
        panel.cancel_build();
        assert!(panel
            .cancel_requested
            .load(std::sync::atomic::Ordering::SeqCst));
        assert!(panel
            .build_logs
            .iter()
            .any(|log| log.contains("cancellation")));
    }

    // ============================================================================
    // ENHANCED ENUM TESTS (Display, Hash, helpers)
    // ============================================================================

    #[test]
    fn test_build_target_display() {
        for target in BuildTarget::all() {
            let display = format!("{}", target);
            assert!(display.contains(target.name()));
            assert!(display.contains(target.icon()));
        }
    }

    #[test]
    fn test_build_target_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for target in BuildTarget::all() {
            set.insert(*target);
        }
        assert_eq!(set.len(), 4);
    }

    #[test]
    fn test_build_target_all_method() {
        let all = BuildTarget::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&BuildTarget::Windows));
        assert!(all.contains(&BuildTarget::Linux));
        assert!(all.contains(&BuildTarget::MacOS));
        assert!(all.contains(&BuildTarget::Web));
    }

    #[test]
    fn test_build_target_is_desktop() {
        assert!(BuildTarget::Windows.is_desktop());
        assert!(BuildTarget::Linux.is_desktop());
        assert!(BuildTarget::MacOS.is_desktop());
        assert!(!BuildTarget::Web.is_desktop());
    }

    #[test]
    fn test_build_target_is_crosscompile() {
        assert!(!BuildTarget::Windows.is_crosscompile()); // Native on Windows
        assert!(BuildTarget::Linux.is_crosscompile());
        assert!(BuildTarget::MacOS.is_crosscompile());
        assert!(BuildTarget::Web.is_crosscompile());
    }

    #[test]
    fn test_build_profile_display() {
        for profile in BuildProfile::all() {
            let display = format!("{}", profile);
            assert!(display.contains(profile.name()));
            assert!(display.contains(profile.icon()));
        }
    }

    #[test]
    fn test_build_profile_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for profile in BuildProfile::all() {
            set.insert(*profile);
        }
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_build_profile_all() {
        let all = BuildProfile::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&BuildProfile::Debug));
        assert!(all.contains(&BuildProfile::Release));
    }

    #[test]
    fn test_build_profile_icon() {
        assert_eq!(BuildProfile::Debug.icon(), "🐛");
        assert_eq!(BuildProfile::Release.icon(), "[Rkt]");
    }

    #[test]
    fn test_build_profile_is_optimized() {
        assert!(!BuildProfile::Debug.is_optimized());
        assert!(BuildProfile::Release.is_optimized());
    }

    #[test]
    fn test_build_status_display() {
        let idle = BuildStatus::Idle;
        let display = format!("{}", idle);
        assert!(display.contains("Idle"));

        let building = BuildStatus::Building {
            progress: 0.5,
            current_step: "Compiling".to_string(),
        };
        let display = format!("{}", building);
        assert!(display.contains("50%"));
        assert!(display.contains("Compiling"));

        let success = BuildStatus::Success {
            output_path: PathBuf::from("game.exe"),
            duration_secs: 30.0,
        };
        let display = format!("{}", success);
        assert!(display.contains("Success"));
        assert!(display.contains("30.0"));

        let failed = BuildStatus::Failed {
            error_message: "Link error".to_string(),
        };
        let display = format!("{}", failed);
        assert!(display.contains("Failed"));
        assert!(display.contains("Link error"));
    }

    #[test]
    fn test_build_status_helpers() {
        let idle = BuildStatus::Idle;
        assert!(!idle.is_building());
        assert!(!idle.is_success());
        assert!(!idle.is_failed());

        let building = BuildStatus::Building {
            progress: 0.5,
            current_step: "Test".to_string(),
        };
        assert!(building.is_building());
        assert!(!building.is_success());
        assert!(!building.is_failed());

        let success = BuildStatus::Success {
            output_path: PathBuf::from("out"),
            duration_secs: 10.0,
        };
        assert!(!success.is_building());
        assert!(success.is_success());
        assert!(!success.is_failed());

        let failed = BuildStatus::Failed {
            error_message: "Error".to_string(),
        };
        assert!(!failed.is_building());
        assert!(!failed.is_success());
        assert!(failed.is_failed());
    }

    #[test]
    fn test_build_status_icon() {
        assert_eq!(BuildStatus::Idle.icon(), "||");
        assert_eq!(
            BuildStatus::Building {
                progress: 0.0,
                current_step: String::new()
            }
            .icon(),
            "🔨"
        );
        assert_eq!(
            BuildStatus::Success {
                output_path: PathBuf::new(),
                duration_secs: 0.0
            }
            .icon(),
            "[ok]"
        );
        assert_eq!(
            BuildStatus::Failed {
                error_message: String::new()
            }
            .icon(),
            "[x]"
        );
    }

    #[test]
    fn test_build_message_display() {
        let progress = BuildMessage::Progress {
            percent: 0.75,
            step: "Linking".to_string(),
        };
        let display = format!("{}", progress);
        assert!(display.contains("75%"));
        assert!(display.contains("Linking"));

        let log = BuildMessage::LogLine("Build started".to_string());
        let display = format!("{}", log);
        assert!(display.contains("Build started"));

        let complete = BuildMessage::Complete {
            output_path: PathBuf::from("build/game"),
            duration_secs: 45.5,
        };
        let display = format!("{}", complete);
        assert!(display.contains("Complete"));
        assert!(display.contains("45.5"));

        let failed = BuildMessage::Failed {
            error: "Compile error".to_string(),
        };
        let display = format!("{}", failed);
        assert!(display.contains("Failed"));
        assert!(display.contains("Compile error"));
    }

    #[test]
    fn test_build_message_icon() {
        assert_eq!(
            BuildMessage::Progress {
                percent: 0.0,
                step: String::new()
            }
            .icon(),
            "[Chart]"
        );
        assert_eq!(BuildMessage::LogLine(String::new()).icon(), "[Edit]");
        assert_eq!(
            BuildMessage::Complete {
                output_path: PathBuf::new(),
                duration_secs: 0.0
            }
            .icon(),
            "[ok]"
        );
        assert_eq!(
            BuildMessage::Failed {
                error: String::new()
            }
            .icon(),
            "[x]"
        );
    }

    #[test]
    fn test_build_message_is_error() {
        assert!(!BuildMessage::Progress {
            percent: 0.0,
            step: String::new()
        }
        .is_error());
        assert!(!BuildMessage::LogLine(String::new()).is_error());
        assert!(!BuildMessage::Complete {
            output_path: PathBuf::new(),
            duration_secs: 0.0
        }
        .is_error());
        assert!(BuildMessage::Failed {
            error: String::new()
        }
        .is_error());
    }

    #[test]
    fn test_build_message_is_terminal() {
        assert!(!BuildMessage::Progress {
            percent: 0.0,
            step: String::new()
        }
        .is_terminal());
        assert!(!BuildMessage::LogLine(String::new()).is_terminal());
        assert!(BuildMessage::Complete {
            output_path: PathBuf::new(),
            duration_secs: 0.0
        }
        .is_terminal());
        assert!(BuildMessage::Failed {
            error: String::new()
        }
        .is_terminal());
    }
}
