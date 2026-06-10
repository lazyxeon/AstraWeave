//! Thin CLI wrapper for the aw-net-server library: parse arguments into a
//! [`ServerConfig`] and run the server. All server logic lives in `lib.rs`.

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use aw_net_proto::SigningKey;
use aw_net_server::{run_server, ServerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(
                "info"
                    .parse()
                    .map_err(|e| anyhow!("Invalid log directive: {}", e))?,
            ),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let config = parse_args(&args)?;
    run_server(config).await
}

fn parse_args(args: &[String]) -> Result<ServerConfig> {
    let mut config = ServerConfig::default();
    let mut key_provided = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--disable-tls" => {
                #[cfg(not(debug_assertions))]
                {
                    return Err(anyhow!(
                        "SECURITY: TLS cannot be disabled in release builds"
                    ));
                }
                #[cfg(debug_assertions)]
                {
                    config.tls_enabled = false;
                    info!("TLS disabled via command line (debug build only)");
                }
            }
            "--tls-cert" => {
                if i + 1 < args.len() {
                    config.tls_cert_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--tls-key" => {
                if i + 1 < args.len() {
                    config.tls_key_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--shared-key-hex" => {
                let value = take_value(args, &mut i, "--shared-key-hex")?;
                // SECURITY: never echo the supplied value — it is key material.
                config.signing_key = SigningKey::from_hex(value).map_err(|_| {
                    anyhow!(
                        "invalid --shared-key-hex: expected exactly 64 hexadecimal characters \
                         (32 bytes); the supplied value is not echoed"
                    )
                })?;
                key_provided = true;
            }
            "--sig-failure-policy" => {
                let value = take_value(args, &mut i, "--sig-failure-policy")?;
                config.sig_failure_policy = value
                    .parse()
                    .map_err(|e| anyhow!("invalid --sig-failure-policy: {e}"))?;
            }
            "--ws-listen" => {
                let value = take_value(args, &mut i, "--ws-listen")?;
                config.ws_listen = value
                    .parse()
                    .map_err(|e| anyhow!("invalid --ws-listen address '{value}': {e}"))?;
            }
            "--http-listen" => {
                let value = take_value(args, &mut i, "--http-listen")?;
                config.http_listen = value
                    .parse()
                    .map_err(|e| anyhow!("invalid --http-listen address '{value}': {e}"))?;
            }
            "--db-path" => {
                let value = take_value(args, &mut i, "--db-path")?;
                config.db_path = PathBuf::from(value);
            }
            _ => {}
        }
        i += 1;
    }

    if !key_provided {
        warn!(
            "no --shared-key-hex provided; using the built-in DEVELOPMENT signing key \
             (NOT FOR PRODUCTION)"
        );
    }

    Ok(config)
}

/// Consume the value following `args[*i]` for `flag`, failing fast if absent.
fn take_value<'a>(args: &'a [String], i: &mut usize, flag: &str) -> Result<&'a str> {
    *i += 1;
    args.get(*i)
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow!("{flag} requires a value"))
}
