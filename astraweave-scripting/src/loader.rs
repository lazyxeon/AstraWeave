use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;

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
    pub async fn load(path: impl AsRef<Path>) -> Result<ScriptAsset> {
        let path = path.as_ref();
        let jailed = jail_script_path(path)?;
        let source = fs::read_to_string(&jailed)
            .await
            .context(format!("Failed to read script file: {}", path.display()))?;

        // Compute hash for change detection
        let hash = compute_hash(&source);

        Ok(ScriptAsset {
            path: path.to_path_buf(),
            source,
            hash,
        })
    }
}

fn compute_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}
