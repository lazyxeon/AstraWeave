use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use time::OffsetDateTime;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

#[derive(Parser)]
#[command(
    name = "aw_build",
    version,
    about = "Bundle binaries/assets and build delta patches"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Make a release bundle zip + manifest
    Bundle {
        /// Directory containing built binaries, e.g. target/release
        #[arg(long, default_value = "target/release")]
        bin_dir: PathBuf,
        /// Path to include as assets root (copied recursively)
        #[arg(long, default_value = "assets")]
        assets: PathBuf,
        /// Output directory for dist files
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        /// Bundle name (e.g., astraweave-windows-x64)
        #[arg(long)]
        name: String,
        /// Version string (e.g., 0.4.0)
        #[arg(long)]
        version: String,
    },
    /// Create a delta patch zip between two manifests
    Patch {
        #[arg(long)]
        old_manifest: PathBuf,
        #[arg(long)]
        new_manifest: PathBuf,
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        /// Patch name (e.g., patch-0.4.0-to-0.4.1)
        #[arg(long)]
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    name: String,
    version: String,
    created_at: String,
    files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileEntry {
    rel: String,
    sha256: String,
    size: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Bundle {
            bin_dir,
            assets,
            out,
            name,
            version,
        } => bundle(bin_dir, assets, out, name, version),
        Cmd::Patch {
            old_manifest,
            new_manifest,
            out,
            name,
        } => patch(old_manifest, new_manifest, out, name),
    }
}

fn bundle(
    bin_dir: PathBuf,
    assets: PathBuf,
    out: PathBuf,
    name: String,
    version: String,
) -> Result<()> {
    fs::create_dir_all(&out)?;
    let work = staging(&bin_dir, &assets, &out, &name)?;
    let manifest = gen_manifest(&work, &name, &version)?;
    let manifest_path = out.join(format!("{name}-{version}.manifest.json"));
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

    let zip_path = out.join(format!("{name}-{version}.zip"));
    write_zip(&zip_path, &work)?;
    println!("Bundle: {}", zip_path.display());
    println!("Manifest: {}", manifest_path.display());
    Ok(())
}

fn patch(old_manifest: PathBuf, new_manifest: PathBuf, out: PathBuf, name: String) -> Result<()> {
    let old: Manifest = serde_json::from_slice(&fs::read(&old_manifest)?)?;
    let new: Manifest = serde_json::from_slice(&fs::read(&new_manifest)?)?;

    let changed: Vec<&FileEntry> = new
        .files
        .iter()
        .filter(|nf| {
            !old.files
                .iter()
                .any(|of| of.rel == nf.rel && of.sha256 == nf.sha256)
        })
        .collect();

    if changed.is_empty() {
        println!("No changes between manifests");
        return Ok(());
    }

    let patch_dir = new_manifest
        .parent()
        .unwrap()
        .join(format!("_patch_src_{name}"));
    if patch_dir.exists() {
        fs::remove_dir_all(&patch_dir)?;
    }
    fs::create_dir_all(&patch_dir)?;

    // Copy changed files from new bundle folder location (assumes layout under dist/<name>-<ver> folder)
    // We resolve by reading rel under sibling directory structure:
    // The user should keep bundles unzipped nearby for patch assembly (CI does this).
    for ent in changed {
        // Search up to two levels up from manifest location for the file; CI handles exact paths.
        // For local usage, place unzipped bundle next to manifest.
        let candidate = find_rel_upwards(new_manifest.parent().unwrap(), &ent.rel)
            .with_context(|| format!("locate {}", ent.rel))?;
        let dest = patch_dir.join(&ent.rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(candidate, &dest)?;
    }

    fs::create_dir_all(&out)?;
    let zip_path = out.join(format!("{name}.zip"));
    write_zip(&zip_path, &patch_dir)?;
    println!("Patch: {}", zip_path.display());
    Ok(())
}

fn gen_manifest(root: &Path, name: &str, version: &str) -> Result<Manifest> {
    let paths: Vec<_> = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .collect();

    let files = paths
        .par_iter()
        .map(|e| {
            let p = e.path();
            let rel = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let data = fs::read(p).unwrap();
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let sha = hex::encode(hasher.finalize());
            FileEntry {
                rel,
                sha256: sha,
                size: data.len() as u64,
            }
        })
        .collect();

    Ok(Manifest {
        name: name.into(),
        version: version.into(),
        created_at: OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)?,
        files,
    })
}

fn write_zip(zip_path: &Path, src_root: &Path) -> Result<()> {
    let f = fs::File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(f);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(src_root).into_iter().filter_map(Result::ok) {
        let p = entry.path();
        if entry.file_type().is_dir() {
            continue;
        }
        let rel = p
            .strip_prefix(src_root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(rel, options)?;
        let bytes = fs::read(p)?;
        zip.write_all(&bytes)?;
    }
    zip.finish()?;
    Ok(())
}

fn staging(bin_dir: &Path, assets: &Path, out: &Path, name: &str) -> Result<PathBuf> {
    let stage = out.join(format!("{}_stage", name));
    if stage.exists() {
        fs::remove_dir_all(&stage)?;
    }
    fs::create_dir_all(&stage)?;

    // copy binaries (all files in bin_dir)
    if bin_dir.exists() {
        for e in WalkDir::new(bin_dir).into_iter().filter_map(Result::ok) {
            if e.file_type().is_file() {
                let rel = e.path().strip_prefix(bin_dir).unwrap();
                let dst = stage.join("bin").join(rel);
                if let Some(p) = dst.parent() {
                    fs::create_dir_all(p)?;
                }
                fs::copy(e.path(), &dst)?;
            }
        }
    }

    // copy assets (optional)
    if assets.exists() {
        for e in WalkDir::new(assets).into_iter().filter_map(Result::ok) {
            if e.file_type().is_file() {
                let rel = e.path().strip_prefix(assets).unwrap();
                let dst = stage.join("assets").join(rel);
                if let Some(p) = dst.parent() {
                    fs::create_dir_all(p)?;
                }
                fs::copy(e.path(), &dst)?;
            }
        }
    }

    Ok(stage)
}

/// Walk up to find a relative path under unzipped bundle folders (helper for local patch builds).
fn find_rel_upwards(start: &Path, rel: &str) -> Result<PathBuf> {
    let mut cur = Some(start.to_path_buf());
    while let Some(p) = cur {
        let candidate = p.join(rel);
        if candidate.exists() {
            return Ok(candidate);
        }
        cur = p.parent().map(|x| x.to_path_buf());
    }
    anyhow::bail!("not found: {}", rel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn manifest_diff_detects_changes() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        fs::write(&a, "hello").unwrap();
        fs::write(&b, "world").unwrap();

        let root1 = dir.path().join("root1");
        let root2 = dir.path().join("root2");
        fs::create_dir_all(&root1).unwrap();
        fs::create_dir_all(&root2).unwrap();
        fs::copy(&a, root1.join("a.txt")).unwrap();
        fs::copy(&b, root1.join("b.txt")).unwrap();

        fs::copy(&a, root2.join("a.txt")).unwrap();
        fs::write(root2.join("b.txt"), "world!").unwrap(); // changed

        let m1 = gen_manifest(&root1, "n", "1").unwrap();
        let m2 = gen_manifest(&root2, "n", "2").unwrap();

        let changed: Vec<&FileEntry> = m2
            .files
            .iter()
            .filter(|nf| {
                !m1.files
                    .iter()
                    .any(|of| of.rel == nf.rel && of.sha256 == nf.sha256)
            })
            .collect();

        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].rel, "b.txt");
    }
}
