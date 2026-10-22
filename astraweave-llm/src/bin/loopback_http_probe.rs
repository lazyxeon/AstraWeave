#![forbid(unsafe_code)]

//! Child process for the proxy regression.
//!
//! The parent sets `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY` with
//! `Command::env`. This binary does not call `set_var` or `remove_var`.
//! `loopback` builds [`astraweave_llm::loopback_http_client`]. `default` builds
//! reqwest's client, which reads those variables inside `Client::build`.

use std::env;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let (Some(mode), Some(url)) = (args.next(), args.next()) else {
        eprintln!("usage: loopback_http_probe <loopback|default> <url>");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("usage: loopback_http_probe <loopback|default> <url>");
        return ExitCode::from(2);
    }

    let builder = match mode.as_str() {
        "loopback" => astraweave_llm::loopback_http_client(),
        "default" => reqwest::Client::builder(),
        other => {
            eprintln!("unknown mode {other}");
            return ExitCode::from(2);
        }
    };
    let client = match builder.timeout(Duration::from_secs(2)).build() {
        Ok(client) => client,
        Err(err) => {
            println!("ERR {err}");
            return ExitCode::from(1);
        }
    };

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
        let response = client.get(&url).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            anyhow::bail!("status {status}: {body}");
        }
        Ok::<String, anyhow::Error>(body)
    });
    match result {
        Ok(body) => {
            print!("{body}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            println!("ERR {err:#}");
            ExitCode::from(1)
        }
    }
}
