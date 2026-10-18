//! Professional-grade plan visualization CLI tool
//!
//! This tool provides production-quality plan visualization with comprehensive
//! error handling, multiple output formats, and extensive configuration options.

use anyhow::{bail, Context, Result};
use astraweave_behavior::goap::*;
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "visualize-plan")]
#[command(version, about = "Visualize GOAP plans with multiple output formats", long_about = None)]
struct Cli {
    /// Path to goal file
    #[arg(value_name = "GOAL_FILE")]
    goal_file: PathBuf,

    /// Path to world state file (optional, creates default if not provided)
    #[arg(short, long, value_name = "STATE_FILE")]
    state: Option<PathBuf>,

    /// Visualization format
    #[arg(short, long, value_enum, default_value_t = Format::AsciiTree)]
    format: Format,

    /// Output file path (stdout if not specified)
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output: Option<PathBuf>,

    /// Show action costs in visualization
    #[arg(long, default_value_t = true)]
    show_costs: bool,

    /// Show action risks in visualization
    #[arg(long, default_value_t = true)]
    show_risks: bool,

    /// Show state changes in visualization
    #[arg(long)]
    show_state_changes: bool,

    /// Path to action history file for learning data
    #[arg(long, value_name = "HISTORY_FILE")]
    history: Option<PathBuf>,

    /// Maximum planning iterations (safety limit)
    #[arg(long, default_value_t = 10000)]
    max_iterations: usize,

    /// Validate goal before planning
    #[arg(long, default_value_t = true)]
    validate: bool,

    /// Verbose output (show planning details)
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Format {
    /// ASCII tree with Unicode box drawing
    AsciiTree,
    /// ASCII timeline showing execution sequence
    AsciiTimeline,
    /// DOT format for GraphViz
    Dot,
    /// Simple text list
    Text,
    /// JSON for programmatic consumption
    Json,
}

fn main() -> Result<()> {
    // Initialize with professional logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Validate inputs with comprehensive error messages
    if !cli.goal_file.exists() {
        bail!(
            "Goal file '{}' does not exist.\n\
             Hint: Check the path or use 'validate-goals' to verify the file.",
            cli.goal_file.display()
        );
    }

    if let Some(ref state_file) = cli.state {
        if !state_file.exists() {
            bail!(
                "State file '{}' does not exist.\n\
                 Hint: Omit --state flag to use default world state.",
                state_file.display()
            );
        }
    }

    // Execute visualization pipeline
    match run_visualization(&cli) {
        Ok(output) => {
            // Write output to file or stdout
            if let Some(ref output_path) = cli.output {
                fs::write(output_path, &output).with_context(|| {
                    format!("Failed to write output to '{}'", output_path.display())
                })?;

                if cli.verbose {
                    eprintln!("✓ Visualization written to '{}'", output_path.display());
                }
            } else {
                println!("{}", output);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("✗ Visualization failed: {}", e);
            eprintln!("\nTroubleshooting:");
            eprintln!(
                "  1. Verify goal file is valid: cargo run --bin validate-goals -- {}",
                cli.goal_file.display()
            );
            eprintln!("  2. Check that goal is achievable from the current state");
            eprintln!("  3. Try reducing complexity or adding intermediate goals");
            eprintln!("  4. Use --verbose flag for detailed planning information");
            std::process::exit(1);
        }
    }
}

/// Main visualization pipeline with professional error handling
fn run_visualization(cli: &Cli) -> Result<String> {
    if cli.verbose {
        eprintln!("=== Plan Visualization Pipeline ===");
        eprintln!("Goal file: {}", cli.goal_file.display());
    }

    // Step 1: Load and optionally validate goal
    let goal_def = GoalDefinition::load(&cli.goal_file)
        .with_context(|| format!("Failed to load goal from '{}'", cli.goal_file.display()))?;

    if cli.validate {
        if cli.verbose {
            eprintln!("Validating goal...");
        }

        let validator = GoalValidator::new();
        let result = validator.validate(&goal_def);

        if !result.is_valid() {
            eprintln!("⚠ Goal validation found errors:");
            for error in &result.errors {
                eprintln!("  - {}", error.message);
            }
            bail!("Goal validation failed. Fix errors or use --no-validate to skip validation.");
        }

        if cli.verbose && !result.warnings.is_empty() {
            eprintln!("⚠ Warnings:");
            for warning in &result.warnings {
                eprintln!("  - {}", warning.message);
            }
        }
    }

    let goal = goal_def.to_goal();

    if cli.verbose {
        eprintln!("✓ Goal loaded: '{}'", goal.name);
        eprintln!("  Priority: {}", goal.priority);
        eprintln!("  Sub-goals: {}", goal.sub_goals.len());
        eprintln!("  Total goal count: {}", goal.total_goal_count());
    }

    // Step 2: Load or create world state
    let world_state = if let Some(ref state_path) = cli.state {
        if cli.verbose {
            eprintln!("Loading world state from '{}'...", state_path.display());
        }
        load_world_state(state_path)?
    } else {
        if cli.verbose {
            eprintln!("Creating default world state...");
        }
        create_default_world_state()
    };

    if cli.verbose {
        eprintln!(
            "✓ World state ready ({} variables)",
            world_state.iter().count()
        );
    }

    // Step 3: Load action history if provided
    let history = if let Some(ref history_path) = cli.history {
        if cli.verbose {
            eprintln!(
                "Loading action history from '{}'...",
                history_path.display()
            );
        }

        match HistoryPersistence::load(history_path, PersistenceFormat::Json) {
            Ok(h) => {
                if cli.verbose {
                    eprintln!(
                        "✓ History loaded ({} actions tracked)",
                        h.action_names().len()
                    );
                }
                h
            }
            Err(e) => {
                eprintln!("⚠ Failed to load history: {}", e);
                eprintln!("  Using empty history instead.");
                ActionHistory::new()
            }
        }
    } else {
        ActionHistory::new()
    };

    // Step 4: Create planner and register actions
    if cli.verbose {
        eprintln!("Initializing planner...");
    }

    let mut planner = AdvancedGOAP::new();
    planner.set_max_iterations(cli.max_iterations);
    register_all_actions(&mut planner);

    if cli.verbose {
        eprintln!(
            "✓ Planner ready ({} actions available)",
            planner.action_count()
        );
    }

    // Step 5: Generate plan
    if cli.verbose {
        eprintln!("Planning...");
    }

    let plan = planner.plan(&world_state, &goal).ok_or_else(|| {
        anyhow::anyhow!(
            "No plan found.\n\
             Possible reasons:\n\
             - Goal is unreachable from current state\n\
             - Required actions are not available\n\
             - Planning exceeded iteration limit ({})\n\
             - Preconditions are never satisfied",
            cli.max_iterations
        )
    })?;

    if cli.verbose {
        eprintln!("✓ Plan generated ({} actions)", plan.len());
        for (i, action) in plan.iter().enumerate() {
            eprintln!("  {}. {}", i + 1, action);
        }
    }

    // Step 6: Visualize plan
    if cli.verbose {
        eprintln!("Generating visualization ({:?} format)...", cli.format);
    }

    let viz_format = match cli.format {
        Format::AsciiTree => VisualizationFormat::AsciiTree,
        Format::AsciiTimeline => VisualizationFormat::AsciiTimeline,
        Format::Dot => VisualizationFormat::Dot,
        Format::Text => VisualizationFormat::Text,
        Format::Json => VisualizationFormat::Json,
    };

    let visualizer = PlanVisualizer::new(viz_format)
        .with_costs(cli.show_costs)
        .with_risks(cli.show_risks)
        .with_state_changes(cli.show_state_changes);

    let output = visualizer.visualize_plan(&plan, planner.get_actions(), &history, &world_state);

    if cli.verbose {
        eprintln!("✓ Visualization complete");
    }

    Ok(output)
}

/// Load world state from JSON file
fn load_world_state(path: &PathBuf) -> Result<WorldState> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read state file '{}'", path.display()))?;

    let json: serde_json::Value =
        serde_json::from_str(&content).with_context(|| "Failed to parse state file as JSON")?;

    let mut world_state = WorldState::new();

    if let Some(obj) = json.as_object() {
        for (key, value) in obj {
            let state_value = json_to_state_value(value)
                .with_context(|| format!("Failed to convert JSON value for key '{}'", key))?;
            world_state.set(key, state_value);
        }
    } else {
        bail!("State file must contain a JSON object");
    }

    Ok(world_state)
}

/// Convert JSON value to StateValue with comprehensive type handling
fn json_to_state_value(value: &serde_json::Value) -> Result<StateValue> {
    match value {
        serde_json::Value::Bool(b) => Ok(StateValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(StateValue::Int(i as i32))
            } else if let Some(f) = n.as_f64() {
                Ok(StateValue::Float(OrderedFloat(f as f32)))
            } else {
                bail!("Invalid number format")
            }
        }
        serde_json::Value::String(s) => Ok(StateValue::String(s.clone())),
        serde_json::Value::Object(obj) => {
            // Support range and approx formats
            if let (Some(min), Some(max)) = (obj.get("min"), obj.get("max")) {
                if let (Some(min_i), Some(max_i)) = (min.as_i64(), max.as_i64()) {
                    return Ok(StateValue::IntRange(min_i as i32, max_i as i32));
                }
            }
            if let (Some(value), Some(tolerance)) = (obj.get("value"), obj.get("tolerance")) {
                if let (Some(v), Some(t)) = (value.as_f64(), tolerance.as_f64()) {
                    return Ok(StateValue::FloatApprox(v as f32, t as f32));
                }
            }
            bail!("Unsupported object format in state value")
        }
        _ => bail!("Unsupported JSON type for state value"),
    }
}

/// Create a default world state with sensible starting values
fn create_default_world_state() -> WorldState {
    let mut world = WorldState::new();

    // Player/Agent state
    world.set("my_hp", StateValue::Int(100));
    world.set("my_max_hp", StateValue::Int(100));
    world.set("my_ammo", StateValue::Int(30));
    world.set("my_max_ammo", StateValue::Int(30));
    world.set("my_x", StateValue::Int(0));
    world.set("my_y", StateValue::Int(0));
    world.set("my_in_cover", StateValue::Bool(false));

    // Combat state
    world.set("in_combat", StateValue::Bool(false));
    world.set("enemies_visible", StateValue::Int(0));
    world.set("allies_visible", StateValue::Int(0));

    // Environment
    world.set("time_of_day", StateValue::String("day".to_string()));
    world.set("weather", StateValue::String("clear".to_string()));

    world
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_state_value_bool() {
        let json = serde_json::json!(true);
        let result = json_to_state_value(&json).unwrap();
        assert_eq!(result, StateValue::Bool(true));
    }

    #[test]
    fn test_json_to_state_value_int() {
        let json = serde_json::json!(42);
        let result = json_to_state_value(&json).unwrap();
        assert_eq!(result, StateValue::Int(42));
    }

    #[test]
    fn test_json_to_state_value_float() {
        let json = serde_json::json!(3.14);
        let result = json_to_state_value(&json).unwrap();
        match result {
            StateValue::Float(f) => assert!((f.0 - 3.14).abs() < 0.01),
            _ => panic!("Expected Float"),
        }
    }

    #[test]
    fn test_json_to_state_value_string() {
        let json = serde_json::json!("test");
        let result = json_to_state_value(&json).unwrap();
        assert_eq!(result, StateValue::String("test".to_string()));
    }

    #[test]
    fn test_json_to_state_value_int_range() {
        let json = serde_json::json!({"min": 10, "max": 20});
        let result = json_to_state_value(&json).unwrap();
        assert_eq!(result, StateValue::IntRange(10, 20));
    }

    #[test]
    fn test_json_to_state_value_float_approx() {
        let json = serde_json::json!({"value": 5.5, "tolerance": 0.1});
        let result = json_to_state_value(&json).unwrap();
        match result {
            StateValue::FloatApprox(v, t) => {
                assert!((v - 5.5).abs() < 0.01);
                assert!((t - 0.1).abs() < 0.01);
            }
            _ => panic!("Expected FloatApprox"),
        }
    }

    #[test]
    fn test_default_world_state() {
        let world = create_default_world_state();
        assert_eq!(world.get("my_hp"), Some(&StateValue::Int(100)));
        assert_eq!(world.get("my_ammo"), Some(&StateValue::Int(30)));
        assert_eq!(world.get("in_combat"), Some(&StateValue::Bool(false)));
    }
}
