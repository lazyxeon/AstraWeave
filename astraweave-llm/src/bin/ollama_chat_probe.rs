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
        eprintln!(
            "usage: ollama_chat_probe <complete|batch|qwen-stream|qwen-think|qwen-health|phi-health|hermes-health|hermes-stream> [url]"
        );
        return ExitCode::from(2);
    };
    let url = args
        .next()
        .unwrap_or_else(|| "http://localhost:12345".to_string());
    if args.next().is_some() {
        eprintln!(
            "usage: ollama_chat_probe <complete|batch|qwen-stream|qwen-think|qwen-health|phi-health|hermes-health|hermes-stream> [url]"
        );
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
        match mode.as_str() {
            "complete" => {
                let client =
                    astraweave_llm::OllamaChatClient::new(url.clone(), "test-model".to_string())?;
                client.complete("test").await
            }
            "batch" => {
                let client =
                    astraweave_llm::OllamaChatClient::new(url.clone(), "test-model".to_string())?;
                let prompts = vec!["p1".to_string(), "p2".to_string()];
                client
                    .complete_batch(&prompts)
                    .await
                    .map(|parts| parts.join(""))
            }
            "qwen-stream" => {
                use futures_util::StreamExt;
                let client =
                    astraweave_llm::qwen3_ollama::Qwen3Ollama::new(url.clone(), "test-model")?;
                let mut stream = client.complete_streaming("test").await?;
                let mut acc = String::new();
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(text) => acc.push_str(&text),
                        Err(err) => {
                            if acc.is_empty() {
                                return Err(err);
                            }
                            return Ok(acc);
                        }
                    }
                }
                Ok(acc)
            }
            "qwen-think" => {
                use futures_util::StreamExt;
                let client =
                    astraweave_llm::qwen3_ollama::Qwen3Ollama::new(url.clone(), "test-model")?
                        .with_thinking(true);
                let mut stream = client.complete_streaming("test").await?;
                let mut acc = String::new();
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(text) => acc.push_str(&text),
                        Err(err) => {
                            if acc.is_empty() {
                                return Err(err);
                            }
                            return Ok(acc);
                        }
                    }
                }
                Ok(acc)
            }
            "qwen-health" => astraweave_llm::qwen3_ollama::Qwen3Ollama::new(url.clone(), "m")?
                .health_check()
                .await
                .map(|_| String::new()),
            "phi-health" => astraweave_llm::phi3_ollama::Phi3Ollama::new(url.clone(), "m")?
                .health_check()
                .await
                .map(|_| String::new()),
            "hermes-health" => {
                astraweave_llm::hermes2pro_ollama::Hermes2ProOllama::new(url.clone(), "m")?
                    .health_check()
                    .await
                    .map(|_| String::new())
            }
            "hermes-stream" => {
                use futures_util::StreamExt;
                let client = astraweave_llm::hermes2pro_ollama::Hermes2ProOllama::new(
                    url.clone(),
                    "test-model",
                )?;
                let mut stream = client.complete_streaming("test").await?;
                let mut acc = String::new();
                while let Some(item) = stream.next().await {
                    match item {
                        Ok(text) => acc.push_str(&text),
                        Err(err) => {
                            if acc.is_empty() {
                                return Err(err);
                            }
                            return Ok(acc);
                        }
                    }
                }
                Ok(acc)
            }
            other => anyhow::bail!("unknown mode {other}"),
        }
    });
    match result {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            println!("ERR {err:#}");
            ExitCode::from(1)
        }
    }
}
