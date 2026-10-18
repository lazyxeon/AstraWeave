use anyhow::{Context, Result};
use astraweave_behavior::goap::*;
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "validate-goals")]
#[command(about = "Validate GOAP goal files", long_about = None)]
struct Cli {
    /// Path to goal file or directory
    #[arg(value_name = "PATH")]
    path: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Show warnings in addition to errors
    #[arg(short, long)]
    warnings: bool,

    /// Show info messages
    #[arg(short, long)]
    info: bool,

    /// Exit with error code if validation fails
    #[arg(long)]
    strict: bool,

    /// Validate all .toml files in directory recursively
    #[arg(short, long)]
    recursive: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    /// Human-readable text output
    Text,
    /// JSON output for programmatic consumption
    Json,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let validator = GoalValidator::new();

    if cli.path.is_file() {
        validate_file(&cli.path, &validator, &cli)?;
    } else if cli.path.is_dir() {
        validate_directory(&cli.path, &validator, &cli)?;
    } else {
        anyhow::bail!("Path '{}' does not exist", cli.path.display());
    }

    Ok(())
}

fn validate_file(path: &PathBuf, validator: &GoalValidator, cli: &Cli) -> Result<()> {
    let goal_def = GoalDefinition::load(path)
        .with_context(|| format!("Failed to load goal from '{}'", path.display()))?;

    let result = validator.validate(&goal_def);

    match cli.format {
        OutputFormat::Text => print_text_result(path, &result, cli),
        OutputFormat::Json => print_json_result(path, &result, cli)?,
    }

    if cli.strict && !result.is_valid() {
        std::process::exit(1);
    }

    Ok(())
}

fn validate_directory(dir: &PathBuf, validator: &GoalValidator, cli: &Cli) -> Result<()> {
    let mut files = Vec::new();
    collect_toml_files(dir, &mut files, cli.recursive)?;

    if files.is_empty() {
        println!("No .toml files found in '{}'", dir.display());
        return Ok(());
    }

    let mut total_valid = 0;
    let mut total_invalid = 0;
    let mut results = Vec::new();

    for file in &files {
        match GoalDefinition::load(file) {
            Ok(goal_def) => {
                let result = validator.validate(&goal_def);
                if result.is_valid() {
                    total_valid += 1;
                } else {
                    total_invalid += 1;
                }
                results.push((file.clone(), result));
            }
            Err(e) => {
                eprintln!("Error loading '{}': {}", file.display(), e);
                total_invalid += 1;
            }
        }
    }

    match cli.format {
        OutputFormat::Text => {
            println!("\n=== Validation Summary ===");
            println!("Total files: {}", files.len());
            println!("Valid: {} ✓", total_valid);
            println!("Invalid: {} ✗", total_invalid);
            println!();

            for (path, result) in &results {
                print_text_result(path, result, cli);
            }
        }
        OutputFormat::Json => {
            print_json_directory_results(&results, cli)?;
        }
    }

    if cli.strict && total_invalid > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn collect_toml_files(dir: &PathBuf, files: &mut Vec<PathBuf>, recursive: bool) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
            files.push(path);
        } else if path.is_dir() && recursive {
            collect_toml_files(&path, files, recursive)?;
        }
    }

    Ok(())
}

fn print_text_result(path: &PathBuf, result: &ValidationResult, cli: &Cli) {
    if result.is_valid() {
        println!("✓ {} is valid", path.display());

        if cli.warnings && !result.warnings.is_empty() {
            println!("  Warnings:");
            for warning in &result.warnings {
                println!("    - {}", warning.message);
            }
        }

        if cli.info && !result.info.is_empty() {
            println!("  Info:");
            for info in &result.info {
                println!("    - {}", info.message);
            }
        }
    } else {
        println!("✗ {} has errors", path.display());

        for error in &result.errors {
            print!("  ERROR: {}", error.message);
            if let Some(field) = &error.field {
                print!(" (in field '{}')", field);
            }
            println!();

            if let Some(suggestion) = &error.suggestion {
                println!("    Suggestion: {}", suggestion);
            }
        }

        if cli.warnings && !result.warnings.is_empty() {
            println!("  Warnings:");
            for warning in &result.warnings {
                println!("    - {}", warning.message);
            }
        }
    }
    println!();
}

fn print_json_result(path: &PathBuf, result: &ValidationResult, _cli: &Cli) -> Result<()> {
    let output = serde_json::json!({
        "file": path.to_string_lossy(),
        "valid": result.is_valid(),
        "errors": result.errors.iter().map(|e| {
            serde_json::json!({
                "message": e.message,
                "field": e.field,
                "suggestion": e.suggestion,
                "severity": format!("{:?}", e.severity),
            })
        }).collect::<Vec<_>>(),
        "warnings": result.warnings.iter().map(|w| w.message.clone()).collect::<Vec<_>>(),
        "info": result.info.iter().map(|i| i.message.clone()).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn print_json_directory_results(results: &[(PathBuf, ValidationResult)], _cli: &Cli) -> Result<()> {
    let total_files = results.len();
    let valid_count = results.iter().filter(|(_, r)| r.is_valid()).count();
    let invalid_count = total_files - valid_count;

    let output = serde_json::json!({
        "summary": {
            "total_files": total_files,
            "valid": valid_count,
            "invalid": invalid_count,
        },
        "files": results.iter().map(|(path, result)| {
            serde_json::json!({
                "file": path.to_string_lossy(),
                "valid": result.is_valid(),
                "error_count": result.errors.len(),
                "warning_count": result.warnings.len(),
                "info_count": result.info.len(),
            })
        }).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
