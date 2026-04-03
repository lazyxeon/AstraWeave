//! Blender subprocess conversion engine.
//!
//! This module handles spawning Blender as a subprocess to execute
//! export scripts, with proper timeout handling and cancellation support.

use crate::cache::{CacheLookup, ConversionCache};
use crate::discovery::BlenderInstallation;
use crate::error::{BlendError, BlendResult};
use crate::export_script::generate_export_script;
use crate::options::ConversionOptions;
use crate::progress::{CancellationToken, ConversionStage, ProgressTracker};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Result of a conversion operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    /// Path to the output glTF/GLB file.
    pub output_path: PathBuf,
    /// Size of the output file in bytes.
    pub output_size: u64,
    /// Duration of the conversion.
    pub duration: Duration,
    /// Whether result came from cache.
    pub from_cache: bool,
    /// Blender version used.
    pub blender_version: String,
    /// Any texture files generated.
    pub texture_files: Vec<PathBuf>,
    /// Linked libraries that were processed.
    pub linked_libraries: Vec<PathBuf>,
    /// Blender stdout output (if captured).
    pub stdout: Option<String>,
    /// Blender stderr output (if captured).
    pub stderr: Option<String>,
}

/// Blender export result from the Python script.
#[derive(Debug, Deserialize)]
struct BlenderExportResult {
    success: bool,
    error: Option<String>,
    traceback: Option<String>,
    output: Option<BlenderExportOutput>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for deserialization
struct BlenderExportOutput {
    output_file: String,
    file_size: u64,
    format: String,
}

/// A conversion job that can be executed.
pub struct ConversionJob {
    /// Source .blend file path.
    source_path: PathBuf,
    /// Output file path.
    output_path: PathBuf,
    /// Conversion options.
    options: ConversionOptions,
    /// Blender installation to use.
    installation: BlenderInstallation,
    /// Progress tracker.
    progress: Arc<ProgressTracker>,
    /// Cancellation token.
    cancellation: CancellationToken,
    /// Temporary directory for export script.
    temp_dir: Option<TempDir>,
    /// Cached stdout.
    stdout_buffer: Arc<Mutex<String>>,
    /// Cached stderr.
    stderr_buffer: Arc<Mutex<String>>,
}

impl ConversionJob {
    /// Creates a new conversion job.
    pub fn new(
        source_path: impl Into<PathBuf>,
        output_path: impl Into<PathBuf>,
        options: ConversionOptions,
        installation: BlenderInstallation,
    ) -> Self {
        let progress = Arc::new(ProgressTracker::new());
        let cancellation = progress.cancellation_token();

        Self {
            source_path: source_path.into(),
            output_path: output_path.into(),
            options,
            installation,
            progress,
            cancellation,
            temp_dir: None,
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

    /// Executes the conversion with optional caching.
    pub async fn execute(
        &mut self,
        mut cache: Option<&mut ConversionCache>,
    ) -> BlendResult<ConversionResult> {
        let start_time = Instant::now();

        self.progress.set_stage(ConversionStage::Initializing);
        self.progress.set_message("Checking cache...");

        // Check cache first
        if let Some(ref mut c) = cache {
            if self.options.cache.enabled {
                match c.lookup(&self.source_path, &self.options, &self.installation.version) {
                    Ok(CacheLookup::Hit { output_path, entry }) => {
                        info!("Cache hit for: {}", self.source_path.display());
                        self.progress.set_stage(ConversionStage::Completed);

                        return Ok(ConversionResult {
                            output_path,
                            output_size: entry.output_size,
                            duration: start_time.elapsed(),
                            from_cache: true,
                            blender_version: entry.blender_version.to_string(),
                            texture_files: entry
                                .texture_files
                                .iter()
                                .map(|p| c.cache_dir().join(p))
                                .collect(),
                            linked_libraries: entry.linked_libraries,
                            stdout: None,
                            stderr: None,
                        });
                    }
                    Ok(CacheLookup::Miss { reason }) => {
                        debug!("Cache miss ({}): {}", reason, self.source_path.display());
                    }
                    Err(e) => {
                        warn!("Cache lookup failed: {}", e);
                    }
                }
            }
        }

        // Check for cancellation
        if self.cancellation.is_cancelled() {
            return Err(BlendError::Cancelled);
        }

        // Validate source file exists
        if !self.source_path.exists() {
            return Err(BlendError::BlendFileNotFound {
                path: self.source_path.clone(),
            });
        }

        // Create temp directory for script
        self.progress.set_stage(ConversionStage::Initializing);
        self.progress.set_message("Preparing export script...");

        let temp_dir = TempDir::new().map_err(BlendError::IoError)?;
        self.temp_dir = Some(temp_dir);

        let temp_path = self.temp_dir.as_ref().unwrap().path();

        // Compute source hash for deterministic naming
        let source_hash = ConversionCache::hash_file(&self.source_path)?;

        // Generate export script
        let script_content = generate_export_script(
            &self.source_path,
            &self.output_path,
            &self.options,
            &source_hash,
        );

        let script_path = temp_path.join("export_script.py");
        tokio::fs::write(&script_path, &script_content)
            .await
            .map_err(BlendError::IoError)?;

        debug!("Export script written to: {}", script_path.display());

        // Check for cancellation
        if self.cancellation.is_cancelled() {
            return Err(BlendError::Cancelled);
        }

        // Execute Blender
        self.progress.set_stage(ConversionStage::LoadingBlendFile);
        self.progress.set_message("Starting Blender...");

        self.run_blender(&script_path).await?;

        // Parse result
        let result_path = PathBuf::from(format!("{}.result.json", self.output_path.display()));
        let export_result = self.parse_blender_result(&result_path).await?;

        if !export_result.success {
            let error_msg = export_result
                .error
                .unwrap_or_else(|| "Unknown error".to_string());
            let traceback = export_result.traceback.unwrap_or_default();

            return Err(BlendError::ConversionFailed {
                message: error_msg,
                exit_code: None,
                stderr: String::new(),
                blender_output: Some(traceback),
            });
        }

        let output_info = export_result
            .output
            .ok_or_else(|| BlendError::ConversionFailed {
                message: "No output information from Blender".to_string(),
                exit_code: None,
                stderr: String::new(),
                blender_output: None,
            })?;

        // Validate the GLB output file
        self.progress
            .set_message("Validating GLB output integrity...");
        validate_glb(&self.output_path).await?;

        // Collect texture files
        let texture_files = self
            .collect_texture_files(
                self.output_path.parent().unwrap_or(Path::new(".")),
                &source_hash,
            )
            .await;

        let conversion_duration = start_time.elapsed();

        let stdout = self.stdout_buffer.lock().await.clone();
        let stderr = self.stderr_buffer.lock().await.clone();

        // Extract linked libraries from Blender output
        let linked_libraries = self.extract_linked_libraries(&stdout, &stderr);

        // Store in cache
        if let Some(ref mut c) = cache {
            if self.options.cache.enabled {
                let _ = c.store(
                    &self.source_path,
                    &self.output_path,
                    &self.options,
                    &self.installation.version,
                    conversion_duration.as_millis() as u64,
                    texture_files.clone(),
                    linked_libraries.clone(),
                );
            }
        }

        self.progress.complete();

        Ok(ConversionResult {
            output_path: self.output_path.clone(),
            output_size: output_info.file_size,
            duration: conversion_duration,
            from_cache: false,
            blender_version: self.installation.version.to_string(),
            texture_files,
            linked_libraries,
            stdout: if stdout.is_empty() {
                None
            } else {
                Some(stdout)
            },
            stderr: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
        })
    }

    /// Runs Blender with the export script.
    async fn run_blender(&self, script_path: &Path) -> BlendResult<()> {
        let mut cmd = Command::new(&self.installation.executable_path);

        // Configure Blender command
        cmd.arg("--background") // No UI
            .arg("--python")
            .arg(script_path)
            .arg("--")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add extra arguments
        for arg in &self.options.process.extra_blender_args {
            cmd.arg(arg);
        }

        // Set environment variables
        for (key, value) in &self.options.process.environment {
            cmd.env(key, value);
        }

        // Set thread count
        if self.options.process.threads > 0 {
            cmd.arg("--threads")
                .arg(self.options.process.threads.to_string());
        }

        // Set working directory
        if let Some(ref work_dir) = self.options.process.working_directory {
            cmd.current_dir(work_dir);
        }

        debug!(
            "Running Blender: {:?} --background --python {:?}",
            self.installation.executable_path, script_path
        );

        // Spawn process
        let mut child = cmd
            .spawn()
            .map_err(|e| BlendError::BlenderExecutionFailed {
                path: self.installation.executable_path.clone(),
                reason: format!("Failed to spawn Blender: {}", e),
            })?;

        // Capture output streams
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let stdout_buffer = self.stdout_buffer.clone();
        let stderr_buffer = self.stderr_buffer.clone();
        let progress = self.progress.clone();

        // Stream stdout
        let stdout_task = tokio::spawn(async move {
            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    // Parse progress from Blender output
                    if line.contains("Loading:") {
                        progress.set_stage(ConversionStage::LoadingBlendFile);
                    } else if line.contains("Processing linked") {
                        progress.set_stage(ConversionStage::ProcessingLinkedLibraries);
                    } else if line.contains("Exporting meshes") || line.contains("ExportingMeshes")
                    {
                        progress.set_stage(ConversionStage::ExportingMeshes);
                    } else if line.contains("Export complete") {
                        progress.set_stage(ConversionStage::WritingOutput);
                    }

                    progress.set_message(&line);
                    stdout_buffer.lock().await.push_str(&line);
                    stdout_buffer.lock().await.push('\n');
                }
            }
        });

        // Stream stderr
        let stderr_task = tokio::spawn(async move {
            if let Some(stderr) = stderr {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    stderr_buffer.lock().await.push_str(&line);
                    stderr_buffer.lock().await.push('\n');
                }
            }
        });

        // Wait with timeout
        let process_timeout = self.options.process.timeout;
        let wait_result = self
            .wait_with_cancellation(&mut child, process_timeout)
            .await;

        // Ensure output tasks complete
        let _ = tokio::join!(stdout_task, stderr_task);

        match wait_result {
            Ok(status) => {
                if status.success() {
                    Ok(())
                } else {
                    let stderr = self.stderr_buffer.lock().await.clone();
                    Err(BlendError::ConversionFailed {
                        message: format!("Blender exited with status: {}", status),
                        exit_code: status.code(),
                        stderr: stderr.clone(),
                        blender_output: if stderr.is_empty() {
                            None
                        } else {
                            Some(stderr)
                        },
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Waits for process with timeout and cancellation support.
    async fn wait_with_cancellation(
        &self,
        child: &mut Child,
        process_timeout: Duration,
    ) -> BlendResult<std::process::ExitStatus> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(100);

        loop {
            // Check cancellation
            if self.cancellation.is_cancelled() {
                let _ = child.kill().await;
                self.progress.mark_cancelled();
                return Err(BlendError::Cancelled);
            }

            // Check timeout
            if start.elapsed() > process_timeout {
                let _ = child.kill().await;
                return Err(BlendError::Timeout {
                    operation: "Blender conversion".to_string(),
                    duration: process_timeout,
                    path: self.source_path.clone(),
                    timeout_secs: process_timeout.as_secs(),
                });
            }

            // Try to get exit status
            match timeout(poll_interval, child.wait()).await {
                Ok(Ok(status)) => return Ok(status),
                Ok(Err(e)) => {
                    return Err(BlendError::BlenderExecutionFailed {
                        path: self.installation.executable_path.clone(),
                        reason: format!("Process wait failed: {}", e),
                    });
                }
                Err(_) => {
                    // Timeout on poll, continue loop
                    continue;
                }
            }
        }
    }

    /// Parses the Blender result JSON file.
    async fn parse_blender_result(&self, result_path: &Path) -> BlendResult<BlenderExportResult> {
        if !result_path.exists() {
            return Err(BlendError::ConversionFailed {
                message: "Blender did not produce result file".to_string(),
                exit_code: None,
                stderr: String::new(),
                blender_output: Some(self.stderr_buffer.lock().await.clone()),
            });
        }

        let content = tokio::fs::read_to_string(result_path).await.map_err(|e| {
            BlendError::FileReadError {
                path: result_path.to_path_buf(),
                message: format!("Failed to read result: {}", e),
                source: e,
            }
        })?;

        let result: BlenderExportResult =
            serde_json::from_str(&content).map_err(|e| BlendError::ConversionFailed {
                message: format!("Failed to parse Blender result: {}", e),
                exit_code: None,
                stderr: String::new(),
                blender_output: Some(content),
            })?;

        Ok(result)
    }

    /// Collects generated texture files.
    async fn collect_texture_files(&self, output_dir: &Path, source_hash: &str) -> Vec<PathBuf> {
        let mut textures = Vec::new();

        if let Ok(mut entries) = tokio::fs::read_dir(output_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Match texture files generated with deterministic naming
                    if name.starts_with(source_hash) {
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                        if matches!(ext, "png" | "jpg" | "jpeg" | "webp" | "tga" | "bmp") {
                            textures.push(path);
                        }
                    }
                }
            }
        }

        textures
    }

    /// Extracts linked library paths from Blender output.
    ///
    /// Parses stdout/stderr for library loading messages like:
    /// - "Read library: '/path/to/library.blend'"
    /// - "lib_link_main: library loaded: '/path/to/library.blend'"
    fn extract_linked_libraries(&self, stdout: &str, stderr: &str) -> Vec<PathBuf> {
        use std::collections::HashSet;

        let mut libraries = HashSet::new();

        // Patterns that indicate linked library loading in Blender output
        // Blender logs library loading with messages like:
        // "Read library:  '/path/to/lib.blend'"
        // "lib_link_main: 'path/to/lib.blend'"
        let combined_output = format!("{}\n{}", stdout, stderr);

        for line in combined_output.lines() {
            // Pattern 1: "Read library: '/path/to/file.blend'"
            if line.contains("Read library:") || line.contains("lib_link_main:") {
                if let Some(start) = line.find('\'') {
                    if let Some(end) = line[start + 1..].find('\'') {
                        let lib_path = &line[start + 1..start + 1 + end];
                        if lib_path.ends_with(".blend") {
                            libraries.insert(PathBuf::from(lib_path));
                        }
                    }
                }
            }
            // Pattern 2: Double-quoted paths
            if line.contains("Read library:") || line.contains("lib_link_main:") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        let lib_path = &line[start + 1..start + 1 + end];
                        if lib_path.ends_with(".blend") {
                            libraries.insert(PathBuf::from(lib_path));
                        }
                    }
                }
            }
        }

        libraries.into_iter().collect()
    }
}

/// Builder for creating conversion jobs.
pub struct ConversionJobBuilder {
    source_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    options: ConversionOptions,
    installation: Option<BlenderInstallation>,
}

impl ConversionJobBuilder {
    /// Creates a new builder with default options.
    pub fn new() -> Self {
        Self {
            source_path: None,
            output_path: None,
            options: ConversionOptions::default(),
            installation: None,
        }
    }

    /// Sets the source .blend file path.
    pub fn source(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Sets the output file path.
    pub fn output(mut self, path: impl Into<PathBuf>) -> Self {
        self.output_path = Some(path.into());
        self
    }

    /// Sets the conversion options.
    pub fn options(mut self, options: ConversionOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the Blender installation to use.
    pub fn installation(mut self, installation: BlenderInstallation) -> Self {
        self.installation = Some(installation);
        self
    }

    /// Builds the conversion job.
    pub fn build(self) -> BlendResult<ConversionJob> {
        let source_path = self
            .source_path
            .ok_or_else(|| BlendError::ConfigurationError {
                message: "Source path not specified".to_string(),
            })?;

        let output_path = self.output_path.unwrap_or_else(|| {
            let stem = source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            source_path.with_file_name(format!("{}.{}", stem, self.options.format.extension()))
        });

        let installation = self
            .installation
            .ok_or_else(|| BlendError::BlenderNotFound {
                searched_paths: Vec::new(),
            })?;

        Ok(ConversionJob::new(
            source_path,
            output_path,
            self.options,
            installation,
        ))
    }
}

impl Default for ConversionJobBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates a GLB file's structural integrity.
///
/// Checks:
/// 1. File exists and is readable
/// 2. GLB magic bytes (`glTF` = 0x46546C67)
/// 3. GLB version (must be 2)
/// 4. Total length matches file size
/// 5. JSON chunk present with valid header
/// 6. Binary chunk length doesn't exceed file bounds
async fn validate_glb(path: &Path) -> BlendResult<()> {
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(path).await.map_err(|e| {
        BlendError::ConversionFailed {
            message: format!("Cannot open GLB output for validation: {e}"),
            exit_code: None,
            stderr: String::new(),
            blender_output: None,
        }
    })?;

    let file_size = file
        .metadata()
        .await
        .map_err(BlendError::IoError)?
        .len();

    if file_size < 12 {
        return Err(BlendError::ConversionFailed {
            message: format!("GLB file too small ({file_size} bytes, minimum 12)"),
            exit_code: None,
            stderr: String::new(),
            blender_output: None,
        });
    }

    // Read the 12-byte GLB header
    let mut header = [0u8; 12];
    file.read_exact(&mut header).await.map_err(BlendError::IoError)?;

    // Check magic bytes: 0x46546C67 = "glTF" in little-endian
    let magic = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    if magic != 0x46546C67 {
        return Err(BlendError::ConversionFailed {
            message: format!(
                "Invalid GLB magic bytes: expected 0x46546C67 (glTF), got 0x{magic:08X}"
            ),
            exit_code: None,
            stderr: String::new(),
            blender_output: None,
        });
    }

    // Check version (must be 2)
    let version = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
    if version != 2 {
        return Err(BlendError::ConversionFailed {
            message: format!("Unsupported GLB version: {version} (expected 2)"),
            exit_code: None,
            stderr: String::new(),
            blender_output: None,
        });
    }

    // Check total length matches file size
    let total_length = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
    if u64::from(total_length) != file_size {
        return Err(BlendError::ConversionFailed {
            message: format!(
                "GLB length mismatch: header says {total_length} bytes, file is {file_size} bytes"
            ),
            exit_code: None,
            stderr: String::new(),
            blender_output: None,
        });
    }

    // Read JSON chunk header (8 bytes: length + type)
    if file_size < 20 {
        return Err(BlendError::ConversionFailed {
            message: "GLB file too small for JSON chunk header".to_string(),
            exit_code: None,
            stderr: String::new(),
            blender_output: None,
        });
    }

    let mut chunk_header = [0u8; 8];
    file.read_exact(&mut chunk_header)
        .await
        .map_err(BlendError::IoError)?;

    let json_chunk_length = u32::from_le_bytes([
        chunk_header[0],
        chunk_header[1],
        chunk_header[2],
        chunk_header[3],
    ]);
    let json_chunk_type = u32::from_le_bytes([
        chunk_header[4],
        chunk_header[5],
        chunk_header[6],
        chunk_header[7],
    ]);

    // JSON chunk type must be 0x4E4F534A = "JSON" in little-endian
    if json_chunk_type != 0x4E4F534A {
        return Err(BlendError::ConversionFailed {
            message: format!(
                "First GLB chunk is not JSON: expected 0x4E4F534A, got 0x{json_chunk_type:08X}"
            ),
            exit_code: None,
            stderr: String::new(),
            blender_output: None,
        });
    }

    // Check JSON chunk doesn't exceed file bounds
    let json_end = 12u64 + 8 + u64::from(json_chunk_length);
    if json_end > file_size {
        return Err(BlendError::ConversionFailed {
            message: format!(
                "JSON chunk exceeds file bounds: chunk ends at byte {json_end}, file is {file_size} bytes"
            ),
            exit_code: None,
            stderr: String::new(),
            blender_output: None,
        });
    }

    // If there's a binary chunk, validate its header too
    if json_end + 8 <= file_size {
        // Seek past JSON data to the binary chunk header
        let json_data_remaining = json_chunk_length as usize;
        let mut skip_buf = vec![0u8; json_data_remaining.min(65536)];
        let mut remaining = json_data_remaining;
        while remaining > 0 {
            let to_read = remaining.min(skip_buf.len());
            file.read_exact(&mut skip_buf[..to_read])
                .await
                .map_err(BlendError::IoError)?;
            remaining -= to_read;
        }

        let mut bin_header = [0u8; 8];
        file.read_exact(&mut bin_header)
            .await
            .map_err(BlendError::IoError)?;

        let bin_chunk_length = u32::from_le_bytes([
            bin_header[0],
            bin_header[1],
            bin_header[2],
            bin_header[3],
        ]);
        let bin_chunk_type = u32::from_le_bytes([
            bin_header[4],
            bin_header[5],
            bin_header[6],
            bin_header[7],
        ]);

        // Binary chunk type must be 0x004E4942 = "BIN\0" in little-endian
        if bin_chunk_type != 0x004E4942 {
            warn!(
                "Second GLB chunk is not BIN: got 0x{:08X} (may be valid extension chunk)",
                bin_chunk_type
            );
        }

        let bin_end = json_end + 8 + u64::from(bin_chunk_length);
        if bin_end > file_size {
            return Err(BlendError::ConversionFailed {
                message: format!(
                    "Binary chunk exceeds file bounds: chunk ends at byte {bin_end}, file is {file_size} bytes"
                ),
                exit_code: None,
                stderr: String::new(),
                blender_output: None,
            });
        }
    }

    debug!(
        "GLB validation passed: {} ({file_size} bytes, version {version})",
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::DiscoveryMethod;
    use crate::version::BlenderVersion;

    fn mock_installation() -> BlenderInstallation {
        BlenderInstallation {
            executable_path: PathBuf::from("/usr/bin/blender"),
            version: BlenderVersion::new(4, 0, 0),
            discovery_method: DiscoveryMethod::SystemPath,
            install_dir: PathBuf::from("/usr/bin"),
        }
    }

    #[test]
    fn test_conversion_job_builder() {
        let result = ConversionJobBuilder::new()
            .source("/test/model.blend")
            .output("/output/model.glb")
            .installation(mock_installation())
            .build();

        assert!(result.is_ok());
        let job = result.unwrap();
        assert_eq!(job.source_path, PathBuf::from("/test/model.blend"));
        assert_eq!(job.output_path, PathBuf::from("/output/model.glb"));
    }

    #[test]
    fn test_conversion_job_builder_auto_output() {
        let result = ConversionJobBuilder::new()
            .source("/test/model.blend")
            .installation(mock_installation())
            .build();

        assert!(result.is_ok());
        let job = result.unwrap();
        assert!(job.output_path.to_string_lossy().contains("model.glb"));
    }

    #[test]
    fn test_conversion_job_builder_missing_source() {
        let result = ConversionJobBuilder::new()
            .installation(mock_installation())
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_conversion_job_builder_missing_installation() {
        let result = ConversionJobBuilder::new()
            .source("/test/model.blend")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_conversion_result_from_cache() {
        let result = ConversionResult {
            output_path: PathBuf::from("/cache/model.glb"),
            output_size: 1000,
            duration: Duration::from_millis(10),
            from_cache: true,
            blender_version: "4.0.0".to_string(),
            texture_files: vec![],
            linked_libraries: vec![],
            stdout: None,
            stderr: None,
        };

        assert!(result.from_cache);
        assert!(result.duration < Duration::from_secs(1));
    }

    #[test]
    fn test_extract_linked_libraries_single_quoted() {
        let job = ConversionJob::new(
            "/test/model.blend",
            "/output/model.glb",
            ConversionOptions::default(),
            mock_installation(),
        );

        let stdout = "Info: Read library:  '/path/to/library.blend', './meshes.blend'";
        let stderr = "";

        let libs = job.extract_linked_libraries(stdout, stderr);
        assert!(libs
            .iter()
            .any(|p| p.to_string_lossy().contains("library.blend")));
    }

    #[test]
    fn test_extract_linked_libraries_double_quoted() {
        let job = ConversionJob::new(
            "/test/model.blend",
            "/output/model.glb",
            ConversionOptions::default(),
            mock_installation(),
        );

        let stdout = "";
        let stderr = r#"lib_link_main: "/assets/materials.blend" loaded successfully"#;

        let libs = job.extract_linked_libraries(stdout, stderr);
        assert!(libs
            .iter()
            .any(|p| p.to_string_lossy().contains("materials.blend")));
    }

    #[test]
    fn test_extract_linked_libraries_no_libraries() {
        let job = ConversionJob::new(
            "/test/model.blend",
            "/output/model.glb",
            ConversionOptions::default(),
            mock_installation(),
        );

        let stdout = "Info: Exporting scene\nInfo: Export complete";
        let stderr = "";

        let libs = job.extract_linked_libraries(stdout, stderr);
        assert!(libs.is_empty());
    }

    // --- Mutation kill tests for extract_linked_libraries line 534/545 ---
    // Verify EXACT path extraction (no leading/trailing quotes or off-by-one)
    #[test]
    fn extract_linked_libraries_exact_single_quote_path() {
        let job = ConversionJob::new(
            "/test/m.blend",
            "/out/m.glb",
            ConversionOptions::default(),
            mock_installation(),
        );
        let stdout = "Read library: '/exact/path.blend'";
        let libs = job.extract_linked_libraries(stdout, "");
        assert_eq!(libs.len(), 1);
        let p = libs[0].to_string_lossy().to_string();
        // Must NOT start with a quote character — catches start+1→start-1/start*1
        assert!(!p.starts_with('\''), "path must not start with quote: {p}");
        assert_eq!(p, "/exact/path.blend");
    }

    #[test]
    fn extract_linked_libraries_exact_double_quote_path() {
        let job = ConversionJob::new(
            "/test/m.blend",
            "/out/m.glb",
            ConversionOptions::default(),
            mock_installation(),
        );
        let stderr = r#"lib_link_main: "/exact/double.blend" loaded"#;
        let libs = job.extract_linked_libraries("", stderr);
        assert_eq!(libs.len(), 1);
        let p = libs[0].to_string_lossy().to_string();
        assert!(
            !p.starts_with('"'),
            "path must not start with double-quote: {p}"
        );
        assert_eq!(p, "/exact/double.blend");
    }
}
