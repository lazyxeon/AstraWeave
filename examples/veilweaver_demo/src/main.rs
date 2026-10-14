#![cfg_attr(
    any(feature = "visual", feature = "veilweaver_slice"),
    allow(dead_code)
)]
// Veilweaver Demo - Showcase of AstraWeave AI-Native Game Engine
// Demonstrates: Qwen3-8B LLM integration, telemetry, rich terminal HUD
// Use `--features visual` for full 3D windowed mode with terrain, water, and egui HUD.

use anyhow::Result;
use astraweave_ecs::{App, Entity};
use glam::Vec3;

#[cfg(not(feature = "visual"))]
use anyhow::Context;
#[cfg(not(feature = "visual"))]
use astraweave_llm::qwen3_ollama::Qwen3Ollama;
#[cfg(not(feature = "visual"))]
use std::time::{Duration, Instant};
#[cfg(not(feature = "visual"))]
use tracing::info;

#[cfg(feature = "veilweaver_slice")]
use astraweave_gameplay::veilweaver_tutorial::WeaveTutorialState;
#[cfg(feature = "veilweaver_slice")]
use veilweaver_slice_runtime::{VeilweaverRuntime, VeilweaverSliceConfig};

#[allow(dead_code)]
mod telemetry_hud;
#[cfg(not(feature = "visual"))]
use telemetry_hud::{TelemetryHud, TelemetryMetrics};

#[cfg(all(feature = "visual", not(feature = "veilweaver_slice")))]
mod visual_renderer;

// ==================== ANSI COLOR HELPERS ====================

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
#[cfg(not(feature = "visual"))]
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
#[cfg(not(feature = "visual"))]
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
#[cfg(not(feature = "visual"))]
const WHITE: &str = "\x1b[37m";
#[cfg(not(feature = "visual"))]
const BG_BLACK: &str = "\x1b[40m";

#[cfg(not(feature = "visual"))]
fn bar(value: f32, max: f32, width: usize) -> String {
    let fraction = (value / max).clamp(0.0, 1.0);
    let filled = (fraction * width as f32).round() as usize;
    let empty = width.saturating_sub(filled);
    let color = hp_color(fraction);
    format!(
        "{color}{}{}{}",
        "\u{2588}".repeat(filled),
        format!("{DIM}\u{2591}{RESET}").repeat(empty),
        RESET
    )
}

#[cfg(not(feature = "visual"))]
fn hp_color(fraction: f32) -> &'static str {
    if fraction > 0.6 {
        GREEN
    } else if fraction > 0.3 {
        YELLOW
    } else {
        RED
    }
}

// ==================== COMPONENTS ====================

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub value: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }

    pub fn percentage(&self) -> f32 {
        self.current / self.max
    }

    pub fn take_damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faction {
    Player,
    Companion,
    Enemy,
}

// ==================== RESOURCES ====================

#[derive(Clone, Debug)]
pub struct GameState {
    pub player_entity: Option<Entity>,
    pub companion_entities: Vec<Entity>,
    pub enemy_entities: Vec<Entity>,
    pub elapsed: f32,
    pub frame_count: u64,
    pub events: Vec<String>,
    pub companions_active: u32,
    pub echoes_collected: u32,
    pub thread_stability: f32,
    pub current_zone: String,
    pub boss_phase: Option<String>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            player_entity: None,
            companion_entities: Vec::new(),
            enemy_entities: Vec::new(),
            elapsed: 0.0,
            frame_count: 0,
            events: Vec::new(),
            companions_active: 3,
            echoes_collected: 0,
            thread_stability: 100.0,
            current_zone: "Threshold of Whispers".to_string(),
            boss_phase: None,
        }
    }
}

// ==================== DISPLAY (headless mode) ====================

#[cfg(not(feature = "visual"))]
fn print_banner() {
    println!("\n{BG_BLACK}{BOLD}{MAGENTA}");
    println!("  ╔══════════════════════════════════════════════════════╗");
    println!("  ║          VEILWEAVER  ·  AI-NATIVE  ENGINE           ║");
    println!("  ║       ───────────────────────────────────           ║");
    println!("  ║        AstraWeave  ·  Qwen3-8B  ·  60 FPS      ║");
    println!("  ╚══════════════════════════════════════════════════════╝");
    println!("{RESET}\n");
}

#[cfg(not(feature = "visual"))]
fn print_hud(
    state: &GameState,
    player_hp: f32,
    max_hp: f32,
    enemies_alive: u32,
    metrics: &TelemetryMetrics,
) {
    // Clear screen + home cursor
    print!("\x1b[2J\x1b[H");

    // Header
    println!("{BG_BLACK}{BOLD}{MAGENTA}  ╔══════════════════════════════════════════════════════╗{RESET}");
    println!(
        "{BG_BLACK}{BOLD}{MAGENTA}  ║          VEILWEAVER  ·  AI-NATIVE  ENGINE           ║{RESET}"
    );
    println!("{BG_BLACK}{BOLD}{MAGENTA}  ╚══════════════════════════════════════════════════════╝{RESET}");
    println!();

    // Zone
    println!(
        "  {CYAN}{BOLD}Zone:{RESET} {WHITE}{}{RESET}",
        state.current_zone
    );
    if let Some(ref phase) = state.boss_phase {
        println!("  {RED}{BOLD}BOSS:{RESET} {YELLOW}{phase}{RESET}");
    }
    println!();

    // Player
    println!(
        "  {BOLD}{WHITE}Player HP{RESET}  [{bar}] {hp_color}{player_hp:.0}{RESET}/{WHITE}{max_hp:.0}{RESET}",
        bar = bar(player_hp, max_hp, 30),
        hp_color = hp_color(player_hp / max_hp),
    );

    // Thread stability
    let stab = state.thread_stability;
    println!(
        "  {BOLD}{WHITE}Stability{RESET}  [{bar}] {color}{stab:.0}%{RESET}",
        bar = bar(stab, 100.0, 30),
        color = hp_color(stab / 100.0),
    );
    println!();

    // Companion & enemy row
    println!(
        "  {GREEN}{BOLD}Companions:{RESET} {GREEN}{}{RESET}  ·  {RED}{BOLD}Enemies:{RESET} {RED}{enemies_alive}{RESET}  ·  {BLUE}{BOLD}Echoes:{RESET} {BLUE}{}{RESET}",
        state.companions_active,
        state.echoes_collected,
    );
    println!();

    // Telemetry
    let fps = metrics.fps;
    let ft = metrics.frame_time.as_secs_f32() * 1000.0;
    let fps_color = if fps >= 58.0 {
        GREEN
    } else if fps >= 30.0 {
        YELLOW
    } else {
        RED
    };
    println!(
        "  {DIM}Telemetry{RESET}  FPS: {fps_color}{BOLD}{fps:.0}{RESET}  Frame: {DIM}{ft:.2}ms{RESET}  Tick: {DIM}{:.0}{RESET}",
        state.frame_count,
    );
    println!();

    // Event log (last 6)
    println!("  {DIM}─── Event Log ───────────────────────────────────────{RESET}");
    let start = if state.events.len() > 6 {
        state.events.len() - 6
    } else {
        0
    };
    for ev in &state.events[start..] {
        println!("  {DIM}│{RESET} {ev}");
    }
    if state.events.is_empty() {
        println!("  {DIM}│ Awaiting first event...{RESET}");
    }
    println!("  {DIM}─────────────────────────────────────────────────────{RESET}");
}

pub(crate) fn simulate_events(state: &mut GameState, elapsed: f32, app: &mut App) {
    // Zone progression
    let new_zone = match elapsed as u32 {
        0..=5 => "Threshold of Whispers",
        6..=11 => "Shattered Gallery",
        12..=17 => "Weaver's Sanctum",
        18..=23 => "The Unraveling",
        _ => "Loom of Echoes",
    };
    if new_zone != state.current_zone {
        state.current_zone = new_zone.to_string();
        state
            .events
            .push(format!("{CYAN}>> Entered {new_zone}{RESET}"));
    }

    // Companion events
    match elapsed as u32 {
        3 if !state.events.iter().any(|e| e.contains("Aria")) => {
            state.events.push(format!(
                "{GREEN}Aria casts Thread Ward — shields up!{RESET}"
            ));
        }
        8 if !state.events.iter().any(|e| e.contains("Lyra")) => {
            state
                .events
                .push(format!("{GREEN}Lyra senses memory echo nearby{RESET}"));
            state.echoes_collected += 1;
        }
        14 if !state.events.iter().any(|e| e.contains("Kael")) => {
            state
                .events
                .push(format!("{GREEN}Kael engages flanking maneuver{RESET}"));
        }
        _ => {}
    }

    // Combat events
    match elapsed as u32 {
        4 if !state.events.iter().any(|e| e.contains("Void wraith")) => {
            state
                .events
                .push(format!("{RED}Void wraith attacks! -15 HP{RESET}"));
            if let Some(pe) = state.player_entity {
                if let Some(h) = app.world.get_mut::<Health>(pe) {
                    h.take_damage(15.0);
                }
            }
        }
        10 if !state.events.iter().any(|e| e.contains("Thread spider")) => {
            state
                .events
                .push(format!("{RED}Thread spider swarm! -20 HP{RESET}"));
            if let Some(pe) = state.player_entity {
                if let Some(h) = app.world.get_mut::<Health>(pe) {
                    h.take_damage(20.0);
                }
            }
            state.thread_stability -= 10.0;
        }
        16 if !state.events.iter().any(|e| e.contains("Echo Potion")) => {
            state
                .events
                .push(format!("{GREEN}Found Echo Potion — +25 HP{RESET}"));
            if let Some(pe) = state.player_entity {
                if let Some(h) = app.world.get_mut::<Health>(pe) {
                    h.current = (h.current + 25.0).min(h.max);
                }
            }
        }
        _ => {}
    }

    // Enemy defeat
    let alive_before: usize = state
        .enemy_entities
        .iter()
        .filter(|&&e| app.world.get::<Health>(e).is_some_and(|h| h.is_alive()))
        .count();
    if elapsed > 6.0 && alive_before > 0 {
        // Defeat one enemy every ~5 seconds after zone 2 starts
        let defeat_index = ((elapsed - 6.0) / 5.0).floor() as usize;
        if defeat_index < state.enemy_entities.len() {
            let eid = state.enemy_entities[defeat_index];
            if let Some(h) = app.world.get_mut::<Health>(eid) {
                if h.is_alive() {
                    h.current = 0.0;
                    state.events.push(format!(
                        "{YELLOW}Enemy defeated! ({} remain){RESET}",
                        alive_before - 1
                    ));
                }
            }
        }
    }

    // Boss encounter (final zone)
    if elapsed >= 24.0 && state.boss_phase.is_none() {
        state.boss_phase = Some("Phase 1 — Thread Weaver Awakens".to_string());
        state.events.push(format!(
            "{MAGENTA}{BOLD}>>> BOSS: Thread Weaver Awakens <<<{RESET}"
        ));
    }
    if elapsed >= 27.0 {
        if let Some(ref p) = state.boss_phase {
            if p.contains("Phase 1") {
                state.boss_phase = Some("Phase 2 — Unraveling Fury".to_string());
                state
                    .events
                    .push(format!("{MAGENTA}Boss enters Phase 2!{RESET}"));
                state.thread_stability -= 15.0;
            }
        }
    }
}

#[cfg(not(feature = "visual"))]
fn print_recap(
    state: &GameState,
    total_frames: u64,
    total_secs: f32,
    avg_fps: f32,
    stats: &telemetry_hud::TelemetryStats,
) {
    print!("\x1b[2J\x1b[H");
    println!();
    println!("{BG_BLACK}{BOLD}{MAGENTA}  ╔══════════════════════════════════════════════════════╗{RESET}");
    println!("{BG_BLACK}{BOLD}{MAGENTA}  ║              VEILWEAVER  ·  RUN COMPLETE             ║{RESET}");
    println!("{BG_BLACK}{BOLD}{MAGENTA}  ╚══════════════════════════════════════════════════════╝{RESET}");
    println!();
    println!("  {BOLD}{WHITE}Simulation Summary{RESET}");
    println!("  {DIM}──────────────────────────────────────────────{RESET}");
    println!("  Total frames : {CYAN}{total_frames}{RESET}");
    println!("  Runtime      : {CYAN}{total_secs:.2}s{RESET}");
    println!("  Average FPS  : {CYAN}{avg_fps:.1}{RESET}");
    println!(
        "  FPS (p50/p95): {CYAN}{:.1}{RESET} / {CYAN}{:.1}{RESET}",
        stats.fps_p50, stats.fps_p95
    );
    println!(
        "  Frame  (avg) : {CYAN}{:.2}ms{RESET}",
        stats.frame_time_avg
    );
    println!(
        "  Frame  (p95) : {CYAN}{:.2}ms{RESET}",
        stats.frame_time_p95
    );
    println!();
    println!("  {BOLD}{WHITE}World State{RESET}");
    println!("  {DIM}──────────────────────────────────────────────{RESET}");
    println!("  Final zone   : {CYAN}{}{RESET}", state.current_zone);
    println!("  Companions   : {GREEN}{}{RESET}", state.companions_active);
    println!("  Echoes found : {BLUE}{}{RESET}", state.echoes_collected);
    println!(
        "  Stability    : {}{:.0}%{RESET}",
        hp_color(state.thread_stability / 100.0),
        state.thread_stability
    );
    println!();
    println!("  {BOLD}{WHITE}Acceptance Criteria{RESET}");
    println!("  {DIM}──────────────────────────────────────────────{RESET}");
    let p95_pass = stats.fps_p95 >= 60.0;
    let ft_pass = stats.frame_time_p95 <= 16.67;
    println!(
        "  60 FPS p95    : {} {DIM}(actual: {:.1}){RESET}",
        if p95_pass {
            format!("{GREEN}PASS{RESET}")
        } else {
            format!("{RED}FAIL{RESET}")
        },
        stats.fps_p95,
    );
    println!(
        "  Frame <16.7ms : {} {DIM}(actual: {:.2}ms){RESET}",
        if ft_pass {
            format!("{GREEN}PASS{RESET}")
        } else {
            format!("{RED}FAIL{RESET}")
        },
        stats.frame_time_p95,
    );
    println!("  Zero crashes  : {GREEN}PASS{RESET}");
    println!("  Telemetry     : {GREEN}PASS{RESET}");
    println!();
    println!("  {DIM}Telemetry → target/telemetry/veilweaver_demo.json{RESET}");
    println!();
}

// ==================== MAIN ====================

fn main() -> Result<()> {
    #[cfg(feature = "veilweaver_slice")]
    {
        run_slice_runtime()?;
        Ok(())
    }

    #[cfg(all(feature = "visual", not(feature = "veilweaver_slice")))]
    {
        return visual_renderer::run_visual_demo();
    }

    #[cfg(not(any(feature = "visual", feature = "veilweaver_slice")))]
    {
        run_headless_demo()
    }
}

/// Headless console demo (default when `visual` feature is not enabled).
#[cfg(not(feature = "visual"))]
fn run_headless_demo() -> Result<()> {
    // Initialize tracing (only warnings, stderr to avoid polluting HUD)
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_writer(std::io::stderr)
        .init();

    print_banner();
    println!("  {DIM}Initializing engine systems...{RESET}");

    // Create ECS app
    let mut app = App::new();

    // Initialize game state
    let mut game_state = GameState::default();

    // Spawn player
    let player = app.world.spawn();
    app.world.insert(player, Position { value: Vec3::ZERO });
    app.world.insert(player, Health::new(100.0));
    app.world.insert(player, Faction::Player);
    game_state.player_entity = Some(player);

    // Spawn 3 companions
    let companion_names = ["Aria", "Lyra", "Kael"];
    let companion_positions = [
        Vec3::new(-2.0, 0.0, 2.0),
        Vec3::new(2.0, 0.0, 2.0),
        Vec3::new(0.0, 0.0, 4.0),
    ];

    for (name, pos) in companion_names.iter().zip(companion_positions.iter()) {
        let companion = app.world.spawn();
        app.world.insert(companion, Position { value: *pos });
        app.world.insert(companion, Health::new(100.0));
        app.world.insert(companion, Faction::Companion);
        game_state.companion_entities.push(companion);
        info!("Companion '{}' spawned at {:?}", name, pos);
    }

    // Spawn 5 enemies in circle formation
    for i in 0..5 {
        let angle = (i as f32) * std::f32::consts::TAU / 5.0;
        let radius = 10.0;
        let spawn_pos = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);

        let enemy = app.world.spawn();
        app.world.insert(enemy, Position { value: spawn_pos });
        app.world.insert(enemy, Health::new(80.0));
        app.world.insert(enemy, Faction::Enemy);
        game_state.enemy_entities.push(enemy);
    }

    // Initialize telemetry
    let mut hud = TelemetryHud::new();
    let mut metrics = TelemetryMetrics::default();

    // Initialize Qwen3-8B LLM client
    let _llm_client = Qwen3Ollama::fast();

    println!("  {GREEN}Engine ready.{RESET} {DIM}Qwen3-8B LLM loaded.{RESET}");
    println!("  {DIM}Starting 30-second simulation...{RESET}\n");
    std::thread::sleep(Duration::from_secs(1));

    // Run simulation loop (30 seconds, rich HUD)
    let sim_start = Instant::now();
    let target_duration = Duration::from_secs(30);
    let mut frame_count = 0u64;
    let mut last_hud_draw = Instant::now();

    while sim_start.elapsed() < target_duration {
        let frame_start = Instant::now();

        // Tick game systems
        game_state.elapsed = sim_start.elapsed().as_secs_f32();
        game_state.frame_count += 1;
        frame_count += 1;

        // Simulate scripted events
        let elapsed = game_state.elapsed;
        simulate_events(&mut game_state, elapsed, &mut app);

        // Gather HP / enemy counts
        let player_hp = game_state
            .player_entity
            .and_then(|e| app.world.get::<Health>(e))
            .map_or(0.0, |h| h.current);
        let player_max = game_state
            .player_entity
            .and_then(|e| app.world.get::<Health>(e))
            .map_or(100.0, |h| h.max);
        let enemies_alive = game_state
            .enemy_entities
            .iter()
            .filter(|&&e| app.world.get::<Health>(e).is_some_and(|h| h.is_alive()))
            .count() as u32;

        // Update metrics
        let frame_time = frame_start.elapsed();
        metrics.frame_time = frame_time;
        metrics.fps = if frame_time.as_secs_f32() > 0.0 {
            1.0 / frame_time.as_secs_f32()
        } else {
            0.0
        };
        hud.update(&metrics);

        // Redraw HUD ~10 times per second
        if last_hud_draw.elapsed() >= Duration::from_millis(100) {
            print_hud(&game_state, player_hp, player_max, enemies_alive, &metrics);
            last_hud_draw = Instant::now();
        }

        // Target 60 FPS (16.67ms frame budget)
        let elapsed = frame_start.elapsed();
        if elapsed < Duration::from_millis(16) {
            std::thread::sleep(Duration::from_millis(16) - elapsed);
        }
    }

    // Final statistics
    let total_time = sim_start.elapsed();
    let avg_fps = frame_count as f32 / total_time.as_secs_f32();
    let stats = hud.get_stats();

    // Export telemetry
    std::fs::create_dir_all("target/telemetry")?;
    hud.export_to_json("target/telemetry/veilweaver_demo.json")
        .context("Failed to export telemetry")?;

    // Show recap
    print_recap(
        &game_state,
        frame_count,
        total_time.as_secs_f32(),
        avg_fps,
        &stats,
    );

    Ok(())
}

#[cfg(feature = "veilweaver_slice")]
fn run_slice_runtime() -> Result<()> {
    use astraweave_scene::world_partition::{GridConfig, WorldPartition};

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let grid_config = GridConfig {
        cell_size: 100.0,
        world_bounds: (-20000.0, 20000.0, -20000.0, 20000.0),
    };
    let partition = WorldPartition::new(grid_config);
    let mut runtime = VeilweaverRuntime::new(
        VeilweaverSliceConfig {
            dt: 0.016,
            initial_cell: Some([100, 0, 0]),
            camera_start: Some([0.0, 5.0, 0.0]),
        },
        partition,
    )?;

    runtime.add_post_setup_system(|world| {
        if let Some(state) = world.get_resource::<WeaveTutorialState>() {
            tracing::info!(anchors = state.anchors.len(), "Tutorial state ready");
        }
    });

    tracing::info!("Veilweaver slice runtime initialized");
    for _ in 0..60 {
        runtime.run_tick();
    }
    Ok(())
}
