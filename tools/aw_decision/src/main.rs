//! Headless decision inspector.
//!
//! ```text
//! aw_decision show --script aravath-bridge --tick 31 --entity 1
//! aw_decision diff --script aravath-bridge --tick 31 --entity 1 --seal 1,17
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use aw_decision::{diff_seal, dump_record, show, SCENE_LABEL};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aw_decision")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print one recorded decision.
    Show {
        #[arg(long, default_value = "aravath-bridge")]
        script: String,
        #[arg(long)]
        tick: u64,
        #[arg(long)]
        entity: u64,
        #[arg(long)]
        dump: Option<PathBuf>,
    },
    /// Rerun one tick with a single sealed cell.
    Diff {
        #[arg(long, default_value = "aravath-bridge")]
        script: String,
        #[arg(long)]
        tick: u64,
        #[arg(long)]
        entity: u64,
        /// Cell to seal, written as `x,y`.
        #[arg(long)]
        seal: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Show {
            script,
            tick,
            entity,
            dump,
        } => show_cmd(&script, tick, entity, dump.as_deref()),
        Cmd::Diff {
            script,
            tick,
            entity,
            seal,
        } => diff_cmd(&script, tick, entity, &seal),
    };
    match result {
        Ok(text) => {
            print!("{text}");
            if !text.ends_with('\n') {
                println!();
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn show_cmd(
    script: &str,
    tick: u64,
    entity: u64,
    dump: Option<&std::path::Path>,
) -> Result<String, String> {
    require_script(script)?;
    if let Some(path) = dump {
        dump_record(path, tick, entity)?;
    }
    let mut text = show(tick, entity)?;
    if !text.contains(SCENE_LABEL) {
        text = format!("{SCENE_LABEL}\n{text}");
    }
    Ok(text)
}

fn diff_cmd(script: &str, tick: u64, entity: u64, seal: &str) -> Result<String, String> {
    require_script(script)?;
    let (x, y) = parse_cell(seal)?;
    diff_seal(tick, entity, x, y)
}

fn require_script(script: &str) -> Result<(), String> {
    if script == "aravath-bridge" {
        Ok(())
    } else {
        Err(format!("unknown script {script}"))
    }
}

fn parse_cell(text: &str) -> Result<(i32, i32), String> {
    let (x, y) = text
        .split_once(',')
        .ok_or_else(|| format!("seal must be x,y, got {text}"))?;
    let x = x
        .trim()
        .parse::<i32>()
        .map_err(|_| format!("seal x is not an integer: {x}"))?;
    let y = y
        .trim()
        .parse::<i32>()
        .map_err(|_| format!("seal y is not an integer: {y}"))?;
    Ok((x, y))
}
