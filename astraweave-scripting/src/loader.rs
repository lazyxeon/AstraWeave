use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;

use astraweave_security::sandbox::DEFAULT_MAX_SCRIPT_BYTES;

#[derive(Debug, Clone)]
pub struct ScriptAsset {
    pub path: PathBuf,
    pub source: String,
    pub hash: String,
}

pub struct ScriptLoader;

pub(crate) fn jail_script_path(path: &Path) -> Result<PathBuf> {
    let base = std::env::current_dir().context("script jail base")?;
    astraweave_security::path::safe_under(&base, path)
        .map_err(|err| anyhow::anyhow!("path traversal rejected: {err}"))
}

impl ScriptLoader {
    /// Load a relative `.rhai` path. Absolute paths and `..` are rejected.
    /// The read stops at [`DEFAULT_MAX_SCRIPT_BYTES`].
    pub async fn load(path: impl AsRef<Path>) -> Result<ScriptAsset> {
        let path = path.as_ref();
        validate_loader_path(path)?;
        let jailed = jail_script_path(path)?;
        let file = fs::File::open(&jailed)
            .await
            .with_context(|| format!("Failed to read script file: {}", path.display()))?;
        let mut buf = Vec::new();
        file.take(DEFAULT_MAX_SCRIPT_BYTES as u64 + 1)
            .read_to_end(&mut buf)
            .await
            .with_context(|| format!("Failed to read script file: {}", path.display()))?;
        if buf.len() > DEFAULT_MAX_SCRIPT_BYTES {
            bail!(
                "{} exceeds the {} byte script cap",
                path.display(),
                DEFAULT_MAX_SCRIPT_BYTES
            );
        }
        let source = String::from_utf8(buf).context("script is not utf-8")?;
        let hash = compute_hash(&source);

        Ok(ScriptAsset {
            path: path.to_path_buf(),
            source,
            hash,
        })
    }
}

fn validate_loader_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        bail!("script path is empty");
    }
    if path.is_absolute() {
        bail!("absolute script paths are rejected");
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            Component::ParentDir => bail!("path traversal rejected"),
            Component::RootDir | Component::Prefix(_) => {
                bail!("absolute script paths are rejected")
            }
        }
    }
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if ext != "rhai" {
        bail!("script path must end with .rhai");
    }
    Ok(())
}

fn compute_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
