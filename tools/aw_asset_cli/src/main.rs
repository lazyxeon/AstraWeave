#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use asset_signing::KeyStore;
use astraweave_asset::{AssetDatabase, AssetKind};
use base64::Engine;
use clap::{Parser, Subcommand};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;
use which::which;

mod texture_baker;
mod validators;

use texture_baker::{bake_texture, infer_config_from_path, ColorSpace};
use validators::{
    validate_ktx2_mipmaps, validate_material_toml, validate_texture, TextureValidationConfig,
};

#[derive(Parser)]
#[command(name = "aw_asset_cli")]
#[command(about = "AstraWeave asset pipeline tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run full asset cooking pipeline from config file
    Cook {
        /// Path to pipeline config file (default: aw_pipeline.toml)
        #[arg(default_value = "aw_pipeline.toml")]
        config: String,
    },
    /// Bake individual texture with mipmap generation
    BakeTexture {
        /// Input texture file path
        input: PathBuf,
        /// Output directory
        #[arg(short, long, default_value = "baked_textures")]
        output: PathBuf,
        /// Force color space (srgb or linear)
        #[arg(long)]
        color_space: Option<String>,
        /// Treat as normal map
        #[arg(long)]
        normal_map: bool,
    },
    /// Validate assets (Phase PBR-G)
    Validate {
        /// Path to asset or directory to validate
        path: PathBuf,
        /// Validation config file (optional)
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Output format: text, json
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Fail on warnings
        #[arg(long)]
        strict: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Cook { config } => cook_pipeline(&config),
        Commands::BakeTexture {
            input,
            output,
            color_space,
            normal_map,
        } => {
            let mut config = infer_config_from_path(&input);

            if let Some(cs) = color_space {
                config.color_space = match cs.to_lowercase().as_str() {
                    "srgb" => ColorSpace::Srgb,
                    "linear" => ColorSpace::Linear,
                    _ => anyhow::bail!("Invalid color space: {} (use 'srgb' or 'linear')", cs),
                };
            }

            if normal_map {
                config.is_normal_map = true;
                config.color_space = ColorSpace::Linear;
            }

            let metadata = bake_texture(&input, &output, &config)?;
            println!("\nBaked texture metadata:");
            println!("{}", serde_json::to_string_pretty(&metadata)?);
            Ok(())
        }
        Commands::Validate {
            path,
            config,
            format,
            strict,
        } => validate_assets_command(&path, config.as_deref(), &format, strict),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PipelineCfg {
    source: String, // e.g. "assets_src"
    output: String, // e.g. "assets"
    rules: Vec<Rule>,
    compress: bool, // new: compress outputs
    validate: bool, // new: validate after cooking
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum Rule {
    #[serde(rename = "texture")]
    Texture { glob: String, normal_map: bool },
    #[serde(rename = "model")]
    Model { glob: String },
    #[serde(rename = "audio")]
    Audio { glob: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ManifestEntry {
    src: String,
    out: String,
    sha256: String,
    kind: String,
    guid: String,              // new
    dependencies: Vec<String>, // new
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    entries: Vec<ManifestEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SignedManifest {
    entries: Vec<ManifestEntry>,
    signature: String, // base64 encoded Ed25519 signature
    #[serde(skip_serializing_if = "Option::is_none")]
    public_key: Option<String>, // PEM-encoded public key for verification
    #[serde(skip_serializing_if = "Option::is_none")]
    signed_at: Option<String>, // RFC3339 timestamp
}

fn cook_pipeline(cfg_path: &str) -> Result<()> {
    let cfg_text = fs::read_to_string(cfg_path).with_context(|| format!("read {}", cfg_path))?;
    let cfg: PipelineCfg = toml::from_str(&cfg_text)?;

    fs::create_dir_all(&cfg.output)?;

    // Initialize asset database
    let mut db = AssetDatabase::new();
    db.scan_directory(Path::new(&cfg.source))?;

    let mut manifest: Vec<ManifestEntry> = Vec::new();

    for rule in &cfg.rules {
        match rule {
            Rule::Texture {
                glob,
                normal_map: _,
            } => {
                for entry in globwalk(&cfg.source, glob)? {
                    let out = process_texture(&entry, &cfg.output, cfg.compress)?;
                    let guid = register_asset(&mut db, &entry, AssetKind::Texture)?;
                    manifest.push(record("texture", &entry, &out, &guid, &db)?);
                }
            }
            Rule::Model { glob } => {
                for entry in globwalk(&cfg.source, glob)? {
                    let out = process_model(&entry, &cfg.output, cfg.compress)?;
                    let guid = register_asset(&mut db, &entry, AssetKind::Mesh)?;
                    manifest.push(record("model", &entry, &out, &guid, &db)?);
                }
            }
            Rule::Audio { glob } => {
                for entry in globwalk(&cfg.source, glob)? {
                    let out = process_audio(&entry, &cfg.output, cfg.compress)?;
                    let guid = register_asset(&mut db, &entry, AssetKind::Audio)?;
                    manifest.push(record("audio", &entry, &out, &guid, &db)?);
                }
            }
        }
    }

    // Save asset database manifest
    let db_manifest_path = Path::new(&cfg.output).join("assets.json");
    db.save_manifest(&db_manifest_path)?;

    // Sign the manifest
    let signed_manifest = sign_manifest(&manifest)?;
    let manifest_path = Path::new(&cfg.output).join("manifest.json");
    fs::write(&manifest_path, serde_json::to_vec_pretty(&signed_manifest)?)?;
    println!("Wrote {}", manifest_path.display());

    if cfg.validate {
        validate_assets(&signed_manifest.entries, &db)?;
        println!("Validation passed");
    }

    Ok(())
}

fn globwalk(root: &str, pat: &str) -> Result<Vec<PathBuf>> {
    let mut v = vec![];

    // Handle brace expansion like *.{png,jpg,jpeg}
    let patterns = if pat.contains('{') && pat.contains('}') {
        let start = pat.find('{').unwrap();
        let end = pat.find('}').unwrap();
        let prefix = &pat[..start];
        let suffix = &pat[end + 1..];
        let extensions = &pat[start + 1..end];

        extensions
            .split(',')
            .map(|ext| format!("{}{}{}", prefix, ext, suffix))
            .collect::<Vec<_>>()
    } else {
        vec![pat.to_string()]
    };

    for pattern_str in patterns {
        for e in WalkDir::new(root) {
            let e = e?;
            if e.file_type().is_file() {
                let p = e.into_path();
                // Create pattern relative to root for matching
                let relative_path = p.strip_prefix(root).unwrap_or(&p);
                if glob::Pattern::new(&pattern_str)?.matches_path(relative_path) && !v.contains(&p)
                {
                    v.push(p);
                }
            }
        }
    }
    Ok(v)
}

fn register_asset(db: &mut AssetDatabase, src: &Path, kind: AssetKind) -> Result<String> {
    // Infer dependencies based on kind
    let dependencies = match kind {
        AssetKind::Mesh => {
            // For glTF, parse dependencies
            if src.extension().and_then(|e| e.to_str()) == Some("gltf") {
                // For now, empty; in full impl, parse glTF for textures
                vec![]
            } else {
                vec![]
            }
        }
        _ => vec![],
    };
    db.register_asset(src, kind, dependencies)
}

fn record(
    kind: &str,
    src: &Path,
    out: &Path,
    guid: &str,
    db: &AssetDatabase,
) -> Result<ManifestEntry> {
    let mut f = fs::File::open(out)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut f, &mut hasher)?;
    let sha = hex::encode(hasher.finalize());
    let dependencies = db
        .get_dependencies(guid)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();
    Ok(ManifestEntry {
        src: src.to_string_lossy().to_string(),
        out: out.to_string_lossy().to_string(),
        sha256: sha,
        kind: kind.into(),
        guid: guid.to_string(),
        dependencies,
    })
}

fn process_texture(src: &Path, out_root: &str, compress: bool) -> Result<PathBuf> {
    fs::create_dir_all(out_root)?;
    let stem = src.file_stem().unwrap().to_string_lossy();
    let out = Path::new(out_root).join(format!("{stem}.ktx2"));
    let processed = if compress {
        out.with_extension("ktx2.gz")
    } else {
        out.clone()
    };
    // Prefer toktx; fallback basisu; fallback copy
    // Check for compression tools and fail hard if missing
    let has_toktx = which("toktx").is_ok();
    let has_basisu = which("basisu").is_ok();

    if !has_toktx && !has_basisu {
        return Err(anyhow!(
            "No texture compression tools found in PATH.\n\
            \n\
            Install one of:\n\
            1. toktx (Khronos KTX-Software)\n\
               - Download: https://github.com/KhronosGroup/KTX-Software/releases\n\
               - Provides: KTX2 creation with UASTC/BC7/ASTC compression\n\
            \n\
            2. basisu (Basis Universal)\n\
               - Download: https://github.com/BinomialLLC/basis_universal/releases\n\
               - Provides: .basis compression (transcodes to multiple formats)\n\
            \n\
            After installation, ensure the tool is in your system PATH."
        ));
    }

    if let Ok(toktx) = which("toktx") {
        // BasisU UASTC KTX2 with Zstd
        let status = Command::new(toktx)
            .args([
                "--genmipmap",
                "--uastc",
                "--zcmp",
                "18",
                out.to_str().unwrap(),
                src.to_str().unwrap(),
            ])
            .status()?;
        if status.success() {
            if compress {
                compress_file(&out, &processed)?;
                fs::remove_file(&out)?;
            }
            return Ok(processed);
        }
    }
    if let Ok(basisu) = which("basisu") {
        let _tmp = out.with_extension("basis");
        let status = Command::new(basisu)
            .args(["-uastc", "-comp_level", "2", "-file", src.to_str().unwrap()])
            .status()?;
        if status.success() {
            // leave .basis or convert later; for now write .basis → .ktx2 not implemented
            fs::copy(src, &out)?; // placeholder
            if compress {
                compress_file(&out, &processed)?;
                fs::remove_file(&out)?;
            }
            return Ok(processed);
        }
    }
    fs::copy(src, &out)?; // fallback
    if compress {
        compress_file(&out, &processed)?;
        fs::remove_file(&out)?;
    }
    Ok(processed)
}

fn process_model(src: &Path, out_root: &str, compress: bool) -> Result<PathBuf> {
    fs::create_dir_all(out_root)?;
    let stem = src.file_stem().unwrap().to_string_lossy();
    let out = Path::new(out_root).join(format!("{stem}.meshbin"));
    let processed = if compress {
        out.with_extension("meshbin.gz")
    } else {
        out.clone()
    };
    if src.extension().map(|e| e.to_string_lossy().to_lowercase()) == Some("gltf".into())
        || src.extension().map(|e| e.to_string_lossy().to_lowercase()) == Some("glb".into())
    {
        let (_doc, _buffers, _images) = gltf::import(src)?;
        // Minimal example: just copy the glTF for now; swap to a meshbin writer later
        fs::copy(src, &out)?;
        if compress {
            compress_file(&out, &processed)?;
            fs::remove_file(&out)?;
        }
        Ok(processed)
    } else {
        fs::copy(src, &out)?;
        if compress {
            compress_file(&out, &processed)?;
            fs::remove_file(&out)?;
        }
        Ok(processed)
    }
}

fn process_audio(src: &Path, out_root: &str, compress: bool) -> Result<PathBuf> {
    fs::create_dir_all(out_root)?;
    let stem = src.file_stem().unwrap().to_string_lossy();
    let out = Path::new(out_root).join(format!("{stem}.ogg"));
    let processed = if compress {
        out.with_extension("ogg.gz")
    } else {
        out.clone()
    };
    if let Ok(oggenc) = which("oggenc") {
        let status = Command::new(oggenc)
            .args([
                "-q",
                "4",
                src.to_str().unwrap(),
                "-o",
                out.to_str().unwrap(),
            ])
            .status()?;
        if status.success() {
            if compress {
                compress_file(&out, &processed)?;
                fs::remove_file(&out)?;
            }
            return Ok(processed);
        }
    }
    // fallback: keep wav as-is
    let out_wav = Path::new(out_root).join(format!("{stem}.wav"));
    let processed = if compress {
        out_wav.with_extension("wav.gz")
    } else {
        out_wav.clone()
    };
    fs::copy(src, &out_wav)?;
    if compress {
        compress_file(&out_wav, &processed)?;
        fs::remove_file(&out_wav)?;
    }
    Ok(processed)
}

fn compress_file(input: &Path, output: &Path) -> Result<()> {
    let input_data = fs::read(input)?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    std::io::Write::write_all(&mut encoder, &input_data)?;
    let compressed = encoder.finish()?;
    fs::write(output, compressed)?;
    Ok(())
}

fn validate_assets(manifest: &[ManifestEntry], db: &AssetDatabase) -> Result<()> {
    for entry in manifest {
        // Check if file exists and hash matches
        if !Path::new(&entry.out).exists() {
            anyhow::bail!("Output file {} does not exist", entry.out);
        }
        let mut f = fs::File::open(&entry.out)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut f, &mut hasher)?;
        let sha = hex::encode(hasher.finalize());
        if sha != entry.sha256 {
            anyhow::bail!("Hash mismatch for {}", entry.out);
        }
        // Check dependencies
        if let Some(meta) = db.get_asset(&entry.guid) {
            for dep in &meta.dependencies {
                if db.get_asset(dep).is_none() {
                    anyhow::bail!("Missing dependency {} for {}", dep, entry.guid);
                }
            }
        }
    }
    Ok(())
}

fn sign_manifest(manifest: &[ManifestEntry]) -> Result<SignedManifest> {
    // Load or generate persistent signing key from OS keyring
    let keystore = KeyStore::load_or_generate("developer_signing_key")
        .map_err(|e| anyhow::anyhow!("Failed to access signing key: {}", e))?;

    // Serialize manifest entries for signing (canonical JSON)
    let manifest_json = serde_json::to_string(manifest)?;

    // Sign the manifest
    let signature = keystore.sign(&manifest_json);

    // Export public key alongside manifest for verification
    let public_key_pem = format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
        base64::engine::general_purpose::STANDARD.encode(keystore.verifying_key().as_bytes())
    );

    Ok(SignedManifest {
        entries: manifest.to_vec(),
        signature,
        public_key: Some(public_key_pem),
        signed_at: Some(chrono::Utc::now().to_rfc3339()),
    })
}

/// Validate assets command handler for Phase PBR-G Task 1
fn validate_assets_command(
    path: &Path,
    config_path: Option<&Path>,
    format: &str,
    strict: bool,
) -> Result<()> {
    use validators::{TextureValidationConfig, ValidationResult};

    // Load validation config or use defaults
    let config = if let Some(cfg_path) = config_path {
        let cfg_str = fs::read_to_string(cfg_path)
            .with_context(|| format!("Failed to read config from {}", cfg_path.display()))?;
        toml::from_str(&cfg_str)
            .with_context(|| format!("Failed to parse config TOML from {}", cfg_path.display()))?
    } else {
        TextureValidationConfig::default()
    };

    let mut results: Vec<ValidationResult> = Vec::new();

    // Validate single file or directory
    if path.is_file() {
        let result = validate_single_asset(path, &config)?;
        results.push(result);
    } else if path.is_dir() {
        // Walk directory and validate all supported assets
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let file_path = entry.path();

                // Check if file is a supported asset type
                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                    match ext.to_lowercase().as_str() {
                        "ktx2" | "png" | "jpg" | "jpeg" | "tga" | "bmp" | "toml" => {
                            match validate_single_asset(file_path, &config) {
                                Ok(result) => results.push(result),
                                Err(e) => {
                                    // Create error result for failed validation
                                    results.push(ValidationResult {
                                        asset_path: file_path.display().to_string(),
                                        passed: false,
                                        errors: vec![format!("Validation failed: {}", e)],
                                        warnings: vec![],
                                        info: vec![],
                                    });
                                }
                            }
                        }
                        _ => {} // Skip unsupported file types
                    }
                }
            }
        }
    } else {
        anyhow::bail!("Path {} is neither a file nor a directory", path.display());
    }

    // Output results in requested format
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&results)
                .context("Failed to serialize results to JSON")?;
            println!("{}", json);
        }
        _ => {
            // Text format (default)
            println!("\n=== Asset Validation Results ===\n");

            let mut passed_count = 0;
            let mut failed_count = 0;
            let mut warning_count = 0;

            for result in &results {
                if result.passed && result.warnings.is_empty() {
                    passed_count += 1;
                    println!("✅ {}", result.asset_path);
                } else if result.passed && !result.warnings.is_empty() {
                    passed_count += 1;
                    warning_count += result.warnings.len();
                    println!(
                        "⚠️  {} ({} warnings)",
                        result.asset_path,
                        result.warnings.len()
                    );
                } else {
                    failed_count += 1;
                    println!("❌ {} ({} errors)", result.asset_path, result.errors.len());
                }

                // Show errors
                for error in &result.errors {
                    println!("   ERROR: {}", error);
                }

                // Show warnings
                for warning in &result.warnings {
                    println!("   WARN:  {}", warning);
                }

                // Show info (only in verbose mode or if explicitly requested)
                if !result.info.is_empty() && std::env::var("VERBOSE").is_ok() {
                    for info in &result.info {
                        println!("   INFO:  {}", info);
                    }
                }

                println!(); // Blank line between results
            }

            // Summary
            println!("=== Summary ===");
            println!("Total assets: {}", results.len());
            println!("Passed: {}", passed_count);
            println!("Failed: {}", failed_count);
            println!("Warnings: {}", warning_count);
        }
    }

    // Exit with error code if strict mode and there are warnings or errors
    let has_issues = results.iter().any(|r| !r.passed || !r.warnings.is_empty());
    if strict && has_issues {
        eprintln!("\n❌ Validation failed in strict mode (errors or warnings present)");
        std::process::exit(1);
    }

    Ok(())
}

/// Validate a single asset file
fn validate_single_asset(
    path: &Path,
    config: &TextureValidationConfig,
) -> Result<validators::ValidationResult> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "ktx2" => validate_ktx2_mipmaps(path),
        "toml" => validate_material_toml(path),
        "png" | "jpg" | "jpeg" | "tga" | "bmp" => validate_texture(path, config),
        _ => Ok(validators::ValidationResult {
            asset_path: path.display().to_string(),
            passed: false,
            errors: vec![format!("Unsupported file extension: {}", ext)],
            warnings: vec![],
            info: vec![],
        }),
    }
}
