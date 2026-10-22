#![forbid(unsafe_code)]

//! Child process for the closed-port Ollama chat tests.
//!
//! The parent sets the timeout variables with `Command::env`. This binary does
//! not call `set_var` or `remove_var`.

use astraweave_llm::LlmClient;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(mode) = args.next() else {
        eprintln!("usage: ollama_chat_probe <complete|batch>");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("usage: ollama_chat_probe <complete|batch>");
        return ExitCode::from(2);
    }

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            println!("ERR {err}");
            return ExitCode::from(1);
        }
    };
    let result = runtime.block_on(async {
        let client = astraweave_llm::OllamaChatClient::new(
            "http://localhost:12345".to_string(),
            "test-model".to_string(),
        )?;
        match mode.as_str() {
            "complete" => client.complete("test").await.map(|_| ()),
            "batch" => {
                let prompts = vec!["p1".to_string(), "p2".to_string()];
                client.complete_batch(&prompts).await.map(|_| ())
            }
            other => anyhow::bail!("unknown mode {other}"),
        }
    });
    match result {
        Ok(()) => {
            println!("OK");
            ExitCode::SUCCESS
        }
        Err(err) => {
            println!("ERR {err:#}");
            ExitCode::from(1)
        }
    }
}
