use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

/// Download manager with resume support and integrity checks
pub struct Downloader {
    client: reqwest::Client,
    max_retries: u32,
    retry_delay_ms: u64,
    max_concurrent: usize,
}

/// Download task description
pub struct DownloadTask {
    pub url: String,
    pub dest_path: PathBuf,
    pub key: String,
}

/// Download result with hash
#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub path: PathBuf,
    pub sha256: String,
    pub size: u64,
}

impl Downloader {
    /// Create new downloader
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("AstraWeave-Assets/0.1.0")
            .timeout(std::time::Duration::from_secs(300)) // 5 min for large files
            .build()?;

        Ok(Self {
            client,
            max_retries: 3,
            retry_delay_ms: 1000,
            max_concurrent: 8, // Default: 8 concurrent downloads
        })
    }

    /// Set maximum concurrent downloads
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Download file with progress bar and integrity check
    pub async fn download(
        &self,
        url: &str,
        dest_path: &Path,
        show_progress: bool,
    ) -> Result<DownloadResult> {
        // Create parent directory
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Temp file for atomic write
        let temp_path = dest_path.with_extension("tmp");

        // Retry loop with exponential backoff
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay_ms = self.retry_delay_ms * 2u64.pow(attempt - 1);
                eprintln!(
                    "⏳ Retry {}/{} for {} (waiting {}ms)",
                    attempt, self.max_retries, url, delay_ms
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }

            match self.download_attempt(url, &temp_path, show_progress).await {
                Ok(result) => {
                    // Atomic move to final destination
                    fs::rename(&temp_path, dest_path)
                        .await
                        .context("Failed to move downloaded file")?;

                    return Ok(DownloadResult {
                        path: dest_path.to_path_buf(),
                        sha256: result.sha256,
                        size: result.size,
                    });
                }
                Err(e) => {
                    last_error = Some(e);
                    // Clean up temp file on error
                    let _ = fs::remove_file(&temp_path).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Download failed after retries")))
    }

    /// Download multiple files in parallel with concurrency limit
    pub async fn download_parallel(
        &self,
        tasks: Vec<DownloadTask>,
        show_progress: bool,
    ) -> Result<Vec<(String, Result<DownloadResult>)>> {
        if tasks.is_empty() {
            return Ok(Vec::new());
        }

        // Create semaphore to limit concurrency
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        // Create multi-progress bar (only if showing progress)
        let multi_progress = if show_progress {
            Some(Arc::new(MultiProgress::new()))
        } else {
            None
        };

        // Spawn tasks
        let mut handles = Vec::new();

        for task in tasks {
            let semaphore = semaphore.clone();
            let client = self.client.clone();
            let max_retries = self.max_retries;
            let retry_delay_ms = self.retry_delay_ms;
            let multi_progress = multi_progress.clone();

            let handle = tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore.acquire().await.expect("Semaphore closed");

                // Create progress bar
                let pb = if let Some(ref mp) = multi_progress {
                    let pb = mp.add(ProgressBar::new(0));
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template("{msg}\n{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                            .expect("Invalid progress template")
                            .progress_chars("#>-"),
                    );
                    pb.set_message(format!(
                        "Downloading {}",
                        Self::filename_from_url(&task.url)
                    ));
                    Some(pb)
                } else {
                    None
                };

                // Download with retries
                let mut last_error = None;

                for attempt in 0..=max_retries {
                    if attempt > 0 {
                        let delay_ms = retry_delay_ms * 2u64.pow(attempt - 1);
                        if let Some(ref pb) = pb {
                            pb.set_message(format!(
                                "Retry {}/{} for {}",
                                attempt, max_retries, task.key
                            ));
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    }

                    // Create temp file
                    let temp_path = task.dest_path.with_extension("tmp");

                    match Self::download_single(&client, &task.url, &temp_path, pb.as_ref()).await {
                        Ok(result) => {
                            // Atomic move
                            if let Err(e) = tokio::fs::rename(&temp_path, &task.dest_path).await {
                                last_error = Some(anyhow::anyhow!("Failed to move file: {}", e));
                                let _ = tokio::fs::remove_file(&temp_path).await;
                                continue;
                            }

                            if let Some(ref pb) = pb {
                                pb.finish_with_message(format!("✅ {}", task.key));
                            }

                            return (task.key, Ok(result));
                        }
                        Err(e) => {
                            last_error = Some(e);
                            let _ = tokio::fs::remove_file(&temp_path).await;
                        }
                    }
                }

                if let Some(ref pb) = pb {
                    pb.finish_with_message(format!("❌ {} failed", task.key));
                }

                (
                    task.key,
                    Err(last_error
                        .unwrap_or_else(|| anyhow::anyhow!("Download failed after retries"))),
                )
            });

            handles.push(handle);
        }

        // Wait for all tasks
        let mut results = Vec::new();

        for handle in handles {
            match handle.await {
                Ok((key, result)) => results.push((key, result)),
                Err(e) => {
                    return Err(anyhow::anyhow!("Task panicked: {}", e));
                }
            }
        }

        Ok(results)
    }

    /// Download single file (without retry logic, used by parallel downloader)
    async fn download_single(
        client: &reqwest::Client,
        url: &str,
        dest_path: &Path,
        pb: Option<&ProgressBar>,
    ) -> Result<DownloadResult> {
        // Create parent directory
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Send request
        let response = client
            .get(url)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error {}: {}", response.status(), url));
        }

        // Get content length
        let total_size = response.content_length().unwrap_or(0);

        // Update progress bar
        if let Some(pb) = pb {
            pb.set_length(total_size);
        }

        // Create file and hasher
        let mut file = tokio::fs::File::create(dest_path)
            .await
            .context("Failed to create file")?;
        let mut hasher = Sha256::new();
        let mut downloaded = 0u64;

        // Stream download
        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read chunk")?;

            // Write to file
            file.write_all(&chunk)
                .await
                .context("Failed to write chunk")?;

            // Update hash
            hasher.update(&chunk);

            // Update progress
            downloaded += chunk.len() as u64;
            if let Some(pb) = pb {
                pb.set_position(downloaded);
            }
        }

        // Finalize
        file.sync_all().await.context("Failed to sync file")?;

        // Compute hash
        let hash_bytes = hasher.finalize();
        let sha256 = hex::encode(hash_bytes);

        Ok(DownloadResult {
            path: dest_path.to_path_buf(),
            sha256,
            size: downloaded,
        })
    }

    /// Single download attempt
    async fn download_attempt(
        &self,
        url: &str,
        dest_path: &Path,
        show_progress: bool,
    ) -> Result<DownloadResult> {
        // Send request
        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP error {}: {}", response.status(), url));
        }

        // Get content length
        let total_size = response.content_length().unwrap_or(0);

        // Create progress bar
        let pb = if show_progress && total_size > 0 {
            let pb = ProgressBar::new(total_size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
                    .progress_chars("#>-"),
            );
            pb.set_message(format!("Downloading {}", Self::filename_from_url(url)));
            Some(pb)
        } else {
            None
        };

        // Create file and hasher
        let mut file = fs::File::create(dest_path)
            .await
            .context("Failed to create file")?;
        let mut hasher = Sha256::new();
        let mut downloaded = 0u64;

        // Stream download
        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read chunk")?;

            // Write to file
            file.write_all(&chunk)
                .await
                .context("Failed to write chunk")?;

            // Update hash
            hasher.update(&chunk);

            // Update progress
            downloaded += chunk.len() as u64;
            if let Some(pb) = &pb {
                pb.set_position(downloaded);
            }
        }

        // Finalize
        file.sync_all().await.context("Failed to sync file")?;

        if let Some(pb) = &pb {
            pb.finish_with_message("✅ Download complete");
        }

        // Compute hash
        let hash_bytes = hasher.finalize();
        let sha256 = hex::encode(hash_bytes);

        Ok(DownloadResult {
            path: dest_path.to_path_buf(),
            sha256,
            size: downloaded,
        })
    }

    /// Verify file hash
    pub async fn verify_hash(path: &Path, expected_hash: &str) -> Result<bool> {
        let mut file = fs::File::open(path)
            .await
            .context("Failed to open file for verification")?;

        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];

        use tokio::io::AsyncReadExt;

        loop {
            let n = file.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let hash_bytes = hasher.finalize();
        let computed_hash = hex::encode(hash_bytes);

        Ok(computed_hash == expected_hash)
    }

    /// Extract filename from URL
    fn filename_from_url(url: &str) -> String {
        url.split('/')
            .next_back()
            .unwrap_or("unknown")
            .split('?')
            .next()
            .unwrap_or("unknown")
            .to_string()
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new().expect("Failed to create downloader")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    #[test]
    fn test_filename_extraction() {
        assert_eq!(
            Downloader::filename_from_url("https://example.com/file.png"),
            "file.png"
        );
        assert_eq!(
            Downloader::filename_from_url("https://example.com/file.png?v=1"),
            "file.png"
        );
    }

    #[tokio::test]
    async fn test_hash_verification() {
        use tempfile::NamedTempFile;
        use tokio::io::AsyncWriteExt;

        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        // Write test data
        let mut file = fs::File::create(path).await.unwrap();
        file.write_all(b"test data").await.unwrap();
        file.sync_all().await.unwrap();

        // Compute expected hash
        let expected = "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9";

        // Verify
        let result = Downloader::verify_hash(path, expected).await.unwrap();
        assert!(result, "Hash verification failed");
    }

    fn make_test_downloader(
        client: reqwest::Client,
        max_retries: u32,
        retry_delay_ms: u64,
    ) -> Downloader {
        Downloader {
            client,
            max_retries,
            retry_delay_ms,
            max_concurrent: 8,
        }
    }

    #[tokio::test]
    async fn test_download_success_writes_file_and_hashes() {
        let server = MockServer::start().await;

        let body = b"hello".to_vec();
        Mock::given(method("GET"))
            .and(path("/file.bin"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Content-Length", body.len().to_string())
                    .set_body_bytes(body.clone()),
            )
            .mount(&server)
            .await;

        let temp = tempfile::TempDir::new().unwrap();
        let dest_path = temp.path().join("out").join("file.bin");

        let downloader = Downloader::new().unwrap();
        let result = downloader
            .download(&format!("{}/file.bin", server.uri()), &dest_path, false)
            .await
            .unwrap();

        assert_eq!(result.size, 5);
        assert!(result.path.exists());
        assert_eq!(tokio::fs::read(&dest_path).await.unwrap(), body);

        let mut hasher = Sha256::new();
        hasher.update(b"hello");
        let expected = hex::encode(hasher.finalize());
        assert_eq!(result.sha256, expected);
    }

    struct FlakyResponder {
        calls: Arc<AtomicUsize>,
        ok_body: Vec<u8>,
    }

    impl Respond for FlakyResponder {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                ResponseTemplate::new(500).set_body_string("temporary error")
            } else {
                ResponseTemplate::new(200)
                    .insert_header("Content-Length", self.ok_body.len().to_string())
                    .set_body_bytes(self.ok_body.clone())
            }
        }
    }

    #[tokio::test]
    async fn test_download_retries_then_succeeds_without_sleeping_long() {
        let server = MockServer::start().await;

        let calls = Arc::new(AtomicUsize::new(0));
        let responder = FlakyResponder {
            calls: calls.clone(),
            ok_body: b"ok".to_vec(),
        };

        Mock::given(method("GET"))
            .and(path("/flaky"))
            .respond_with(responder)
            .mount(&server)
            .await;

        let temp = tempfile::TempDir::new().unwrap();
        let dest_path = temp.path().join("flaky.bin");

        let client = reqwest::Client::builder().build().unwrap();
        let downloader = make_test_downloader(client, 1, 1);

        let result = downloader
            .download(&format!("{}/flaky", server.uri()), &dest_path, false)
            .await
            .unwrap();

        assert_eq!(tokio::fs::read(&dest_path).await.unwrap(), b"ok".to_vec());
        assert!(calls.load(Ordering::SeqCst) >= 2);
        assert_eq!(result.size, 2);
    }

    #[tokio::test]
    async fn test_download_parallel_downloads_all_tasks() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/a"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"a".to_vec()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/b"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bb".to_vec()))
            .mount(&server)
            .await;

        let temp = tempfile::TempDir::new().unwrap();
        let dest_a = temp.path().join("a.bin");
        let dest_b = temp.path().join("b.bin");

        let tasks = vec![
            DownloadTask {
                url: format!("{}/a", server.uri()),
                dest_path: dest_a.clone(),
                key: "a".to_string(),
            },
            DownloadTask {
                url: format!("{}/b", server.uri()),
                dest_path: dest_b.clone(),
                key: "b".to_string(),
            },
        ];

        let downloader = Downloader::new().unwrap().with_max_concurrent(2);
        let mut results = downloader.download_parallel(tasks, false).await.unwrap();
        results.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "a");
        assert!(results[0].1.is_ok());
        assert_eq!(tokio::fs::read(&dest_a).await.unwrap(), b"a".to_vec());

        assert_eq!(results[1].0, "b");
        assert!(results[1].1.is_ok());
        assert_eq!(tokio::fs::read(&dest_b).await.unwrap(), b"bb".to_vec());
    }
}
