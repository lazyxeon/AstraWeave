//! Professional-grade plan analysis CLI tool
//!
//! This tool provides production-quality plan analysis with comprehensive
//! quality metrics, bottleneck identification, and optimization suggestions.

use anyhow::{bail, Context, Result};
use astraweave_behavior::goap::*;
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "analyze-plan")]
#[command(version, about = "Analyze GOAP plan quality and suggest optimizations", long_about = None)]
struct Cli {
    /// Path to goal file
    #[arg(value_name = "GOAL_FILE")]
    goal_file: PathBuf,

    /// Path to world state file (optional, creates default if not provided)
    #[arg(short, long, value_name = "STATE_FILE")]
    state: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Output file path (stdout if not specified)
    #[arg(short, long, value_name = "OUTPUT_FILE")]
    output: Option<PathBuf>,

    /// Show optimization suggestions
    #[arg(long, default_value_t = true)]
    show_suggestions: bool,

    /// Show bottleneck analysis
    #[arg(long, default_value_t = true)]
    show_bottlenecks: bool,

    /// Show action-level breakdown
    #[arg(long)]
    show_breakdown: bool,

    /// Path to action history file for learning data
    #[arg(long, value_name = "HISTORY_FILE")]
    history: Option<PathBuf>,

    /// Compare with alternative goal (optional)
    #[arg(long, value_name = "COMPARE_GOAL")]
    compare: Option<PathBuf>,

    /// Maximum planning iterations (safety limit)
    #[arg(long, default_value_t = 10000)]
    max_iterations: usize,

    /// Minimum acceptable success probability (0.0-1.0)
    #[arg(long, value_name = "THRESHOLD")]
    min_success_rate: Option<f32>,

    /// Validate goal before planning
    #[arg(long, default_value_t = true)]
    validate: bool,

    /// Verbose output (show planning details)
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    /// Human-readable text output
    Text,
    /// Markdown format for documentation
    Markdown,
    /// JSON for programmatic consumption
    Json,
}

fn main() -> Result<()> {
    // Initialize professional logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Validate inputs
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

    if let Some(ref compare_file) = cli.compare {
        if !compare_file.exists() {
            bail!(
                "Comparison goal file '{}' does not exist.",
                compare_file.display()
            );
        }
    }

    // Execute analysis pipeline
    match run_analysis(&cli) {
        Ok(output) => {
            // Write output
            if let Some(ref output_path) = cli.output {
                fs::write(output_path, &output).with_context(|| {
                    format!("Failed to write output to '{}'", output_path.display())
                })?;

                if cli.verbose {
                    eprintln!("✓ Analysis written to '{}'", output_path.display());
                }
            } else {
                println!("{}", output);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("✗ Analysis failed: {}", e);
            eprintln!("\nTroubleshooting:");
            eprintln!(
                "  1. Verify goal file: cargo run --bin validate-goals -- {}",
                cli.goal_file.display()
            );
            eprintln!("  2. Check that goal is achievable from the current state");
            eprintln!(
                "  3. Try visualizing first: cargo run --bin visualize-plan -- {}",
                cli.goal_file.display()
            );
            eprintln!("  4. Use --verbose for detailed information");
            std::process::exit(1);
        }
    }
}

/// Main analysis pipeline with professional error handling
fn run_analysis(cli: &Cli) -> Result<String> {
    if cli.verbose {
        eprintln!("=== Plan Analysis Pipeline ===");
        eprintln!("Goal file: {}", cli.goal_file.display());
    }

    // Step 1: Load and optionally validate goal
    let goal_def = load_and_validate_goal(&cli.goal_file, cli.validate, cli.verbose)?;
    let goal = goal_def.to_goal();

    if cli.verbose {
        eprintln!(
            "✓ Goal loaded: '{}' (priority: {}, sub-goals: {})",
            goal.name,
            goal.priority,
            goal.sub_goals.len()
        );
    }

    // Step 2: Load or create world state
    let world_state = load_or_create_world_state(&cli.state, cli.verbose)?;

    // Step 3: Load action history
    let history = load_action_history(&cli.history, cli.verbose)?;

    // Step 4: Create planner and plan
    let (planner, plan) = create_and_plan(
        &world_state,
        &goal,
        &history,
        cli.max_iterations,
        cli.verbose,
    )?;

    if cli.verbose {
        eprintln!("✓ Plan generated ({} actions)", plan.len());
    }

    // Step 5: Analyze plan
    if cli.verbose {
        eprintln!("Analyzing plan quality...");
    }

    let metrics = PlanAnalyzer::analyze(&plan, planner.get_actions(), &history, &world_state);

    if cli.verbose {
        eprintln!("✓ Analysis complete");
        eprintln!("  Total cost: {:.2}", metrics.total_cost);
        eprintln!("  Total risk: {:.2}", metrics.total_risk);
        eprintln!(
            "  Success probability: {:.1}%",
            metrics.success_probability * 100.0
        );
        eprintln!("  Bottlenecks found: {}", metrics.bottlenecks.len());
    }

    // Step 6: Check against thresholds
    if let Some(min_success) = cli.min_success_rate {
        if metrics.success_probability < min_success {
            eprintln!(
                "⚠ WARNING: Plan success probability ({:.1}%) is below threshold ({:.1}%)",
                metrics.success_probability * 100.0,
                min_success * 100.0
            );
        }
    }

    // Step 7: Compare with alternative if requested
    let comparison = if let Some(ref compare_path) = cli.compare {
        if cli.verbose {
            eprintln!(
                "Comparing with alternative goal from '{}'...",
                compare_path.display()
            );
        }
        Some(compare_plans(
            compare_path,
            &world_state,
            &history,
            &planner,
            &metrics,
            cli.verbose,
        )?)
    } else {
        None
    };

    // Step 8: Generate output
    let output = match cli.format {
        OutputFormat::Text => generate_text_report(&metrics, comparison, cli),
        OutputFormat::Markdown => generate_markdown_report(&metrics, comparison, cli),
        OutputFormat::Json => generate_json_report(&metrics, comparison)?,
    };

    Ok(output)
}

/// Load and validate goal with professional error handling
fn load_and_validate_goal(path: &PathBuf, validate: bool, verbose: bool) -> Result<GoalDefinition> {
    let goal_def = GoalDefinition::load(path)
        .with_context(|| format!("Failed to load goal from '{}'", path.display()))?;

    if validate {
        if verbose {
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

        if verbose && !result.warnings.is_empty() {
            eprintln!("⚠ Warnings:");
            for warning in &result.warnings {
                eprintln!("  - {}", warning.message);
            }
        }
    }

    Ok(goal_def)
}

/// Load or create world state
fn load_or_create_world_state(state_path: &Option<PathBuf>, verbose: bool) -> Result<WorldState> {
    if let Some(path) = state_path {
        if verbose {
            eprintln!("Loading world state from '{}'...", path.display());
        }
        load_world_state(path)
    } else {
        if verbose {
            eprintln!("Creating default world state...");
        }
        Ok(create_default_world_state())
    }
}

/// Load action history
fn load_action_history(history_path: &Option<PathBuf>, verbose: bool) -> Result<ActionHistory> {
    if let Some(path) = history_path {
        if verbose {
            eprintln!("Loading action history from '{}'...", path.display());
        }

        match HistoryPersistence::load(path, PersistenceFormat::Json) {
            Ok(h) => {
                if verbose {
                    eprintln!(
                        "✓ History loaded ({} actions tracked)",
                        h.action_names().len()
                    );
                }
                Ok(h)
            }
            Err(e) => {
                eprintln!("⚠ Failed to load history: {}", e);
                eprintln!("  Using empty history instead.");
                Ok(ActionHistory::new())
            }
        }
    } else {
        Ok(ActionHistory::new())
    }
}

/// Create planner and generate plan
fn create_and_plan(
    world_state: &WorldState,
    goal: &Goal,
    history: &ActionHistory,
    max_iterations: usize,
    verbose: bool,
) -> Result<(AdvancedGOAP, Vec<String>)> {
    if verbose {
        eprintln!("Initializing planner...");
    }

    let mut planner = AdvancedGOAP::new();
    planner.set_max_iterations(max_iterations);
    register_all_actions(&mut planner);

    if verbose {
        eprintln!(
            "✓ Planner ready ({} actions available)",
            planner.action_count()
        );
        eprintln!("Planning...");
    }

    let plan = planner.plan(world_state, goal).ok_or_else(|| {
        anyhow::anyhow!(
            "No plan found.\n\
             Possible reasons:\n\
             - Goal is unreachable from current state\n\
             - Required actions are not available\n\
             - Planning exceeded iteration limit ({})",
            max_iterations
        )
    })?;

    Ok((planner, plan))
}

/// Compare with alternative goal
fn compare_plans(
    compare_path: &PathBuf,
    world_state: &WorldState,
    history: &ActionHistory,
    original_planner: &AdvancedGOAP,
    original_metrics: &PlanMetrics,
    verbose: bool,
) -> Result<ComparisonReport> {
    let compare_goal_def = GoalDefinition::load(compare_path)?;
    let compare_goal = compare_goal_def.to_goal();

    let compare_plan = original_planner
        .plan(world_state, &compare_goal)
        .ok_or_else(|| anyhow::anyhow!("Could not generate plan for comparison goal"))?;

    let compare_metrics = PlanAnalyzer::analyze(
        &compare_plan,
        original_planner.get_actions(),
        history,
        world_state,
    );

    if verbose {
        eprintln!("✓ Comparison complete");
    }

    Ok(PlanAnalyzer::compare(original_metrics, &compare_metrics))
}

/// Generate text report
fn generate_text_report(
    metrics: &PlanMetrics,
    comparison: Option<ComparisonReport>,
    cli: &Cli,
) -> String {
    let mut output = String::new();

    output.push_str("=== Plan Quality Analysis ===\n\n");

    // Core metrics
    output.push_str(&format!("Plan Overview:\n"));
    output.push_str(&format!("  Actions: {}\n", metrics.action_count));
    output.push_str(&format!("  Total Cost: {:.2}\n", metrics.total_cost));
    output.push_str(&format!("  Total Risk: {:.2}\n", metrics.total_risk));
    output.push_str(&format!(
        "  Estimated Duration: {:.1}s\n",
        metrics.estimated_duration
    ));
    output.push_str(&format!(
        "  Success Probability: {:.1}%\n\n",
        metrics.success_probability * 100.0
    ));

    // Quality assessment
    let quality = assess_quality(metrics);
    output.push_str(&format!("Overall Quality: {}\n\n", quality));

    // Bottlenecks
    if cli.show_bottlenecks && !metrics.bottlenecks.is_empty() {
        output.push_str("Bottlenecks Identified:\n");
        for (i, bottleneck) in metrics.bottlenecks.iter().enumerate().take(5) {
            output.push_str(&format!(
                "  {}. {} - {:?} (severity: {:.0}%)\n",
                i + 1,
                bottleneck.action_name,
                bottleneck.reason,
                bottleneck.severity * 100.0
            ));
        }
        output.push_str("\n");
    }

    // Action breakdown
    if cli.show_breakdown {
        output.push_str("Action Breakdown:\n");
        for (action_name, action_metrics) in &metrics.action_breakdown {
            output.push_str(&format!(
                "  {} - cost: {:.1}, risk: {:.2}, success: {:.0}%\n",
                action_name,
                action_metrics.cost,
                action_metrics.risk,
                action_metrics.success_rate * 100.0
            ));
        }
        output.push_str("\n");
    }

    // Optimization suggestions
    if cli.show_suggestions {
        let suggestions = PlanAnalyzer::suggest_optimizations(metrics);
        if !suggestions.is_empty() {
            output.push_str("Optimization Suggestions:\n");
            for (i, suggestion) in suggestions.iter().enumerate().take(10) {
                output.push_str(&format!(
                    "  {}. [{:?}] {}\n",
                    i + 1,
                    suggestion.priority,
                    suggestion.message
                ));
                if let Some(improvement) = suggestion.estimated_improvement {
                    output.push_str(&format!("     Estimated improvement: {:.1}\n", improvement));
                }
            }
        }
    }

    // Comparison
    if let Some(comp) = comparison {
        output.push_str("\n=== Plan Comparison ===\n\n");
        output.push_str(&format!("Cost Difference: {:+.2}\n", comp.cost_diff));
        output.push_str(&format!("Risk Difference: {:+.2}\n", comp.risk_diff));
        output.push_str(&format!(
            "Duration Difference: {:+.1}s\n",
            comp.duration_diff
        ));
        output.push_str(&format!(
            "Success Prob Difference: {:+.1}%\n\n",
            comp.success_prob_diff * 100.0
        ));

        match comp.better_plan {
            PlanComparison::Plan1Better => {
                output.push_str("Recommendation: Original plan is better\n")
            }
            PlanComparison::Plan2Better => {
                output.push_str("Recommendation: Alternative plan is better\n")
            }
            PlanComparison::Similar => {
                output.push_str("Recommendation: Plans are similar in quality\n")
            }
        }

        if !comp.recommendations.is_empty() {
            output.push_str("\nDetailed Recommendations:\n");
            for rec in &comp.recommendations {
                output.push_str(&format!("  - {}\n", rec));
            }
        }
    }

    output
}

/// Generate markdown report
fn generate_markdown_report(
    metrics: &PlanMetrics,
    comparison: Option<ComparisonReport>,
    cli: &Cli,
) -> String {
    let mut output = String::new();

    output.push_str("# Plan Quality Analysis\n\n");

    // Core metrics
    output.push_str("## Plan Overview\n\n");
    output.push_str(&format!("- **Actions**: {}\n", metrics.action_count));
    output.push_str(&format!("- **Total Cost**: {:.2}\n", metrics.total_cost));
    output.push_str(&format!("- **Total Risk**: {:.2}\n", metrics.total_risk));
    output.push_str(&format!(
        "- **Estimated Duration**: {:.1}s\n",
        metrics.estimated_duration
    ));
    output.push_str(&format!(
        "- **Success Probability**: {:.1}%\n\n",
        metrics.success_probability * 100.0
    ));

    let quality = assess_quality(metrics);
    output.push_str(&format!("**Overall Quality**: {}\n\n", quality));

    // Bottlenecks
    if cli.show_bottlenecks && !metrics.bottlenecks.is_empty() {
        output.push_str("## Bottlenecks\n\n");
        for (i, bottleneck) in metrics.bottlenecks.iter().enumerate().take(5) {
            output.push_str(&format!(
                "{}. **{}** - {:?} (severity: {:.0}%)\n",
                i + 1,
                bottleneck.action_name,
                bottleneck.reason,
                bottleneck.severity * 100.0
            ));
        }
        output.push_str("\n");
    }

    // Action breakdown table
    if cli.show_breakdown {
        output.push_str("## Action Breakdown\n\n");
        output.push_str("| Action | Cost | Risk | Success Rate |\n");
        output.push_str("|--------|------|------|-------------|\n");
        for (action_name, action_metrics) in &metrics.action_breakdown {
            output.push_str(&format!(
                "| {} | {:.1} | {:.2} | {:.0}% |\n",
                action_name,
                action_metrics.cost,
                action_metrics.risk,
                action_metrics.success_rate * 100.0
            ));
        }
        output.push_str("\n");
    }

    // Suggestions
    if cli.show_suggestions {
        let suggestions = PlanAnalyzer::suggest_optimizations(metrics);
        if !suggestions.is_empty() {
            output.push_str("## Optimization Suggestions\n\n");
            for (i, suggestion) in suggestions.iter().enumerate().take(10) {
                output.push_str(&format!(
                    "{}. **[{:?}]** {}\n",
                    i + 1,
                    suggestion.priority,
                    suggestion.message
                ));
                if let Some(improvement) = suggestion.estimated_improvement {
                    output.push_str(&format!(
                        "   - *Estimated improvement: {:.1}*\n",
                        improvement
                    ));
                }
            }
        }
    }

    // Comparison
    if let Some(comp) = comparison {
        output.push_str("\n## Plan Comparison\n\n");
        output.push_str("| Metric | Difference |\n");
        output.push_str("|--------|------------|\n");
        output.push_str(&format!("| Cost | {:+.2} |\n", comp.cost_diff));
        output.push_str(&format!("| Risk | {:+.2} |\n", comp.risk_diff));
        output.push_str(&format!("| Duration | {:+.1}s |\n", comp.duration_diff));
        output.push_str(&format!(
            "| Success Probability | {:+.1}% |\n\n",
            comp.success_prob_diff * 100.0
        ));

        match comp.better_plan {
            PlanComparison::Plan1Better => {
                output.push_str("**Recommendation**: Original plan is better\n\n")
            }
            PlanComparison::Plan2Better => {
                output.push_str("**Recommendation**: Alternative plan is better\n\n")
            }
            PlanComparison::Similar => {
                output.push_str("**Recommendation**: Plans are similar in quality\n\n")
            }
        }
    }

    output
}

/// Generate JSON report
fn generate_json_report(
    metrics: &PlanMetrics,
    comparison: Option<ComparisonReport>,
) -> Result<String> {
    let json = serde_json::json!({
        "overview": {
            "action_count": metrics.action_count,
            "total_cost": metrics.total_cost,
            "total_risk": metrics.total_risk,
            "estimated_duration": metrics.estimated_duration,
            "success_probability": metrics.success_probability,
        },
        "bottlenecks": metrics.bottlenecks.iter().map(|b| {
            serde_json::json!({
                "action": b.action_name,
                "reason": format!("{:?}", b.reason),
                "severity": b.severity,
            })
        }).collect::<Vec<_>>(),
        "action_breakdown": metrics.action_breakdown.iter().map(|(name, m)| {
            serde_json::json!({
                "action": name,
                "cost": m.cost,
                "risk": m.risk,
                "success_rate": m.success_rate,
                "avg_duration": m.avg_duration,
                "executions": m.executions,
            })
        }).collect::<Vec<_>>(),
        "suggestions": PlanAnalyzer::suggest_optimizations(metrics).iter().map(|s| {
            serde_json::json!({
                "priority": format!("{:?}", s.priority),
                "message": s.message,
                "estimated_improvement": s.estimated_improvement,
            })
        }).collect::<Vec<_>>(),
        "comparison": comparison.as_ref().map(|c| {
            serde_json::json!({
                "cost_diff": c.cost_diff,
                "risk_diff": c.risk_diff,
                "duration_diff": c.duration_diff,
                "success_prob_diff": c.success_prob_diff,
                "better_plan": format!("{:?}", c.better_plan),
                "recommendations": c.recommendations,
            })
        }),
    });

    Ok(serde_json::to_string_pretty(&json)?)
}

/// Assess overall quality
fn assess_quality(metrics: &PlanMetrics) -> &'static str {
    let score =
        metrics.success_probability * 10.0 - metrics.total_risk * 0.5 - metrics.total_cost * 0.1;

    if score >= 8.0 {
        "✓ Excellent"
    } else if score >= 6.0 {
        "✓ Good"
    } else if score >= 4.0 {
        "○ Fair"
    } else if score >= 2.0 {
        "⚠ Poor"
    } else {
        "✗ Critical"
    }
}

/// Load world state from JSON (reused from visualize-plan)
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

fn create_default_world_state() -> WorldState {
    let mut world = WorldState::new();
    world.set("my_hp", StateValue::Int(100));
    world.set("my_max_hp", StateValue::Int(100));
    world.set("my_ammo", StateValue::Int(30));
    world.set("my_max_ammo", StateValue::Int(30));
    world.set("my_x", StateValue::Int(0));
    world.set("my_y", StateValue::Int(0));
    world.set("my_in_cover", StateValue::Bool(false));
    world.set("in_combat", StateValue::Bool(false));
    world.set("enemies_visible", StateValue::Int(0));
    world.set("allies_visible", StateValue::Int(0));
    world
}
