//! Scene decomposition engine.
//!
//! This module decomposes a multi-object .blend file into individual asset
//! files (one GLB per object), with a manifest describing all extracted assets.

use crate::cache::ConversionCache;
use crate::discovery::BlenderInstallation;
use crate::error::{BlendError, BlendResult};
use crate::export_script::generate_decomposition_script;
use crate::options::ConversionOptions;
use crate::progress::{CancellationToken, ConversionStage, ProgressTracker};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, info, warn};

// ============================================================================
// Result types
// ============================================================================

/// Metadata for a single decomposed asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedAsset {
    /// Blender object name.
    pub name: String,
    /// Relative path to the GLB file (e.g. `meshes/Boulder_01.glb`).
    pub filename: String,
    /// Auto-classified category: `vegetation`, `rock`, `terrain`, `prop`, `billboard`.
    pub category: String,
    /// Number of vertices in the mesh.
    pub vertex_count: u64,
    /// File size in bytes.
    pub file_size: u64,
    /// Axis-aligned bounding box (min/max corners), if computed.
    pub bounds: Option<AssetBounds>,
    /// Object dimensions [width, depth, height].
    pub dimensions: Option<[f64; 3]>,
    /// Original transform in the .blend scene.
    pub position: [f64; 3],
    /// Original rotation (Euler XYZ radians).
    pub rotation: [f64; 3],
    /// Original scale.
    pub scale: [f64; 3],
    /// Textures associated with this asset.
    pub textures: Vec<AssetTexture>,
    /// Material names assigned to this object.
    pub materials: Vec<String>,
    /// Structured PBR material parameters (populated from Blender data when available).
    #[serde(default)]
    pub material_descs: Vec<MaterialDesc>,
    /// Blender collections this object belonged to.
    pub collections: Vec<String>,
}

/// Axis-aligned bounding box for an asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetBounds {
    /// Minimum corner [x, y, z].
    pub min: [f64; 3],
    /// Maximum corner [x, y, z].
    pub max: [f64; 3],
}

/// A texture extracted alongside an asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTexture {
    /// Filename relative to the textures/ directory.
    pub filename: String,
    /// PBR channel: `diffuse`, `normal`, `roughness`, `metallic`, `alpha`, `displacement`, `emission`, `unknown`.
    pub channel: String,
    /// Original Blender image name.
    pub original_name: String,
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
}

/// Structured PBR material parameters extracted from a Blender material.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaterialDesc {
    /// Material name from Blender.
    pub name: String,
    /// Base color factor [R, G, B, A].
    #[serde(default = "default_base_color")]
    pub base_color_factor: [f32; 4],
    /// Metallic factor (0.0–1.0).
    #[serde(default)]
    pub metallic_factor: f32,
    /// Roughness factor (0.0–1.0).
    #[serde(default = "default_one")]
    pub roughness_factor: f32,
    /// Emissive color [R, G, B].
    #[serde(default)]
    pub emissive_factor: [f32; 3],
    /// Emissive strength multiplier.
    #[serde(default = "default_one")]
    pub emissive_strength: f32,
    /// Alpha mode: "OPAQUE", "MASK", or "BLEND".
    #[serde(default = "default_alpha_mode")]
    pub alpha_mode: String,
    /// Alpha cutoff for MASK mode.
    #[serde(default = "default_alpha_cutoff")]
    pub alpha_cutoff: f32,
    /// Whether the material should be rendered double-sided.
    #[serde(default)]
    pub double_sided: bool,
    /// Index of refraction.
    #[serde(default = "default_ior")]
    pub ior: f32,
    /// Transmission factor (0.0–1.0).
    #[serde(default)]
    pub transmission_factor: f32,
}

fn default_base_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_one() -> f32 {
    1.0
}

fn default_alpha_mode() -> String {
    "OPAQUE".to_string()
}

fn default_alpha_cutoff() -> f32 {
    0.5
}

fn default_ior() -> f32 {
    1.5
}

/// An extracted HDRI environment map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedHdri {
    /// Filename relative to the hdri/ directory.
    pub filename: String,
    /// Original Blender image name.
    pub original_name: String,
    /// Image width.
    pub width: u32,
    /// Image height.
    pub height: u32,
}

/// An empty (placement marker) extracted from the scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEmpty {
    /// Blender object name.
    pub name: String,
    /// World position.
    pub position: [f64; 3],
    /// Rotation (Euler XYZ radians).
    pub rotation: [f64; 3],
    /// Scale.
    pub scale: [f64; 3],
}

/// Complete result of a scene decomposition operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionResult {
    /// Directory where all assets were written.
    pub output_dir: PathBuf,
    /// All extracted mesh assets.
    pub assets: Vec<DecomposedAsset>,
    /// Extracted empties / placement markers.
    pub empties: Vec<ExtractedEmpty>,
    /// Extracted HDRIs.
    pub hdris: Vec<ExtractedHdri>,
    /// Total objects processed.
    pub total_objects: usize,
    /// Path to the manifest.json (if generated).
    pub manifest_path: Option<PathBuf>,
    /// Path to the textures directory (if textures were extracted).
    pub textures_dir: Option<PathBuf>,
    /// Total duration of the decomposition.
    pub duration: std::time::Duration,
    /// Blender version used.
    pub blender_version: String,
}

/// Raw JSON result parsed from the Python decomposition script.
#[derive(Debug, Deserialize)]
struct RawDecompResult {
    success: bool,
    error: Option<String>,
    #[serde(default)]
    traceback: Option<String>,
    #[serde(default)]
    assets: Vec<serde_json::Value>,
    #[serde(default)]
    empties: Vec<serde_json::Value>,
    #[serde(default)]
    hdris: Vec<serde_json::Value>,
    #[serde(default)]
    total_objects: usize,
    #[serde(default)]
    textures_dir: Option<String>,
    #[serde(default)]
    export_errors: Vec<serde_json::Value>,
}

// ============================================================================
// SceneDecomposer
// ============================================================================

/// Orchestrates the decomposition of a .blend scene into individual assets.
///
/// # Example
///
/// ```no_run
/// use astraweave_blend::decomposer::SceneDecomposer;
/// use astraweave_blend::options::ConversionOptions;
/// use astraweave_blend::discovery::BlenderInstallation;
///
/// # async fn example(installation: BlenderInstallation) -> anyhow::Result<()> {
/// let options = ConversionOptions::scene_decomposition();
/// let decomposer = SceneDecomposer::new(
///     "scene.blend",
///     "output/scene_assets",
///     options,
///     installation,
/// );
/// let result = decomposer.execute().await?;
/// println!("Extracted {} assets", result.assets.len());
/// # Ok(())
/// # }
/// ```
pub struct SceneDecomposer {
    source_path: PathBuf,
    output_dir: PathBuf,
    options: ConversionOptions,
    installation: BlenderInstallation,
    progress: Arc<ProgressTracker>,
    cancellation: CancellationToken,
    stdout_buffer: Arc<Mutex<String>>,
    stderr_buffer: Arc<Mutex<String>>,
}

impl SceneDecomposer {
    /// Creates a new decomposer.
    pub fn new(
        source_path: impl Into<PathBuf>,
        output_dir: impl Into<PathBuf>,
        options: ConversionOptions,
        installation: BlenderInstallation,
    ) -> Self {
        let progress = Arc::new(ProgressTracker::new());
        let cancellation = progress.cancellation_token();

        Self {
            source_path: source_path.into(),
            output_dir: output_dir.into(),
            options,
            installation,
            progress,
            cancellation,
            stdout_buffer: Arc::new(Mutex::new(String::new())),
            stderr_buffer: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Returns a progress tracker subscription.
    pub fn progress(&self) -> Arc<ProgressTracker> {
        self.progress.clone()
    }

    /// Returns the cancellation token.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    /// Executes the decomposition.
    pub async fn execute(self) -> BlendResult<DecompositionResult> {
        let start = Instant::now();

        self.progress.set_stage(ConversionStage::Initializing);
        self.progress.set_message("Preparing decomposition...");

        // Validate source
        if !self.source_path.exists() {
            return Err(BlendError::BlendFileNotFound {
                path: self.source_path.clone(),
            });
        }

        if self.cancellation.is_cancelled() {
            return Err(BlendError::Cancelled);
        }

        // Create output directory
        tokio::fs::create_dir_all(&self.output_dir)
            .await
            .map_err(BlendError::IoError)?;

        // Compute source hash
        let source_hash = ConversionCache::hash_file(&self.source_path)?;

        // Generate decomposition script
        let script_content = generate_decomposition_script(
            &self.source_path,
            &self.output_dir,
            &self.options,
            &source_hash,
        );

        let temp_dir = TempDir::new().map_err(BlendError::IoError)?;
        let script_path = temp_dir.path().join("decompose_script.py");
        tokio::fs::write(&script_path, &script_content)
            .await
            .map_err(BlendError::IoError)?;

        debug!(
            "Decomposition script written to: {}",
            script_path.display()
        );

        // Run Blender
        self.progress.set_stage(ConversionStage::LoadingBlendFile);
        self.progress
            .set_message("Starting Blender for scene decomposition...");

        self.run_blender(&script_path).await?;

        // Parse result
        let result_path = self.output_dir.join("decomposition_result.json");
        let raw = self.parse_result(&result_path).await?;

        if !raw.success {
            let msg = raw.error.unwrap_or_else(|| "Unknown error".to_string());
            let tb = raw.traceback.unwrap_or_default();
            return Err(BlendError::ConversionFailed {
                message: msg,
                exit_code: None,
                stderr: String::new(),
                blender_output: Some(tb),
            });
        }

        // Log per-object export errors from Python
        if !raw.export_errors.is_empty() {
            warn!(
                "{} objects failed to export in Blender",
                raw.export_errors.len()
            );
            for err in &raw.export_errors {
                let name = err.get("name").and_then(|n| n.as_str()).unwrap_or("<unknown>");
                let msg = err.get("error").and_then(|e| e.as_str()).unwrap_or("<no message>");
                warn!("  Export failed: '{name}': {msg}");
            }
        }

        // Parse assets — log deserialization failures instead of silently dropping
        let raw_asset_count = raw.assets.len();
        let mut assets: Vec<DecomposedAsset> = Vec::with_capacity(raw_asset_count);
        for (i, v) in raw.assets.into_iter().enumerate() {
            match serde_json::from_value::<DecomposedAsset>(v.clone()) {
                Ok(a) => assets.push(a),
                Err(e) => {
                    let name = v.get("name").and_then(|n| n.as_str()).unwrap_or("<unknown>");
                    warn!(
                        "Asset {i} '{name}' failed to deserialize: {e}. JSON: {}",
                        serde_json::to_string(&v).unwrap_or_default()
                    );
                }
            }
        }
        if assets.len() < raw_asset_count {
            warn!(
                "Deserialization dropped {} of {} assets",
                raw_asset_count - assets.len(),
                raw_asset_count
            );
        }

        let empties: Vec<ExtractedEmpty> = raw
            .empties
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();

        let hdris: Vec<ExtractedHdri> = raw
            .hdris
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();

        let manifest_path = if self.options.decomposition.generate_manifest {
            let mp = self.output_dir.join("manifest.json");
            if mp.exists() {
                Some(mp)
            } else {
                None
            }
        } else {
            None
        };

        let textures_dir = raw.textures_dir.map(PathBuf::from).and_then(|p| {
            if p.exists() {
                Some(p)
            } else {
                None
            }
        });

        info!(
            "Decomposition complete: {} assets, {} HDRIs in {:?}",
            assets.len(),
            hdris.len(),
            start.elapsed(),
        );

        self.progress.complete();

        Ok(DecompositionResult {
            output_dir: self.output_dir.clone(),
            assets,
            empties,
            hdris,
            total_objects: raw.total_objects,
            manifest_path,
            textures_dir,
            duration: start.elapsed(),
            blender_version: self.installation.version.to_string(),
        })
    }

    async fn run_blender(&self, script_path: &Path) -> BlendResult<()> {
        let mut cmd = Command::new(&self.installation.executable_path);

        cmd.arg("--background")
            .arg("--python")
            .arg(script_path)
            .arg("--")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for arg in &self.options.process.extra_blender_args {
            cmd.arg(arg);
        }

        for (key, value) in &self.options.process.environment {
            cmd.env(key, value);
        }

        if self.options.process.threads > 0 {
            cmd.arg("--threads")
                .arg(self.options.process.threads.to_string());
        }

        if let Some(ref work_dir) = self.options.process.working_directory {
            cmd.current_dir(work_dir);
        }

        debug!(
            "Running Blender decomposition: {:?} --background --python {:?}",
            self.installation.executable_path, script_path
        );

        let mut child =
            cmd.spawn()
                .map_err(|e| BlendError::BlenderExecutionFailed {
                    path: self.installation.executable_path.clone(),
                    reason: format!("Failed to spawn Blender: {}", e),
                })?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let stdout_buf = self.stdout_buffer.clone();
        let stderr_buf = self.stderr_buffer.clone();
        let progress = self.progress.clone();

        let stdout_task = tokio::spawn(async move {
            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if line.contains("Exporting:") {
                        progress.set_stage(ConversionStage::ExportingMeshes);
                    } else if line.contains("Manifest written") {
                        progress.set_stage(ConversionStage::WritingOutput);
                    }
                    progress.set_message(&line);
                    stdout_buf.lock().await.push_str(&line);
                    stdout_buf.lock().await.push('\n');
                }
            }
        });

        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    stderr_buf.lock().await.push_str(&line);
                    stderr_buf.lock().await.push('\n');
                }
            }
        });

        let process_timeout = self.options.process.timeout;

        let wait_result = timeout(process_timeout, child.wait()).await;

        let _ = tokio::join!(stdout_task, stderr_task);

        match wait_result {
            Ok(Ok(status)) => {
                if status.success() {
                    Ok(())
                } else {
                    let stderr = self.stderr_buffer.lock().await.clone();
                    Err(BlendError::ConversionFailed {
                        message: format!("Blender exited with status: {}", status),
                        exit_code: status.code(),
                        stderr,
                        blender_output: Some(self.stdout_buffer.lock().await.clone()),
                    })
                }
            }
            Ok(Err(e)) => Err(BlendError::BlenderExecutionFailed {
                path: self.installation.executable_path.clone(),
                reason: format!("Blender process error: {}", e),
            }),
            Err(_) => Err(BlendError::Timeout {
                operation: "scene decomposition".to_string(),
                duration: process_timeout,
                path: self.source_path.clone(),
                timeout_secs: process_timeout.as_secs(),
            }),
        }
    }

    async fn parse_result(&self, result_path: &Path) -> BlendResult<RawDecompResult> {
        if !result_path.exists() {
            return Err(BlendError::OutputNotProduced {
                expected_path: result_path.to_path_buf(),
            });
        }

        let content = tokio::fs::read_to_string(result_path)
            .await
            .map_err(BlendError::IoError)?;

        serde_json::from_str(&content).map_err(|e| BlendError::ConversionFailed {
            message: format!("Failed to parse decomposition result: {}", e),
            exit_code: None,
            stderr: String::new(),
            blender_output: Some(content),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decomposed_asset_serde_roundtrip() {
        let asset = DecomposedAsset {
            name: "Boulder_01".to_string(),
            filename: "meshes/Boulder_01.glb".to_string(),
            category: "rock".to_string(),
            vertex_count: 1200,
            file_size: 48000,
            bounds: Some(AssetBounds {
                min: [-1.0, -1.0, 0.0],
                max: [1.0, 1.0, 2.0],
            }),
            dimensions: Some([2.0, 2.0, 2.0]),
            position: [10.0, 5.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            textures: vec![AssetTexture {
                filename: "boulder_01_diff.png".to_string(),
                channel: "diffuse".to_string(),
                original_name: "boulder_01_diff".to_string(),
                width: 2048,
                height: 2048,
            }],
            materials: vec!["Boulder_Mat".to_string()],
            collections: vec!["Rocks".to_string()],
        };

        let json = serde_json::to_string(&asset).unwrap();
        let parsed: DecomposedAsset = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Boulder_01");
        assert_eq!(parsed.category, "rock");
        assert_eq!(parsed.textures.len(), 1);
    }

    #[test]
    fn test_decomposition_result_serde() {
        let result = DecompositionResult {
            output_dir: PathBuf::from("/output/scene"),
            assets: vec![],
            empties: vec![],
            hdris: vec![],
            total_objects: 0,
            manifest_path: None,
            textures_dir: None,
            duration: std::time::Duration::from_secs(5),
            blender_version: "4.2.0".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: DecompositionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.blender_version, "4.2.0");
    }
}
