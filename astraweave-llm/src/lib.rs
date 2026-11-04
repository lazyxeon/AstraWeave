#![forbid(unsafe_code)]
#[cfg(feature = "ollama")]
use anyhow::Context;
use anyhow::{bail, Result};
#[cfg(feature = "ollama")]
use astraweave_core::{byte_prefix, byte_suffix};

/// Model text above this is rejected before `serde_json` allocates a document.
pub const MAX_MODEL_JSON_BYTES: usize = 64 * 1024;

/// Hard ceiling for any capped body read, including embedding batches.
pub const MAX_MODEL_BODY_LIMIT: usize = 8 * 1024 * 1024;

/// Raw NDJSON budget. Each token repeats the envelope, so this is larger
/// than the 64 KiB extracted-content cap and still under [`MAX_MODEL_BODY_LIMIT`].
pub const MAX_MODEL_STREAM_BYTES: usize = MAX_MODEL_JSON_BYTES * 16;

/// The prefix of `text` that fits in [`MAX_MODEL_JSON_BYTES`].
pub fn bounded_model_text(text: &str) -> &str {
    if text.len() <= MAX_MODEL_JSON_BYTES {
        return text;
    }
    let mut end = MAX_MODEL_JSON_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Count `extra` toward the raw NDJSON budget.
///
/// Empty lines and invalid UTF-8 still count. Extracted content is capped
/// separately at [`MAX_MODEL_JSON_BYTES`].
///
/// The budget is inclusive. Exactly [`MAX_MODEL_STREAM_BYTES`] is kept, and
/// one more byte fails. [`append_capped`] uses the same `>` check.
#[cfg(feature = "ollama")]
pub fn note_stream_bytes(seen: &mut usize, extra: usize) -> Result<()> {
    *seen = seen.saturating_add(extra);
    if *seen > MAX_MODEL_STREAM_BYTES {
        bail!(
            "model stream exceeds {MAX_MODEL_STREAM_BYTES} byte cap ({} bytes)",
            *seen
        );
    }
    Ok(())
}

/// Keep `text` when it fits in the 64 KiB content cap.
///
/// Exactly [`MAX_MODEL_JSON_BYTES`] is returned. One more byte is an error.
#[cfg(feature = "ollama")]
fn finish_capped(text: String) -> Result<String> {
    if text.len() > MAX_MODEL_JSON_BYTES {
        bail!("model text exceeds 64 KiB cap ({} bytes)", text.len());
    }
    Ok(text)
}

#[cfg(feature = "ollama")]
fn append_capped(dst: &mut String, extra: &str) -> Result<()> {
    let next = dst.len().saturating_add(extra.len());
    if next > MAX_MODEL_JSON_BYTES {
        bail!("model text exceeds 64 KiB cap ({next} bytes)");
    }
    dst.push_str(extra);
    Ok(())
}

/// Take one NDJSON line from the default chat stream.
#[cfg(feature = "ollama")]
fn push_stream_line(assistant_acc: &mut String, line: &str, done_flag: &mut bool) -> Result<()> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some(content) = v
            .get("message")
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
        {
            append_capped(assistant_acc, content)?;
        }
        if let Some(resp_txt) = v.get("response").and_then(|r| r.as_str()) {
            append_capped(assistant_acc, resp_txt)?;
        }
        if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
            if let Some(first) = choices.first() {
                if let Some(content) = first
                    .get("message")
                    .and_then(|msg| msg.get("content"))
                    .and_then(|c| c.as_str())
                {
                    append_capped(assistant_acc, content)?;
                }
            }
        }
        if v.get("done").and_then(|d| d.as_bool()) == Some(true) {
            *done_flag = true;
        }
    } else {
        append_capped(assistant_acc, line)?;
    }
    Ok(())
}

/// Read a model body, stopping at [`MAX_MODEL_JSON_BYTES`].
#[cfg(feature = "ollama")]
pub async fn read_capped_text(response: reqwest::Response) -> Result<String> {
    read_capped_text_limited(response, MAX_MODEL_JSON_BYTES).await
}

/// Refuse a limit that would make the capped reader fail open.
#[cfg(feature = "ollama")]
fn accept_body_limit(limit: usize) -> Result<()> {
    if limit == 0 || limit > MAX_MODEL_BODY_LIMIT {
        bail!("model body limit {limit} exceeds the {MAX_MODEL_BODY_LIMIT} byte ceiling");
    }
    Ok(())
}

/// Read a body until `limit` bytes, then stop.
#[cfg(feature = "ollama")]
pub async fn read_capped_text_limited(response: reqwest::Response, limit: usize) -> Result<String> {
    use futures_util::StreamExt;
    accept_body_limit(limit)?;
    let mut stream = response.bytes_stream();
    let mut buf = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("model body read failed")?;
        let Some(next) = buf.len().checked_add(chunk.len()) else {
            bail!("model body length overflow");
        };
        if next > limit {
            if limit == MAX_MODEL_JSON_BYTES {
                bail!("model text exceeds 64 KiB cap ({next} bytes)");
            }
            bail!("model text exceeds {limit} byte cap ({next} bytes)");
        }
        buf.extend_from_slice(&chunk);
    }
    String::from_utf8(buf).context("model body is not utf-8")
}

/// How long the next body read may block, and which deadline a timeout is.
struct ReadWait {
    wait: std::time::Duration,
    on_timeout: &'static str,
}

/// Silence, the current gap, and the stream lifetime.
///
/// `idle` is the silence limit. `overall` caps the same gap: when it is
/// longer than `idle`, the wait is `idle`, so a caller that passes twice
/// the idle limit does not get two idle periods. `lifetime` is measured
/// from `life_started` and does not move when a chunk arrives.
/// `idle_timeout` is the silence error. The other errors are
/// `streaming overall deadline` and `streaming lifetime deadline`.
#[cfg(feature = "ollama")]
fn next_read_wait(
    idle: std::time::Duration,
    overall: std::time::Duration,
    gap_started: std::time::Instant,
    lifetime: std::time::Duration,
    life_started: std::time::Instant,
    idle_timeout: &'static str,
) -> Result<ReadWait> {
    let gap_left = overall.saturating_sub(gap_started.elapsed());
    let life_left = lifetime.saturating_sub(life_started.elapsed());
    if life_left.is_zero() {
        bail!("streaming lifetime deadline");
    }
    if gap_left.is_zero() {
        bail!("streaming overall deadline");
    }
    let wait = idle.min(gap_left).min(life_left);
    let on_timeout = if life_left <= gap_left && life_left < idle {
        "streaming lifetime deadline"
    } else if wait < idle {
        "streaming overall deadline"
    } else {
        idle_timeout
    };
    Ok(ReadWait { wait, on_timeout })
}

/// Same length as the 120s [`with_call_deadline`] wrappers.
///
/// Default chat, Hermes streaming, and `examples/ollama_probe` use it as
/// the stream lifetime measured from the request start.
#[cfg(feature = "ollama")]
pub const DEFAULT_STREAM_LIFETIME: std::time::Duration = std::time::Duration::from_secs(120);

/// Same length as Qwen's 300s [`with_call_deadline`] wrapper.
#[cfg(feature = "ollama")]
pub const QWEN_STREAM_LIFETIME: std::time::Duration = std::time::Duration::from_secs(300);

/// Read `inner` until silence, the current gap, or `lifetime` ends.
///
/// Each read waits the silence limit, the time still left in the current
/// gap, or the time still left in `lifetime`, whichever is shortest.
/// `lifetime` starts when this stream is built and does not reset. A
/// non-empty chunk starts a new gap, so late wakes whose gaps stay inside
/// `idle` are kept. An empty chunk does not start a gap: an HTTP/2 DATA
/// frame with no bytes must not postpone the deadline. The first error is
/// yielded once. The next poll ends the stream.
#[cfg(feature = "ollama")]
pub fn bytes_within_deadline<S, T, E>(
    inner: S,
    idle: std::time::Duration,
    overall: std::time::Duration,
    lifetime: std::time::Duration,
) -> impl futures_util::Stream<Item = Result<T>> + Send
where
    S: futures_util::Stream<Item = Result<T, E>> + Unpin + Send + 'static,
    T: AsRef<[u8]> + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    use futures_util::StreamExt;
    futures_util::stream::unfold(
        (
            inner,
            false,
            std::time::Instant::now(),
            std::time::Instant::now(),
        ),
        move |(mut inner, stop, gap_started, life_started)| async move {
            if stop {
                return None;
            }
            let decision = match next_read_wait(
                idle,
                overall,
                gap_started,
                lifetime,
                life_started,
                "stream idle timeout",
            ) {
                Ok(decision) => decision,
                Err(err) => {
                    return Some((Err(err), (inner, true, gap_started, life_started)));
                }
            };
            match tokio::time::timeout(decision.wait, inner.next()).await {
                Ok(Some(Ok(chunk))) => {
                    let gap_started = if chunk.as_ref().is_empty() {
                        gap_started
                    } else {
                        std::time::Instant::now()
                    };
                    Some((Ok(chunk), (inner, false, gap_started, life_started)))
                }
                Ok(Some(Err(err))) => Some((
                    Err(anyhow::Error::new(err).context("stream body error")),
                    (inner, true, gap_started, life_started),
                )),
                Ok(None) => None,
                Err(_) => Some((
                    Err(anyhow::anyhow!(decision.on_timeout)),
                    (inner, true, gap_started, life_started),
                )),
            }
        },
    )
}

/// Split a drained NDJSON block into lines.
///
/// Lines before a bad byte are kept. That bad line is an error, and lines
/// after it in the same slice are not decoded.
#[cfg(feature = "ollama")]
pub(crate) fn ndjson_line_strings(bytes: &[u8]) -> Vec<Result<String>> {
    let mut out = Vec::new();
    for mut piece in bytes.split(|b| *b == b'\n') {
        if piece.last() == Some(&b'\r') {
            piece = &piece[..piece.len() - 1];
        }
        if piece.is_empty() {
            continue;
        }
        match String::from_utf8(piece.to_vec()) {
            Ok(line) => out.push(Ok(line)),
            Err(_) => {
                out.push(Err(anyhow::anyhow!("model stream line is not utf-8")));
                break;
            }
        }
    }
    out
}

/// Text from one NDJSON line. `stop` ends the scan after these pieces.
#[cfg(feature = "ollama")]
pub(crate) struct NdjsonYield {
    pub pieces: Vec<String>,
    pub stop: bool,
}

/// Scan NDJSON bytes until one error or until `on_line` says to stop.
///
/// Each read waits the idle limit, the time still left in the gap since
/// the last non-empty chunk, or the time still left in `lifetime`,
/// whichever is shortest. `lifetime` starts when this scan is built.
/// A non-empty chunk starts a new gap. An empty chunk does not. A bad
/// line does not fail earlier lines from the same read. After one error
/// the next poll ends the stream, so a later chunk is not read and the
/// byte cap still applies up to the error.
#[cfg(feature = "ollama")]
pub(crate) fn scan_model_ndjson<S, T, E, F>(
    inner: S,
    idle: std::time::Duration,
    overall: std::time::Duration,
    lifetime: std::time::Duration,
    on_line: F,
) -> impl futures_util::Stream<Item = Result<String>> + Send
where
    S: futures_util::Stream<Item = Result<T, E>> + Unpin + Send + 'static,
    T: AsRef<[u8]> + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
    F: FnMut(&str) -> NdjsonYield + Send + 'static,
{
    use futures_util::StreamExt;

    struct ScanState<B, F> {
        bytes: B,
        buf: Vec<u8>,
        seen: usize,
        pending: std::vec::IntoIter<Result<String>>,
        stop: bool,
        kept: usize,
        on_line: F,
    }

    let bytes = Box::pin(bytes_within_deadline(inner, idle, overall, lifetime));
    futures_util::stream::unfold(
        ScanState {
            bytes,
            buf: Vec::new(),
            seen: 0,
            pending: Vec::new().into_iter(),
            stop: false,
            kept: 0,
            on_line,
        },
        |mut state| async move {
            loop {
                if let Some(item) = state.pending.next() {
                    return Some((item, state));
                }
                if state.stop {
                    return None;
                }
                match state.bytes.next().await {
                    None => return None,
                    Some(Err(err)) => {
                        state.stop = true;
                        return Some((Err(err), state));
                    }
                    Some(Ok(chunk)) => {
                        let chunk = chunk.as_ref();
                        if let Err(err) = note_stream_bytes(&mut state.seen, chunk.len()) {
                            state.stop = true;
                            return Some((Err(err), state));
                        }
                        state.buf.extend_from_slice(chunk);
                        let mut batch = Vec::new();
                        let mut stop_now = false;
                        if let Some(last_nl) = state.buf.iter().rposition(|&b| b == b'\n') {
                            let complete: Vec<u8> = state.buf.drain(..=last_nl).collect();
                            for line in ndjson_line_strings(&complete) {
                                match line {
                                    Err(err) => {
                                        batch.push(Err(err));
                                        stop_now = true;
                                        break;
                                    }
                                    Ok(line) => {
                                        let trimmed = line.trim();
                                        if trimmed.is_empty() {
                                            continue;
                                        }
                                        let yielded = (state.on_line)(trimmed);
                                        for piece in yielded.pieces {
                                            if piece.is_empty() {
                                                continue;
                                            }
                                            let next = state.kept.saturating_add(piece.len());
                                            if next > MAX_MODEL_JSON_BYTES {
                                                batch.push(Err(anyhow::anyhow!(
                                                    "model text exceeds 64 KiB cap ({next} bytes)"
                                                )));
                                                stop_now = true;
                                                break;
                                            }
                                            state.kept = next;
                                            batch.push(Ok(piece));
                                        }
                                        if stop_now || yielded.stop {
                                            stop_now = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        state.pending = batch.into_iter();
                        if stop_now {
                            state.stop = true;
                        }
                    }
                }
            }
        },
    )
}

/// Bound a one-shot model call, headers and body together.
#[cfg(feature = "ollama")]
pub async fn with_call_deadline<T>(
    secs: u64,
    fut: impl std::future::Future<Output = Result<T>>,
) -> Result<T> {
    match tokio::time::timeout(std::time::Duration::from_secs(secs), fut).await {
        Ok(result) => result,
        Err(_) => bail!("model call timed out after {secs}s"),
    }
}

/// Read and parse a model JSON body, stopping at [`MAX_MODEL_JSON_BYTES`].
#[cfg(feature = "ollama")]
pub async fn read_capped_json<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T> {
    parse_bounded_model_json(&read_capped_text(response).await?)
}

/// Read a model body until [`MAX_MODEL_JSON_BYTES`] or `lifetime_left`.
///
/// The 64 KiB cap is the same one [`read_capped_text`] uses. A silent read
/// still ends at the client read timeout when that is shorter. A timeout
/// on `lifetime_left` is `streaming lifetime deadline`.
#[cfg(feature = "ollama")]
pub async fn read_capped_text_within(
    response: reqwest::Response,
    lifetime_left: std::time::Duration,
) -> Result<String> {
    if lifetime_left.is_zero() {
        bail!("streaming lifetime deadline");
    }
    match tokio::time::timeout(lifetime_left, read_capped_text(response)).await {
        Ok(result) => result,
        Err(_) => bail!("streaming lifetime deadline"),
    }
}

/// Read and parse a model JSON body until [`MAX_MODEL_JSON_BYTES`] or
/// `lifetime_left`.
#[cfg(feature = "ollama")]
pub async fn read_capped_json_within<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    lifetime_left: std::time::Duration,
) -> Result<T> {
    parse_bounded_model_json(&read_capped_text_within(response, lifetime_left).await?)
}

#[cfg(test)]
mod bounded_model_text_tests {
    use super::*;

    #[test]
    fn stops_at_the_json_cap() {
        let big = "y".repeat(MAX_MODEL_JSON_BYTES + 10);
        assert_eq!(bounded_model_text(&big).len(), MAX_MODEL_JSON_BYTES);
        assert_eq!(bounded_model_text("short"), "short");
    }
}

#[cfg(all(test, feature = "ollama"))]
mod body_limit_tests {
    use super::*;

    #[test]
    fn a_saturated_limit_is_refused() {
        let err = accept_body_limit(usize::MAX).expect_err("open");
        assert!(err.to_string().contains("ceiling"), "{err}");
        assert!(accept_body_limit(MAX_MODEL_JSON_BYTES).is_ok());
        assert!(accept_body_limit(0).is_err());
    }
}

#[cfg(all(test, feature = "ollama"))]
mod assembled_debug_snippet_tests {
    use super::*;

    #[test]
    fn ollama_assembled_snippets_match_char_safe_helpers() {
        let candidate = "日".repeat(200);
        assert!(candidate.len() > 500);
        let (start, end) =
            ollama_assembled_debug_snippets(&candidate).expect("long assembled text");
        assert_eq!(start, byte_prefix(&candidate, 200));
        assert_eq!(end, byte_suffix(&candidate, 200));
        let raw_tail_start = candidate.len().saturating_sub(200);
        assert!(!candidate.is_char_boundary(raw_tail_start));
    }

    #[test]
    fn ollama_assembled_snippets_short_text_omitted() {
        assert!(ollama_assembled_debug_snippets("short").is_none());
        assert!(ollama_assembled_debug_snippets(&"x".repeat(500)).is_none());
        assert!(ollama_assembled_debug_snippets(&"x".repeat(501)).is_some());
    }
}

#[cfg(all(test, feature = "ollama"))]
mod ndjson_scan_tests {
    use super::*;
    use futures_util::StreamExt;

    fn content_line(line: &str) -> NdjsonYield {
        let json: serde_json::Value = serde_json::from_str(line).expect("line");
        let text = json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let stop = json["done"].as_bool() == Some(true);
        NdjsonYield {
            pieces: if text.is_empty() {
                Vec::new()
            } else {
                vec![text]
            },
            stop,
        }
    }

    fn chunk_then_hang(
        bytes: &'static [u8],
    ) -> std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<Vec<u8>, std::io::Error>> + Send>>
    {
        let first = futures_util::stream::once(async move { Ok(bytes.to_vec()) });
        let hang = futures_util::stream::pending::<Result<Vec<u8>, std::io::Error>>();
        Box::pin(first.chain(hang))
    }

    #[test]
    fn ndjson_lines_split_before_utf8() {
        let lines = ndjson_line_strings(b"{\"a\":1}\n\xff\n{\"a\":2}\n");
        assert_eq!(lines.len(), 2);
        assert!(lines[0].is_ok());
        assert!(lines[1]
            .as_ref()
            .expect_err("bad")
            .to_string()
            .contains("utf-8"));
    }

    #[tokio::test]
    async fn ndjson_scan_keeps_a_good_line_and_ends() {
        let idle = std::time::Duration::from_secs(5);
        let mut stream = Box::pin(scan_model_ndjson(
            chunk_then_hang(b"{\"message\":{\"content\":\"hello\"}}\n\xff\n{\"message\":{\"content\":\"later\"}}\n"),
            idle,
            idle,
            std::time::Duration::from_secs(30),
            content_line,
        ));
        let started = std::time::Instant::now();
        let items = tokio::time::timeout(std::time::Duration::from_millis(500), async {
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item);
            }
            items
        })
        .await
        .expect("scan kept reading after the bad line");
        assert!(started.elapsed() < std::time::Duration::from_millis(500));
        assert_eq!(items.len(), 2, "{items:?}");
        assert_eq!(items[0].as_ref().expect("good"), "hello");
        assert!(items[1]
            .as_ref()
            .expect_err("bad")
            .to_string()
            .contains("utf-8"));
    }

    #[tokio::test]
    async fn ndjson_scan_waits_only_the_time_left() {
        let idle = std::time::Duration::from_secs(5);
        let overall = std::time::Duration::from_millis(250);
        let mut stream = Box::pin(scan_model_ndjson(
            chunk_then_hang(b"{"),
            idle,
            overall,
            std::time::Duration::from_secs(30),
            content_line,
        ));
        let started = std::time::Instant::now();
        let item = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .expect("drip waited a full idle period")
            .expect("stream ended before the deadline");
        let elapsed = started.elapsed();
        let err = item.expect_err("deadline");
        assert!(
            err.to_string().contains("streaming overall deadline"),
            "{err}"
        );
        assert!(elapsed < std::time::Duration::from_secs(1), "{elapsed:?}");
        assert!(
            elapsed >= std::time::Duration::from_millis(150),
            "{elapsed:?}"
        );
        let next = tokio::time::timeout(std::time::Duration::from_millis(200), stream.next())
            .await
            .expect("stream stayed open after the deadline");
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn ndjson_scan_keeps_chunks_whose_gaps_stay_inside_the_window() {
        let idle = std::time::Duration::from_secs(2);
        let overall = std::time::Duration::from_millis(200);
        let inner = futures_util::stream::unfold(0u32, |i| async move {
            if i >= 8 {
                return None;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let line = format!("{{\"message\":{{\"content\":\"n{i:02}\"}}}}\n");
            Some((Ok::<Vec<u8>, std::io::Error>(line.into_bytes()), i + 1))
        });
        let mut stream = Box::pin(scan_model_ndjson(
            Box::pin(inner),
            idle,
            overall,
            std::time::Duration::from_secs(30),
            content_line,
        ));
        let started = std::time::Instant::now();
        let items = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item);
            }
            items
        })
        .await
        .expect("scan stopped on the sum of gaps");
        let elapsed = started.elapsed();
        assert!(
            elapsed > overall,
            "stream ended inside the old wall clock ({elapsed:?})"
        );
        // Eight 50 ms gaps are 400 ms. Two seconds still fails a scan that
        // waits out the 2 s idle limit, and it leaves room for a late wake.
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "eight 50 ms chunks took {elapsed:?}"
        );
        assert_eq!(items.len(), 8, "{items:?}");
        for (i, item) in items.iter().enumerate() {
            assert_eq!(item.as_ref().expect("token"), &format!("n{i:02}"));
        }
    }

    #[tokio::test]
    async fn ndjson_scan_errors_when_a_drip_passes_the_lifetime() {
        let idle = std::time::Duration::from_secs(2);
        let overall = idle.saturating_mul(2);
        let lifetime = std::time::Duration::from_millis(220);
        let inner = futures_util::stream::unfold(0u32, |i| async move {
            if i >= 15 {
                return None;
            }
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            Some((Ok::<Vec<u8>, std::io::Error>(vec![b'x']), i + 1))
        });
        let mut stream = Box::pin(scan_model_ndjson(
            Box::pin(inner),
            idle,
            overall,
            lifetime,
            content_line,
        ));
        let started = std::time::Instant::now();
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let mut err = None;
            while let Some(item) = stream.next().await {
                if let Err(item) = item {
                    err = Some(item.to_string());
                    break;
                }
            }
            err
        })
        .await
        .expect("drip ran past the lifetime");
        let err = outcome.expect("stream ended without an error");
        assert!(err.contains("streaming lifetime deadline"), "{err}");
        let elapsed = started.elapsed();
        assert!(
            elapsed >= std::time::Duration::from_millis(150),
            "{elapsed:?}"
        );
        assert!(elapsed < std::time::Duration::from_secs(2), "{elapsed:?}");
    }

    #[tokio::test]
    async fn ndjson_scan_does_not_reset_the_gap_on_an_empty_chunk() {
        let idle = std::time::Duration::from_secs(2);
        let overall = std::time::Duration::from_millis(150);
        let inner = futures_util::stream::unfold(0u32, |i| async move {
            if i >= 12 {
                return None;
            }
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
            Some((Ok::<Vec<u8>, std::io::Error>(Vec::new()), i + 1))
        });
        let mut stream = Box::pin(scan_model_ndjson(
            Box::pin(inner),
            idle,
            overall,
            std::time::Duration::from_secs(5),
            content_line,
        ));
        let started = std::time::Instant::now();
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let mut err = None;
            while let Some(item) = stream.next().await {
                if let Err(item) = item {
                    err = Some(item.to_string());
                    break;
                }
            }
            err
        })
        .await
        .expect("empty chunks postponed the gap deadline");
        let err = outcome.expect("stream ended without an error");
        assert!(err.contains("streaming overall deadline"), "{err}");
        let elapsed = started.elapsed();
        assert!(
            elapsed >= std::time::Duration::from_millis(80),
            "{elapsed:?}"
        );
        assert!(elapsed < std::time::Duration::from_secs(1), "{elapsed:?}");
    }

    #[tokio::test]
    async fn ndjson_scan_stops_when_the_line_says_done() {
        let idle = std::time::Duration::from_secs(5);
        let mut stream = Box::pin(scan_model_ndjson(
            chunk_then_hang(
                b"{\"message\":{\"content\":\"hi\"},\"done\":true}\n{\"message\":{\"content\":\"nope\"}}\n",
            ),
            idle,
            idle,
            std::time::Duration::from_secs(30),
            content_line,
        ));
        let items = tokio::time::timeout(std::time::Duration::from_millis(500), async {
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item);
            }
            items
        })
        .await
        .expect("done:true left the scan open");
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].as_ref().expect("hi"), "hi");
    }

    #[tokio::test]
    async fn ndjson_scan_stops_when_the_byte_cap_fires() {
        let idle = std::time::Duration::from_secs(5);
        let mut body = vec![b'\n'; MAX_MODEL_STREAM_BYTES + 8];
        body.extend_from_slice(b"{\"message\":{\"content\":\"later\"}}\n");
        let first = futures_util::stream::once(async move { Ok::<_, std::io::Error>(body) });
        let hang = futures_util::stream::pending::<Result<Vec<u8>, std::io::Error>>();
        let mut stream = Box::pin(scan_model_ndjson(
            Box::pin(first.chain(hang)),
            idle,
            idle,
            std::time::Duration::from_secs(30),
            content_line,
        ));
        let item = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
            .await
            .expect("cap left the scan open")
            .expect("stream ended");
        let err = item.expect_err("cap");
        assert!(err.to_string().contains("model stream exceeds"), "{err}");
        let next = tokio::time::timeout(std::time::Duration::from_millis(200), stream.next())
            .await
            .expect("scan read past the cap");
        assert!(next.is_none());
    }

    #[tokio::test]
    async fn ndjson_scan_caps_extracted_content_at_64kib() {
        let idle = std::time::Duration::from_secs(5);
        let content = "x".repeat(MAX_MODEL_JSON_BYTES + 8);
        let line = format!("{{\"message\":{{\"content\":\"{content}\"}}}}\n");
        assert!(line.len() < MAX_MODEL_STREAM_BYTES, "{}", line.len());
        let first =
            futures_util::stream::once(async move { Ok::<_, std::io::Error>(line.into_bytes()) });
        let hang = futures_util::stream::pending::<Result<Vec<u8>, std::io::Error>>();
        let mut stream = Box::pin(scan_model_ndjson(
            Box::pin(first.chain(hang)),
            idle,
            idle,
            std::time::Duration::from_secs(30),
            content_line,
        ));
        let item = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next())
            .await
            .expect("content cap left the scan open")
            .expect("stream ended");
        let err = item.expect_err("content");
        assert!(err.to_string().contains("64 KiB"), "{err}");
    }

    #[test]
    fn header_wait_uses_the_lifetime_when_idle_is_longer() {
        let idle = std::time::Duration::from_secs(400);
        let lifetime_left = std::time::Duration::from_secs(300);
        assert_eq!(header_wait(idle, lifetime_left), lifetime_left);
        assert_eq!(
            header_wait(std::time::Duration::from_secs(1), lifetime_left),
            std::time::Duration::from_secs(1)
        );
        assert!(header_wait(idle, std::time::Duration::ZERO).is_zero());
    }

    #[test]
    fn budgets_keep_exactly_the_cap() {
        let mut seen = 0usize;
        assert!(note_stream_bytes(&mut seen, MAX_MODEL_STREAM_BYTES).is_ok());
        assert_eq!(seen, MAX_MODEL_STREAM_BYTES);
        let over = note_stream_bytes(&mut seen, 1).expect_err("one past the raw budget");
        assert!(over.to_string().contains("model stream exceeds"), "{over}");

        let exact = "x".repeat(MAX_MODEL_JSON_BYTES);
        assert_eq!(
            finish_capped(exact.clone()).expect("exact").len(),
            MAX_MODEL_JSON_BYTES
        );
        let text_over = finish_capped(format!("{exact}y")).expect_err("one past the content cap");
        assert!(text_over.to_string().contains("64 KiB"), "{text_over}");

        let mut acc = String::new();
        assert!(append_capped(&mut acc, &exact).is_ok());
        assert!(append_capped(&mut acc, "y").is_err());
    }
}

#[cfg(all(test, feature = "ollama"))]
mod read_capped_tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn stops_before_the_rest_of_the_body() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            let Ok((mut sock, _)) = listener.accept() else {
                return;
            };
            let mut buf = [0u8; 1];
            let mut seen = Vec::new();
            let _ = sock.set_read_timeout(Some(std::time::Duration::from_secs(2)));
            while seen.len() < 8192 {
                match std::io::Read::read(&mut sock, &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        seen.push(buf[0]);
                        if seen.ends_with(b"\r\n\r\n") {
                            break;
                        }
                    }
                }
            }
            let body = vec![b'a'; MAX_MODEL_JSON_BYTES + 4096];
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = sock.write_all(header.as_bytes());
            let _ = sock.write_all(&body);
            let _ = sock.flush();
        });
        let client = build_model_client(&format!("http://{addr}"), false).expect("client");
        let response = client
            .get(format!("http://{addr}/"))
            .send()
            .await
            .expect("send");
        let err = read_capped_text(response).await.expect_err("oversize");
        assert!(err.to_string().contains("64 KiB"), "{err}");
    }
}

/// Parse JSON that came from a local model. Hostile lengths never reach `serde_json`.
pub fn parse_bounded_model_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > MAX_MODEL_JSON_BYTES {
        bail!("model text exceeds 64 KiB cap ({} bytes)", text.len());
    }
    serde_json::from_str(text).map_err(|err| anyhow::anyhow!("model json: {err}"))
}

/// Keep a loopback model URL.
///
/// A URL is kept only when it is `http` or `https` and has no userinfo. Userinfo,
/// a non-http scheme, and a string the parser rejects are always replaced with
/// `http://127.0.0.1:11434`, including when `allow_remote` is set. A non-loopback
/// host is kept only when `allow_remote` is set. Otherwise the host must be
/// `127.0.0.0/8`, `::1`, or `localhost`.
///
/// Every loopback URL check in the workspace goes through [`is_loopback_http_url`].
pub fn loopback_model_url(url: String, allow_remote: bool) -> String {
    const FALLBACK: &str = "http://127.0.0.1:11434";
    // Userinfo is rejected even when `allow_remote` is set. The raw string is
    // not returned until the URL parses as http(s) with an empty userinfo.
    if !http_url_without_userinfo(&url) {
        return FALLBACK.to_string();
    }
    if allow_remote || is_loopback_http_url(&url) {
        url
    } else {
        FALLBACK.to_string()
    }
}

/// Keep `url` only when [`loopback_model_url`] would not rewrite it.
///
/// Constructors use this so a rejected host or userinfo cannot be stored, and
/// a bearer token cannot be attached to the loopback fallback. The error does
/// not include the URL.
pub fn require_kept_model_url(url: String, allow_remote: bool) -> Result<String> {
    let kept = loopback_model_url(url.clone(), allow_remote);
    if kept != url {
        bail!("model URL was rejected");
    }
    Ok(kept)
}

fn http_url_without_userinfo(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    (parsed.scheme() == "http" || parsed.scheme() == "https") && !url_has_userinfo(url, &parsed)
}

/// Re-check the URL that is about to be requested.
///
/// Userinfo is always rejected. A non-loopback host is rejected unless this
/// client was built with `allowing_remote`.
#[cfg(feature = "ollama")]
pub(crate) fn ensure_model_request(url: &str, allow_remote: bool) -> Result<()> {
    if !http_url_without_userinfo(url) {
        bail!("model URL has userinfo or is not http(s)");
    }
    if !allow_remote && !is_loopback_http_url(url) {
        bail!("model URL is not loopback");
    }
    Ok(())
}

/// Builder for a model HTTP client that cannot leave the machine.
///
/// `no_proxy` ignores `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`. Redirects
/// are followed only when the next hop is still loopback. The name `localhost`
/// is pinned to `127.0.0.1`; the URL port is kept.
///
/// [`allowing_remote`](OllamaClient::allowing_remote) does not use this builder.
/// It keeps the caller URL and uses reqwest's default client, which honors
/// those proxy variables and follows redirects.
#[cfg(feature = "ollama")]
pub fn loopback_http_client() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(loopback_redirect_policy())
        .resolve(
            "localhost",
            std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        )
        .resolve(
            "localhost.",
            std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        )
}

#[cfg(feature = "ollama")]
fn loopback_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 8 {
            attempt.error("too many loopback redirects")
        } else if is_loopback_http_url(attempt.url().as_str()) {
            attempt.follow()
        } else {
            attempt.error("redirect is not loopback")
        }
    })
}

/// Loopback URLs use [`loopback_http_client`]. `allow_remote` keeps a
/// non-loopback URL on reqwest's default client (proxy env and redirects).
/// A build failure is returned. There is no [`reqwest::Client::new`] fallback.
#[cfg(feature = "ollama")]
pub fn build_model_client(url: &str, allow_remote: bool) -> Result<reqwest::Client> {
    let remote = allow_remote && !is_loopback_http_url(url);
    let builder = if remote {
        reqwest::Client::builder()
    } else {
        loopback_http_client()
    };
    builder
        .tcp_nodelay(true)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .pool_max_idle_per_host(8)
        .connect_timeout(std::time::Duration::from_secs(30))
        // Per read, reset after each successful read. `.text()` and `.json()`
        // use it. A chunk stream is not cut off while bytes keep arriving.
        .read_timeout(stream_read_idle())
        .build()
        .map_err(|err| anyhow::anyhow!("model http client: {err}"))
}

/// Silence allowed on one read, from `OLLAMA_TIMEOUT_SECS` (default 180).
///
/// Clamped so a later scale to milliseconds cannot overflow `Duration`.
#[cfg(feature = "ollama")]
pub fn stream_read_idle() -> std::time::Duration {
    const MAX_TIMEOUT_SECS: u64 = u64::MAX / 1_000;
    let secs = std::env::var("OLLAMA_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(180)
        .clamp(1, MAX_TIMEOUT_SECS);
    std::time::Duration::from_secs(secs)
}

/// Wait `idle` for response headers. The body is not covered.
///
/// `RequestBuilder::timeout` would also cancel a chunked body. Callers that
/// stream after the headers use [`bytes_within_deadline`]. A call that has
/// a stream lifetime passes [`header_wait`] so this wait cannot outlive it.
#[cfg(feature = "ollama")]
pub async fn send_for_headers(
    idle: std::time::Duration,
    send: impl std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
) -> Result<reqwest::Response> {
    match tokio::time::timeout(idle, send).await {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(err)) => Err(anyhow::Error::new(err).context("model request")),
        Err(_) => Err(anyhow::anyhow!("model headers idle timeout")),
    }
}

/// POST an Ollama JSON body. A schema `format` that the server rejects is removed
/// and the request is sent once more. A body with no schema object keeps the
/// status-only error string.
#[cfg(feature = "ollama")]
pub(crate) async fn post_ollama_json_value(
    client: &reqwest::Client,
    url: &str,
    mut body: serde_json::Value,
) -> Result<serde_json::Value> {
    loop {
        let response = send_for_headers(stream_read_idle(), client.post(url).json(&body).send())
            .await
            .context("Failed to send request to Ollama")?;
        if response.status().is_success() {
            return read_capped_json(response)
                .await
                .context("Failed to parse Ollama response");
        }
        let status = response.status();
        let had_schema = body.get("format").is_some_and(|value| value.is_object());
        if had_schema {
            let txt = read_capped_text(response)
                .await
                .with_context(|| format!("Ollama status {status} body read failed"))?;
            if plan_format::schema_format_rejected(status.as_u16(), &txt)
                && plan_format::take_schema_format(&mut body)
            {
                continue;
            }
            bail!("Ollama returned error: {status}: {txt}");
        }
        bail!("Ollama returned error: {status}");
    }
}

/// Header wait: the idle limit, or the time still left in the stream
/// lifetime, whichever is shorter.
#[cfg(feature = "ollama")]
pub fn header_wait(
    idle: std::time::Duration,
    lifetime_left: std::time::Duration,
) -> std::time::Duration {
    idle.min(lifetime_left)
}

/// `default`, or a shorter `OLLAMA_STREAM_LIFETIME_MS`.
///
/// A missing, zero, non-numeric, or longer value keeps `default`. The
/// variable cannot extend the call.
#[cfg(feature = "ollama")]
pub fn stream_call_lifetime(default: std::time::Duration) -> std::time::Duration {
    let Ok(raw) = std::env::var("OLLAMA_STREAM_LIFETIME_MS") else {
        return default;
    };
    let Ok(ms) = raw.parse::<u64>() else {
        return default;
    };
    let requested = std::time::Duration::from_millis(ms);
    if requested.is_zero() || requested >= default {
        default
    } else {
        requested
    }
}

/// Wait for headers, stopping at `lifetime_left` when that is shorter
/// than `idle`.
///
/// A timeout caused by the lifetime is `streaming lifetime deadline`.
/// A timeout caused by `idle` stays `model headers idle timeout`.
#[cfg(feature = "ollama")]
pub async fn send_headers_for_call(
    idle: std::time::Duration,
    lifetime_left: std::time::Duration,
    send: impl std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
) -> Result<reqwest::Response> {
    let wait = header_wait(idle, lifetime_left);
    if wait.is_zero() {
        bail!("streaming lifetime deadline");
    }
    match send_for_headers(wait, send).await {
        Err(err) if wait < idle && err.to_string().contains("model headers idle timeout") => {
            Err(err.context("streaming lifetime deadline"))
        }
        other => other,
    }
}
use astraweave_core::{ActionStep, PlanIntent, ToolRegistry, WorldSnapshot};
use tracing::{debug, info};

// --- Core modules (always available) ---
pub mod ab_testing;
pub mod backpressure;
pub mod batch_executor;
pub mod circuit_breaker;
pub mod compression;
pub mod fallback_system;
pub mod few_shot;
pub mod heuristics;
pub mod llm_adapter;
pub mod plan_format;
pub mod plan_parser;
pub mod production_hardening;
pub mod prompt_template;
pub mod prompts;
pub mod rate_limiter;
pub mod retry;
pub mod scheduler;
pub mod schema;
pub mod streaming_parser;
pub mod telemetry;
pub mod tool_guard;

// --- Optional LLM cache (always available by default) ---
#[cfg(feature = "llm_cache")]
pub mod cache;

// --- Phi-3 local model (optional, requires heavy GPU/CPU dependencies) ---
#[cfg(feature = "phi3")]
pub mod phi3;

// --- Ollama-based models (optional, requires reqwest) ---
#[cfg(feature = "ollama")]
pub mod hermes2pro_ollama;
#[cfg(feature = "ollama")]
pub mod phi3_ollama;
#[cfg(feature = "ollama")]
pub mod qwen3_ollama;

#[cfg(feature = "llm_cache")]
use cache::{CachedPlan, PromptCache, PromptKey};
#[cfg(feature = "llm_cache")]
use std::sync::LazyLock;

#[cfg(feature = "llm_cache")]
static GLOBAL_CACHE: LazyLock<PromptCache> = LazyLock::new(|| {
    // Read capacity from environment, default to 4096
    let capacity = std::env::var("LLM_CACHE_CAP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4096);

    // Phase 7: Similarity cache hits are nondeterministic across prompt variants and
    // can cause unexpected cross-test pollution when tests run in parallel.
    // Keep exact-match caching enabled by default, and make similarity matching opt-in.
    let similarity_threshold = std::env::var("LLM_CACHE_SIM_THRESH")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .map(|v| v.clamp(0.0, 1.0))
        .unwrap_or(1.0);

    PromptCache::with_similarity_threshold(capacity, similarity_threshold)
});

/// Clear the global LLM cache (useful for testing)
#[cfg(feature = "llm_cache")]
pub fn clear_global_cache() {
    GLOBAL_CACHE.clear();
}

/// Enum to distinguish between LLM-generated plans and fallback plans with error reasons
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PlanSource {
    Llm(PlanIntent),
    Fallback { plan: PlanIntent, reason: String },
}

/// Trait for LLM clients (mock, Ollama, etc).
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// Complete a prompt and return the full response (blocking until complete)
    async fn complete(&self, prompt: &str) -> Result<String>;

    /// Complete `prompt` and send `format` as Ollama's JSON Schema `format` field.
    ///
    /// The default ignores `format` and calls [`complete`](Self::complete).
    /// Ollama clients attach the object. A 400 that names the schema retries
    /// once without it. [`parse_llm_plan`] and [`plan_parser::parse_llm_response`]
    /// still reject a tool that is not in the registry.
    async fn complete_with_format(
        &self,
        prompt: &str,
        format: &serde_json::Value,
    ) -> Result<String> {
        let _ = format;
        self.complete(prompt).await
    }

    /// Complete a prompt with streaming support (progressive response delivery)
    ///
    /// Returns a stream of text chunks as they arrive from the LLM. Enables:
    /// - Lower time-to-first-token (start parsing before full response arrives)
    /// - Progressive UI updates (show partial results)
    /// - Integration with StreamingParser for batch inference
    ///
    /// Default implementation calls `complete()` and wraps result in single-chunk stream.
    /// Clients should override for true streaming support.
    ///
    /// # Example
    /// ```no_run
    /// # use astraweave_llm::LlmClient;
    /// # use futures_util::StreamExt;
    /// # async fn example(client: &dyn LlmClient) -> anyhow::Result<()> {
    /// let mut stream = client.complete_streaming("Generate plan").await?;
    /// while let Some(chunk) = stream.next().await {
    ///     let text = chunk?;
    ///     println!("Received: {}", text);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn complete_streaming(
        &self,
        prompt: &str,
    ) -> Result<std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<String>> + Send>>> {
        // Default: call blocking complete() and wrap in single-chunk stream
        let result = self.complete(prompt).await?;
        Ok(Box::pin(futures_util::stream::once(
            async move { Ok(result) },
        )))
    }
}

/// Mock client (no model). Emits a basic plan using simple heuristics.
pub struct MockLlm;

#[async_trait::async_trait]
impl LlmClient for MockLlm {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        // A minimal JSON that follows our schema
        // Uses tools from the simplified_tools list (fallback_system.rs:89-108)
        let out = r#"{
          "plan_id":"llm-mock",
          "steps":[
            {"act":"ThrowSmoke","x":7,"y":2},
            {"act":"MoveTo","x":4,"y":2},
            {"act":"Attack","target_id":99}
          ]
        }"#;
        Ok(out.into())
    }
}

/// Mock client that always returns an error (for testing fallback behavior)
pub struct AlwaysErrMock;

#[async_trait::async_trait]
impl LlmClient for AlwaysErrMock {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        bail!("AlwaysErrMock: simulated LLM failure")
    }
}

#[cfg(feature = "ollama")]
pub struct OllamaClient {
    url: String,
    allow_remote: bool,
    pub model: String,
    client: reqwest::Client,
}

#[cfg(feature = "ollama")]
impl OllamaClient {
    /// Stored endpoint. The first hop is checked again before each request.
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[cfg(feature = "ollama")]
impl OllamaClient {
    /// Keep a loopback URL. A URL that would be rewritten is an error.
    pub fn new(url: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        Self::assemble(url, model, false)
    }

    /// Keep a non-loopback URL and use reqwest's default client for it.
    ///
    /// That client honors `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`, and it
    /// follows redirects. A loopback URL still uses [`loopback_http_client`].
    /// Userinfo and any other URL [`loopback_model_url`] would rewrite is an error.
    pub fn allowing_remote(url: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        Self::assemble(url, model, true)
    }

    fn assemble(
        url: impl Into<String>,
        model: impl Into<String>,
        allow_remote: bool,
    ) -> Result<Self> {
        let url = require_kept_model_url(url.into(), allow_remote)?;
        let client = build_model_client(&url, allow_remote)?;
        Ok(Self {
            url,
            allow_remote,
            model: model.into(),
            client,
        })
    }
}

#[cfg(feature = "ollama")]
#[async_trait::async_trait]
impl LlmClient for OllamaClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        #[derive(serde::Serialize)]
        struct Req<'a> {
            model: &'a str,
            prompt: &'a str,
            stream: bool,
        }
        #[derive(serde::Deserialize)]
        struct Resp {
            response: String,
        }

        ensure_model_request(&self.url, self.allow_remote)?;
        let body = Req {
            model: &self.model,
            prompt,
            stream: false,
        };

        let response = send_for_headers(
            stream_read_idle(),
            self.client
                .post(format!("{}/api/generate", self.url))
                .json(&body)
                .send(),
        )
        .await
        .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            bail!("Ollama API returned error status: {}", response.status());
        }

        let parsed: Resp = read_capped_json(response)
            .await
            .context("Failed to parse Ollama response")?;

        Ok(parsed.response)
    }

    async fn complete_with_format(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<String> {
        ensure_model_request(&self.url, self.allow_remote)?;
        let mut body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
        });
        plan_format::insert_schema_format(&mut body, schema);
        let parsed: serde_json::Value =
            post_ollama_json_value(&self.client, &format!("{}/api/generate", self.url), body)
                .await?;
        parsed["response"]
            .as_str()
            .map(str::to_string)
            .context("Missing 'response' field in Ollama output")
    }
}

// New Ollama chat client that targets the local Ollama chat endpoint (e.g. http://127.0.0.1:11434/api/chat)
// This client is resilient to a couple of response shapes returned by different Ollama versions.
#[cfg(feature = "ollama")]
#[derive(Clone)]
pub struct OllamaChatClient {
    url: String,
    allow_remote: bool,
    pub model: String,
    client: reqwest::Client,
    // Low-latency tuning knobs
    low_latency: bool,
    keep_alive: Option<String>, // e.g. "5m" to keep model in RAM
    force_format_json: bool,    // add format: "json" to requests
    early_exit_on_json: bool,   // return as soon as a balanced JSON object is detected
}

#[cfg(feature = "ollama")]
impl OllamaChatClient {
    /// Stored endpoint. The first hop is checked again before each request.
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn new(url: String, model: String) -> Result<Self> {
        Self::assemble(url, model, false)
    }

    /// Keep a non-loopback URL and use reqwest's default client for it.
    ///
    /// That client honors `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`, and it
    /// follows redirects. A loopback URL still uses [`loopback_http_client`].
    /// Userinfo and any other URL [`loopback_model_url`] would rewrite is an error.
    pub fn allowing_remote(url: String, model: String) -> Result<Self> {
        Self::assemble(url, model, true)
    }

    fn assemble(url: String, model: String, allow_remote: bool) -> Result<Self> {
        let url = require_kept_model_url(url, allow_remote)?;
        let client = build_model_client(&url, allow_remote)?;

        let low_latency = std::env::var("OLLAMA_LOW_LATENCY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true);
        let keep_alive = std::env::var("OLLAMA_KEEP_ALIVE").ok(); // e.g., "5m" or "3600s"
        let force_format_json = std::env::var("OLLAMA_FORMAT_JSON")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true);
        let early_exit_on_json = std::env::var("OLLAMA_EARLY_EXIT")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true);

        Ok(Self {
            url,
            allow_remote,
            model,
            client,
            low_latency,
            keep_alive,
            force_format_json,
            early_exit_on_json,
        })
    }

    /// Warm up the model to minimize cold-start latency. Attempts a tiny generation and requests the model to remain in memory.
    pub async fn warmup(&self, timeout_secs: u64) -> Result<()> {
        ensure_model_request(&self.url, self.allow_remote)?;
        #[derive(serde::Serialize)]
        struct Msg<'a> {
            role: &'a str,
            content: &'a str,
        }
        #[derive(serde::Serialize)]
        struct Req<'a> {
            model: &'a str,
            messages: Vec<Msg<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            format: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            keep_alive: Option<&'a str>,
            stream: bool,
            options: serde_json::Value,
        }

        let format_opt = if self.force_format_json {
            Some("json")
        } else {
            None
        };
        let keep_alive_opt = self.keep_alive.as_deref();
        let options = serde_json::json!({ "num_predict": 1, "temperature": 0.0 });
        let body = Req {
            model: &self.model,
            messages: vec![Msg {
                role: "user",
                content: "ping",
            }],
            format: format_opt,
            keep_alive: keep_alive_opt,
            stream: false,
            options,
        };

        let chat_url = format!("{}/api/chat", self.url.trim_end_matches('/'));
        let deadline = std::time::Duration::from_secs(timeout_secs);
        let outcome = tokio::time::timeout(deadline, async {
            let response = send_for_headers(
                deadline,
                self.client
                    .post(&chat_url)
                    .header("Accept", "application/json")
                    .json(&body)
                    .send(),
            )
            .await
            .context("Ollama warmup request failed")?;
            let status = response.status();
            if !status.is_success() {
                let body = read_capped_text(response)
                    .await
                    .with_context(|| format!("Ollama warmup status {status} body read failed"))?;
                bail!("Ollama warmup returned error status {status}: {body}");
            }
            read_capped_text(response)
                .await
                .context("Ollama warmup body read failed")?;
            Ok(())
        })
        .await;
        match outcome {
            Ok(result) => result,
            Err(_) => bail!("Ollama warmup timed out after {timeout_secs}s"),
        }
    }

    /// Experimental: issue multiple prompts sequentially while reusing the same HTTP client and model residency.
    /// Returns the collected raw responses (not parsed into plans). Intended for micro-batching during warm stages.
    pub async fn complete_batch(&self, prompts: &[String]) -> Result<Vec<String>> {
        let mut out = Vec::with_capacity(prompts.len());
        for p in prompts {
            out.push(self.complete(p).await?);
        }
        Ok(out)
    }
}

#[cfg(feature = "ollama")]
impl OllamaChatClient {
    async fn complete_with_optional_format(
        &self,
        prompt: &str,
        format_schema: Option<&serde_json::Value>,
    ) -> Result<String> {
        ensure_model_request(&self.url, self.allow_remote)?;
        let mut schema_owned = format_schema.cloned();
        #[derive(serde::Serialize)]
        struct Msg<'a> {
            role: &'a str,
            content: &'a str,
        }
        #[derive(serde::Serialize)]
        struct Req<'a> {
            model: &'a str,
            messages: Vec<Msg<'a>>,
            stream: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            format: Option<&'a serde_json::Value>,
        }

        let client = self.client.clone();
        // Build a non-chunked JSON body string and set Connection: close so the server
        // will finish the response and close the connection (avoids some streaming behaviors).
        // Build a JSON body for non-streaming attempts. Use `.json()` on the request so
        // reqwest sets proper headers and Content-Length.

        // Some Ollama setups can take a long time to load a model initially (minutes) or
        // will stream responses. Allow a long configurable timeout and fall back to a
        // streaming read if a non-stream response isn't available.
        let idle = stream_read_idle();

        // First try a non-streaming request with a long timeout so we get a buffered JSON
        // response if the server supports it and the model responds within the window.
        let mut text = String::new();
        let chat_url = format!("{}/api/chat", self.url.trim_end_matches('/'));
        // Configure non-stream attempts/timeout via env with conservative defaults
        let max_nonstream_attempts: u32 = std::env::var("OLLAMA_NONSTREAM_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let nonstream_timeout_secs: u64 = std::env::var("OLLAMA_NONSTREAM_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20);

        let mut non_stream_ok = false;
        let mut attempt = 0u32;
        while attempt < max_nonstream_attempts && !non_stream_ok && !self.low_latency {
            attempt += 1;
            let schema_on_this_attempt = schema_owned.is_some();
            let body = Req {
                model: &self.model,
                messages: vec![Msg {
                    role: "user",
                    content: prompt,
                }],
                stream: false,
                format: schema_owned.as_ref(),
            };
            let backoff = std::time::Duration::from_millis(250 * (1 << (attempt - 1)));
            println!(
                "[ollama] non-stream attempt {}/{} to {} (timeout {}s)",
                attempt, max_nonstream_attempts, chat_url, nonstream_timeout_secs
            );
            let ns_attempt_start = std::time::Instant::now();
            // Use .json() so reqwest sets Content-Length and proper headers. Some servers
            // close the connection or reject raw bodies without length which can cause
            // "error sending request" failures; using .json() is more reliable here.
            let attempt_deadline = std::time::Duration::from_secs(nonstream_timeout_secs);
            enum Attempt {
                Text(String),
                Retry,
                SchemaRejected,
                Fail(anyhow::Error),
            }
            let timed = tokio::time::timeout(attempt_deadline, async {
                let resp = match send_for_headers(
                    attempt_deadline,
                    client
                        .post(&chat_url)
                        .header("Accept", "application/json")
                        .json(&body)
                        .send(),
                )
                .await
                {
                    Ok(resp) => resp,
                    Err(err) => {
                        println!(
                            "[ollama] non-streaming request failed on attempt {attempt}: {err:?}"
                        );
                        return Attempt::Retry;
                    }
                };
                println!(
                    "[ollama] non-streaming request returned status {}",
                    resp.status()
                );
                if resp.status().is_success() {
                    match read_capped_text(resp).await {
                        Ok(t) if !t.trim().is_empty() => Attempt::Text(t),
                        Ok(_) => {
                            println!(
                                "[ollama] buffered body was empty on attempt {attempt}, will retry/fallback"
                            );
                            Attempt::Retry
                        }
                        Err(err) => Attempt::Fail(err.context("Ollama non-streaming body read failed")),
                    }
                } else {
                    let status = resp.status();
                    match read_capped_text(resp).await {
                        Ok(b) if schema_on_this_attempt && plan_format::schema_format_rejected(status.as_u16(), &b) => {
                            Attempt::SchemaRejected
                        }
                        Ok(b) => Attempt::Fail(anyhow::anyhow!(
                            "Ollama non-streaming request returned error status {status}: {b}"
                        )),
                        Err(err) => Attempt::Fail(err.context(format!(
                            "Ollama non-streaming status {status} body read failed"
                        ))),
                    }
                }
            })
            .await;
            match timed {
                Err(_) => {
                    println!(
                        "[ollama] non-streaming attempt {attempt} timed out after {nonstream_timeout_secs}s"
                    );
                }
                Ok(Attempt::Text(t)) => {
                    println!(
                        "[ollama] received non-empty buffered body ({} bytes)",
                        t.len()
                    );
                    text = t;
                    non_stream_ok = true;
                    println!(
                        "[ollama] timing nonstream attempt {attempt}: total={}ms",
                        ns_attempt_start.elapsed().as_millis()
                    );
                }
                Ok(Attempt::Retry) => {}
                Ok(Attempt::SchemaRejected) => {
                    schema_owned = None;
                }
                Ok(Attempt::Fail(err)) => return Err(err),
            }

            if !non_stream_ok {
                // small backoff before retrying
                tokio::time::sleep(backoff).await;
            }
        }

        // If we didn't get a usable buffered body, or we're in low-latency mode, attempt a streaming read where we
        // accumulate incoming bytes and try to extract the assistant content as it arrives.
        if text.trim().is_empty() || self.low_latency {
            // Build a streaming request (request the server to stream if it supports it)
            let mut send_schema = schema_owned.clone();
            let mut stream_body = serde_json::json!({
                "model": &self.model,
                "messages": [{ "role": "user", "content": prompt }],
                "stream": true,
                "options": { "temperature": 0.1, "num_predict": 512 }
            });
            plan_format::apply_chat_format(
                &mut stream_body,
                send_schema.as_ref(),
                self.force_format_json,
            );
            if let Some(ka) = &self.keep_alive {
                stream_body["keep_alive"] = serde_json::json!(ka);
            }

            // The stored client set `connect_timeout` in `build_model_client`.
            // Header waits and the body share this instant. Each header wait
            // is the idle limit or the time still left, whichever is shorter.
            let start = std::time::Instant::now();
            let lifetime = stream_call_lifetime(DEFAULT_STREAM_LIFETIME);

            let stream_url = format!("{}/api/chat", self.url.trim_end_matches('/'));
            println!("[ollama] initiating streaming POST to {}", stream_url);
            let resp = send_headers_for_call(
                idle,
                lifetime.saturating_sub(start.elapsed()),
                client
                    .post(&stream_url)
                    .header("Accept", "application/json")
                    .header("Content-Type", "application/json")
                    .json(&stream_body)
                    .send(),
            )
            .await
            .context("Failed to initiate streaming request to Ollama")?;

            println!(
                "[ollama] streaming request returned status {}",
                resp.status()
            );

            // Determine which response we'll stream from: prefer /api/chat, but fall back to /api/generate
            let resp_for_stream: Option<reqwest::Response> = if resp.status().is_success() {
                Some(resp)
            } else {
                let status = resp.status();
                if status.as_u16() == 404 || status.as_u16() == 405 {
                    println!(
                        "[ollama] /api/chat returned {}, attempting /api/generate as fallback",
                        status
                    );
                    let gen_url = format!("{}/api/generate", self.url.trim_end_matches('/'));
                    let gen_resp = send_headers_for_call(
                        idle,
                        lifetime.saturating_sub(start.elapsed()),
                        client
                            .post(&gen_url)
                            .header("Accept", "application/json")
                            .header("Content-Type", "application/json")
                            .json(&stream_body)
                            .send(),
                    )
                    .await
                    .context("Failed to initiate streaming request to Ollama /api/generate")?;

                    println!(
                        "[ollama] /api/generate returned status {}",
                        gen_resp.status()
                    );
                    if gen_resp.status().is_success() {
                        Some(gen_resp)
                    } else {
                        let s = gen_resp.status();
                        // The error body shares `start`. Stop at the time
                        // still left, or at 64 KiB, whichever comes first.
                        let b = read_capped_text_within(
                            gen_resp,
                            lifetime.saturating_sub(start.elapsed()),
                        )
                        .await
                        .with_context(|| {
                            format!("Ollama /api/generate status {s} body read failed")
                        })?;
                        if send_schema.is_some()
                            && plan_format::schema_format_rejected(s.as_u16(), &b)
                        {
                            let _ = send_schema.take();
                            if self.force_format_json {
                                stream_body["format"] = serde_json::json!("json");
                            } else if let Some(map) = stream_body.as_object_mut() {
                                map.remove("format");
                            }
                            let retry = send_headers_for_call(
                                idle,
                                lifetime.saturating_sub(start.elapsed()),
                                client
                                    .post(&gen_url)
                                    .header("Accept", "application/json")
                                    .header("Content-Type", "application/json")
                                    .json(&stream_body)
                                    .send(),
                            )
                            .await
                            .context(
                                "Failed to retry Ollama /api/generate without the schema format",
                            )?;
                            if retry.status().is_success() {
                                Some(retry)
                            } else {
                                let retry_status = retry.status();
                                let retry_txt = read_capped_text_within(
                                    retry,
                                    lifetime.saturating_sub(start.elapsed()),
                                )
                                .await
                                .with_context(|| {
                                    format!(
                                        "Ollama /api/generate status {retry_status} body read failed"
                                    )
                                })?;
                                return Err(anyhow::anyhow!(
                                    "Ollama /api/generate returned error status {retry_status}: {retry_txt}"
                                ));
                            }
                        } else {
                            return Err(anyhow::anyhow!(
                                "Ollama /api/generate returned error status {s}: {b}"
                            ));
                        }
                    }
                } else {
                    // The error body shares `start`. Stop at the time
                    // still left, or at 64 KiB, whichever comes first.
                    let txt =
                        read_capped_text_within(resp, lifetime.saturating_sub(start.elapsed()))
                            .await
                            .with_context(|| {
                                format!("Ollama chat API status {status} body read failed")
                            })?;
                    if send_schema.take().is_some()
                        && plan_format::schema_format_rejected(status.as_u16(), &txt)
                    {
                        if self.force_format_json {
                            stream_body["format"] = serde_json::json!("json");
                        } else if let Some(map) = stream_body.as_object_mut() {
                            map.remove("format");
                        }
                        let retry = send_headers_for_call(
                            idle,
                            lifetime.saturating_sub(start.elapsed()),
                            client
                                .post(&stream_url)
                                .header("Accept", "application/json")
                                .header("Content-Type", "application/json")
                                .json(&stream_body)
                                .send(),
                        )
                        .await
                        .context("Failed to initiate streaming request to Ollama")?;
                        if retry.status().is_success() {
                            Some(retry)
                        } else {
                            let retry_status = retry.status();
                            let retry_txt = read_capped_text_within(
                                retry,
                                lifetime.saturating_sub(start.elapsed()),
                            )
                            .await
                            .with_context(|| {
                                format!("Ollama chat API status {retry_status} body read failed")
                            })?;
                            return Err(anyhow::anyhow!(
                                "Ollama chat API returned error status {retry_status}: {retry_txt}"
                            ));
                        }
                    } else {
                        return Err(anyhow::anyhow!(
                            "Ollama chat API returned error status {status}: {txt}"
                        ));
                    }
                }
            };

            // Now unwrap the response we will stream from
            let resp = resp_for_stream
                .ok_or_else(|| anyhow::anyhow!("No streaming response available"))?;

            // Use the bytes_stream to receive chunks as they arrive. Ollama streams envelope JSON
            // objects per-line where assistant tokens are nested inside `message.content`.
            // We must parse each line, extract `message.content` fragments, concatenate them,
            // and then attempt to extract the final JSON object.
            let mut stream = resp.bytes_stream();
            use futures_util::StreamExt;

            let mut raw: Vec<u8> = Vec::new();
            let mut assistant_acc = String::new();
            let mut bytes_seen: usize = 0;
            let mut done_flag = false;
            // Silence stops after one idle period. `overall` caps that same
            // gap, and when it is longer than `idle` the wait is `idle`.
            // A non-empty chunk starts a new gap, so late macOS wakes whose
            // gaps stay inside the idle window are kept. An empty chunk does
            // not. `lifetime` is measured from `start` and is not reset, so
            // a one-byte drip stops there instead of at 1 MiB.
            let overall = idle.saturating_mul(2);
            let mut gap_started = std::time::Instant::now();

            // We'll periodically flush partial assembled assistant output to disk so
            // an interrupted run can still be inspected. Track last flush size.
            #[allow(unused_mut)]
            let mut last_flush = 0usize;
            let mut first_token_at: Option<std::time::Instant> = None;
            let mut body_error: Option<anyhow::Error> = None;
            loop {
                let decision = match next_read_wait(
                    idle,
                    overall,
                    gap_started,
                    lifetime,
                    start,
                    "streaming read idle timeout",
                ) {
                    Ok(decision) => decision,
                    Err(err) => {
                        body_error = Some(err);
                        break;
                    }
                };
                let item = match tokio::time::timeout(decision.wait, stream.next()).await {
                    Ok(Some(item)) => item,
                    Ok(None) => break,
                    Err(_) => {
                        body_error = Some(anyhow::anyhow!(decision.on_timeout));
                        break;
                    }
                };
                match item {
                    Ok(chunk) => {
                        if !chunk.is_empty() {
                            gap_started = std::time::Instant::now();
                        }
                        // Count bytes before UTF-8 or line filtering. Empty
                        // NDJSON lines and invalid UTF-8 must not stream forever.
                        // A cap error keeps tokens already stored in `assistant_acc`.
                        if let Err(err) = note_stream_bytes(&mut bytes_seen, chunk.len()) {
                            body_error = Some(err);
                            break;
                        }
                        raw.extend_from_slice(&chunk);
                        // One pass over bytes already received. The gap
                        // deadline applies to the next wait, not to lines
                        // sitting in `raw`.
                        let mut consumed = 0usize;
                        let mut capped = false;
                        while consumed < raw.len() {
                            let Some(rel) = raw[consumed..].iter().position(|&b| b == b'\n') else {
                                break;
                            };
                            let nl = consumed + rel;
                            let mut end_line = nl;
                            if end_line > consumed && raw[end_line - 1] == b'\r' {
                                end_line -= 1;
                            }
                            let line_bytes = &raw[consumed..end_line];
                            consumed = nl + 1;
                            if line_bytes.is_empty() {
                                continue;
                            }
                            // Incomplete scalars stay in `raw` until a later chunk
                            // finishes the line, so a split UTF-8 character is kept.
                            let line = match std::str::from_utf8(line_bytes) {
                                Ok(line) => line,
                                Err(_) => {
                                    body_error =
                                        Some(anyhow::anyhow!("model stream line is not utf-8"));
                                    capped = true;
                                    break;
                                }
                            };
                            let line = line.trim();
                            if line.is_empty() {
                                continue;
                            }
                            if first_token_at.is_none() {
                                first_token_at = Some(std::time::Instant::now());
                            }
                            if let Err(err) =
                                push_stream_line(&mut assistant_acc, line, &mut done_flag)
                            {
                                body_error = Some(err);
                                capped = true;
                                break;
                            }
                        }
                        if consumed == raw.len() {
                            raw.clear();
                        } else if consumed > 0 {
                            raw.drain(..consumed);
                        }
                        if capped {
                            break;
                        }
                        // Periodically persist partial output so it's available if the
                        // process is interrupted.
                        // Flush eagerly when we have some new content (>256 bytes)
                        if assistant_acc.len().saturating_sub(last_flush) > 256 {
                            #[cfg(feature = "debug_io")]
                            {
                                if let Err(e) = std::fs::write(
                                    "target/ollama_assistant_acc.txt",
                                    &assistant_acc,
                                ) {
                                    debug!("[ollama] partial write failed: {}", e);
                                } else {
                                    last_flush = assistant_acc.len();
                                    debug!(
                                        "[ollama] flushed {} bytes of assistant_acc to disk",
                                        last_flush
                                    );
                                }
                            }
                        }

                        // If we've accumulated something that looks like a JSON object, try to extract it
                        if let Some(obj) = extract_json_object(&assistant_acc) {
                            if self.early_exit_on_json {
                                let ttfb_ms = first_token_at
                                    .map(|ft| ft.duration_since(start).as_millis())
                                    .unwrap_or(0);
                                let total_ms = start.elapsed().as_millis();
                                debug!(
                                    "[ollama] timing stream: ttfb={}ms, total={}ms (early-exit)",
                                    ttfb_ms, total_ms
                                );
                                return finish_capped(obj);
                            }
                            text = obj;
                            break;
                        }
                    }
                    Err(e) => {
                        body_error = Some(anyhow::Error::new(e).context("streaming body error"));
                        break;
                    }
                }

                if done_flag {
                    break;
                }
            }
            // A lifetime deadline or an idle timeout after at least one token
            // returns those tokens. The error is returned only when the
            // stream produced none. A UTF-8 or cap error follows the same
            // rule: tokens already stored stay.
            if text.trim().is_empty() && assistant_acc.trim().is_empty() {
                if let Some(err) = body_error {
                    return Err(err);
                }
            }

            // Ensure we always flush a final snapshot of assembled content even if small
            if !assistant_acc.is_empty() && assistant_acc.len() > last_flush {
                #[cfg(feature = "debug_io")]
                {
                    let _ = std::fs::write("target/ollama_assistant_acc.txt", &assistant_acc);
                }
            }

            if text.trim().is_empty() {
                // If streaming collected something but we couldn't parse JSON, try to salvage text.
                // Some Ollama variants deliver the assistant output as a quoted JSON string (i.e. a
                // JSON string literal containing another JSON object). Detect and unquote that.
                if !assistant_acc.trim().is_empty() {
                    let mut candidate = assistant_acc.clone();
                    // If the assistant_acc looks like a quoted JSON string, unescape it
                    let t = candidate.trim();
                    if t.starts_with('"') && t.ends_with('"') {
                        if let Ok(unq) = serde_json::from_str::<String>(t) {
                            candidate = unq;
                        }
                    }

                    // Debug: persist the assembled assistant output to a file for offline inspection
                    debug!(
                        "[ollama] assembled assistant_acc length = {}",
                        candidate.len()
                    );
                    if let Some((start_snip, end_snip)) =
                        ollama_assembled_debug_snippets(&candidate)
                    {
                        debug!("[ollama] assembled snippet (start): {}", start_snip);
                        debug!("[ollama] assembled snippet (end): {}", end_snip);
                    } else {
                        debug!("[ollama] assembled content: {}", candidate);
                    }
                    #[cfg(feature = "debug_io")]
                    {
                        if let Err(e) =
                            std::fs::write("target/ollama_assistant_acc.txt", &candidate)
                        {
                            debug!("[ollama] failed to write assistant_acc to file: {}", e);
                        } else {
                            debug!("[ollama] wrote assembled assistant_acc to target/ollama_assistant_acc.txt");
                        }
                    }

                    if let Some(obj) = extract_last_json_object(&candidate)
                        .or_else(|| extract_json_object(&candidate))
                    {
                        debug!("[ollama] extracted JSON object (len={})", obj.len());
                        if self.early_exit_on_json {
                            let ttfb_ms = first_token_at
                                .map(|ft| ft.duration_since(start).as_millis())
                                .unwrap_or(0);
                            let total_ms = start.elapsed().as_millis();
                            debug!("[ollama] timing stream: ttfb={}ms, total={}ms (early-exit salvage)", ttfb_ms, total_ms);
                            return finish_capped(obj);
                        }
                        text = obj;
                    } else {
                        text = strip_code_fences(&candidate);
                    }
                } else if let Ok(s) = std::str::from_utf8(&raw) {
                    if let Some(obj) = extract_json_object(s) {
                        if self.early_exit_on_json {
                            let ttfb_ms = first_token_at
                                .map(|ft| ft.duration_since(start).as_millis())
                                .unwrap_or(0);
                            let total_ms = start.elapsed().as_millis();
                            debug!(
                                "[ollama] timing stream: ttfb={}ms, total={}ms (early-exit buffer)",
                                ttfb_ms, total_ms
                            );
                            return finish_capped(obj);
                        }
                        text = obj;
                    } else {
                        text = strip_code_fences(s);
                    }
                }
            }

            // If we’re here, we didn’t early-exit; log final timings for stream path
            let ttfb_ms = first_token_at
                .map(|ft| ft.duration_since(start).as_millis())
                .unwrap_or(0);
            let total_ms = start.elapsed().as_millis();
            debug!(
                "[ollama] timing stream: ttfb={}ms, total={}ms",
                ttfb_ms, total_ms
            );

            if text.trim().is_empty() {
                bail!("Ollama chat did not return a usable response within timeout. Check `ollama ps` and model readiness.");
            }
        }

        // `text` now contains the response body retrieved via retry/backoff above.

        // Try parsing as JSON value to extract common shapes
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            // 1) { "response": "..." }
            if let Some(resp) = v.get("response").and_then(|r| r.as_str()) {
                return finish_capped(resp.to_string());
            }

            // 2) { "message": { "content": "..." } }
            if let Some(msg) = v.get("message") {
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    // If content contains a fenced code block or an embedded JSON object,
                    // try to extract and return that JSON directly to simplify downstream parsing.
                    if let Some(obj) = extract_json_object(content) {
                        return finish_capped(obj);
                    }
                    let stripped = strip_code_fences(content);
                    return finish_capped(stripped);
                }
            }

            // 3) { "choices": [ { "message": { "content": "..." } } ] }
            if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
                if let Some(first) = choices.first() {
                    if let Some(msg) = first.get("message") {
                        if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                            return finish_capped(content.to_string());
                        }
                    }
                }
            }
        }

        // If no recognized JSON shape matched, return the raw text as a last resort.
        // This covers a body with no newline, which never reached `append_capped`.
        finish_capped(text)
    }
}

#[cfg(feature = "ollama")]
#[async_trait::async_trait]
impl LlmClient for OllamaChatClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        self.complete_with_optional_format(prompt, None).await
    }

    async fn complete_with_format(
        &self,
        prompt: &str,
        format: &serde_json::Value,
    ) -> Result<String> {
        self.complete_with_optional_format(prompt, Some(format))
            .await
    }
}

/// A simple local HTTP LLM client that can work with any OpenAI-compatible API
/// This includes local services like text-generation-webui, LocalAI, etc.
#[cfg(feature = "ollama")]
pub struct LocalHttpClient {
    url: String,
    allow_remote: bool,
    pub model: String,
    pub api_key: Option<String>,
    client: reqwest::Client,
}

#[cfg(feature = "ollama")]
impl LocalHttpClient {
    /// Stored endpoint. The first hop is checked again before each request.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Create a client. A URL that would be rewritten is an error, and no key is stored.
    /// Use [`Self::allowing_remote`] to keep a caller-supplied host on reqwest's
    /// default client (proxy env and redirects). A loopback URL still uses
    /// [`loopback_http_client`].
    pub fn new(url: String, model: String) -> Result<Self> {
        Self::assemble(url, model, None, false)
    }

    /// Create a client with an API key. The URL is loopback-only, same as [`Self::new`].
    /// A URL that would be rewritten returns an error and does not store `api_key`.
    pub fn with_api_key(url: String, model: String, api_key: String) -> Result<Self> {
        Self::assemble(url, model, Some(api_key), false)
    }

    /// Keep `url` even when it is not loopback. This is the explicit opt-in.
    ///
    /// A non-loopback URL uses reqwest's default client: `HTTP_PROXY`,
    /// `HTTPS_PROXY`, and `ALL_PROXY` apply, and redirects are followed. A
    /// loopback URL still uses [`loopback_http_client`]. Userinfo is an error.
    pub fn allowing_remote(url: String, model: String) -> Result<Self> {
        Self::assemble(url, model, None, true)
    }

    /// [`Self::allowing_remote`] plus a bearer token. A rejected URL does not store the key.
    pub fn allowing_remote_with_api_key(
        url: String,
        model: String,
        api_key: String,
    ) -> Result<Self> {
        Self::assemble(url, model, Some(api_key), true)
    }

    fn assemble(
        url: String,
        model: String,
        api_key: Option<String>,
        allow_remote: bool,
    ) -> Result<Self> {
        let url = require_kept_model_url(url, allow_remote)?;
        let client = build_model_client(&url, allow_remote)?;
        Ok(Self {
            url,
            allow_remote,
            model,
            api_key,
            client,
        })
    }
}

/// `http` or `https` URL whose host is loopback: `127.0.0.0/8`, `::1`, or `localhost`.
///
/// Userinfo is rejected, including an empty `@`. This is the only host check.
/// Socket binds use `IpAddr::is_loopback` after `SocketAddr` parsing, which is
/// the same `127.0.0.0/8` and `::1` rule this function applies to a parsed host.
pub fn is_loopback_http_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }
    if url_has_userinfo(url, &parsed) {
        return false;
    }
    match parsed.host() {
        Some(url::Host::Domain(domain)) => is_localhost_domain(domain),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

/// `true` when `url` carries userinfo, including an empty `@` the parser drops.
///
/// An unparseable string is not userinfo. Callers that must also require
/// `http` or `https` use [`loopback_model_url`] or [`is_loopback_http_url`].
pub fn model_url_has_userinfo(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    url_has_userinfo(url, &parsed)
}

/// `http://127.0.0.1:11434@evil.example/` puts the loopback text in the userinfo
/// and sends the request to `evil.example`. An empty `@` is userinfo too, even
/// when the parser drops it and keeps a loopback host.
fn url_has_userinfo(raw: &str, parsed: &url::Url) -> bool {
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return true;
    }
    let Some(scheme_sep) = raw.find("://") else {
        return true;
    };
    let rest = &raw[scheme_sep + 3..];
    let authority = rest.split(['/', '?', '#', '\\']).next().unwrap_or("");
    authority.contains('@')
}

fn is_localhost_domain(domain: &str) -> bool {
    let bare = domain.trim_end_matches('.');
    bare.eq_ignore_ascii_case("localhost") && domain.len() - bare.len() <= 1
}

#[cfg(feature = "ollama")]
#[async_trait::async_trait]
impl LlmClient for LocalHttpClient {
    async fn complete(&self, prompt: &str) -> Result<String> {
        ensure_model_request(&self.url, self.allow_remote)?;
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Message {
            role: String,
            content: String,
        }

        #[derive(serde::Serialize)]
        struct Req {
            model: String,
            messages: Vec<Message>,
            max_tokens: u32,
            temperature: f32,
        }

        #[derive(serde::Deserialize)]
        struct Choice {
            message: Message,
        }

        #[derive(serde::Deserialize)]
        struct Resp {
            choices: Vec<Choice>,
        }

        let body = Req {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 2048,
            temperature: 0.1, // Low temperature for more consistent JSON output
        };

        let mut request = self
            .client
            .post(format!("{}/v1/chat/completions", self.url))
            .header("Content-Type", "application/json");

        if let Some(api_key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = send_for_headers(stream_read_idle(), request.json(&body).send())
            .await
            .context("Failed to send request to local LLM")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = read_capped_text(response)
                .await
                .with_context(|| format!("Local LLM API status {status} body read failed"))?;
            bail!("Local LLM API returned error status {status}: {text}");
        }

        let parsed: Resp = read_capped_json(response)
            .await
            .context("Failed to parse local LLM response")?;

        if parsed.choices.is_empty() {
            bail!("Local LLM returned no choices");
        }

        Ok(parsed.choices[0].message.content.clone())
    }
}

/// Build an instruction that forces JSON output conforming to PlanIntent.
pub fn build_prompt(snap: &WorldSnapshot, reg: &ToolRegistry) -> String {
    let tool_list = reg
        .tools
        .iter()
        .map(|t| format!(" - {} {:?}", t.name, t.args))
        .collect::<Vec<_>>()
        .join("\n");
    let schema = r#"
Strict JSON schema:
{
  "plan_id": "string",
  "steps": [
     {"act":"MoveTo","x":INT,"y":INT} |
     {"act":"Throw","item":"smoke|grenade","x":INT,"y":INT} |
     {"act":"CoverFire","target_id":INT,"duration":FLOAT} |
     {"act":"Revive","ally_id":INT}
  ]
}
Return ONLY JSON with no commentary.
"#;
    format!(
        r#"You are an AI game companion planner. Convert the world snapshot into a legal action plan.
Use ONLY allowed tools and arguments. Do not exceed cooldown or LOS checks (the engine will validate).
Allowed tools:
{tools}

Snapshot (redacted):
{snap}

{schema}"#,
        tools = tool_list,
        snap = serde_json::to_string_pretty(snap).unwrap_or_default(),
        schema = schema
    )
}

/// Parse and validate that the produced steps are in the allowed registry (structural check).
pub fn parse_llm_plan(json_text: &str, reg: &ToolRegistry) -> Result<PlanIntent> {
    // Try direct parse first
    if let Ok(plan) = serde_json::from_str::<PlanIntent>(json_text.trim()) {
        validate_plan(&plan, reg)?;
        return Ok(plan);
    }

    // Strip common code fences and try again
    let cleaned = strip_code_fences(json_text);
    // If there's fenced JSON like ```json { ... } ``` try to extract inner JSON first
    if let Some(fenced) = extract_json_from_fenced(json_text) {
        if let Ok(plan) = serde_json::from_str::<PlanIntent>(fenced.trim()) {
            validate_plan(&plan, reg)?;
            return Ok(plan);
        }
        // try cleaned fenced
        if let Some(inner_clean) = extract_json_from_fenced(&cleaned) {
            if let Ok(plan) = serde_json::from_str::<PlanIntent>(inner_clean.trim()) {
                validate_plan(&plan, reg)?;
                return Ok(plan);
            }
        }
    }
    if let Ok(plan) = serde_json::from_str::<PlanIntent>(cleaned.as_str()) {
        validate_plan(&plan, reg)?;
        return Ok(plan);
    }

    // Attempt to extract the last JSON object from the text and parse it
    if let Some(obj) = extract_last_json_object(&cleaned) {
        if let Ok(plan) = serde_json::from_str::<PlanIntent>(obj.trim()) {
            validate_plan(&plan, reg)?;
            return Ok(plan);
        }
    }

    // Attempt to extract the first JSON object from the text and parse it
    if let Some(obj) = extract_json_object(&cleaned) {
        if let Ok(plan) = serde_json::from_str::<PlanIntent>(obj.trim()) {
            validate_plan(&plan, reg)?;
            return Ok(plan);
        }
    }

    // Try to obtain a JSON Value from several candidates for tolerant extraction
    let v_opt: Option<serde_json::Value> =
        serde_json::from_str::<serde_json::Value>(cleaned.as_str())
            .ok()
            .or_else(|| {
                extract_last_json_object(json_text).and_then(|s| serde_json::from_str(&s).ok())
            })
            .or_else(|| {
                extract_last_json_object(&cleaned).and_then(|s| serde_json::from_str(&s).ok())
            })
            .or_else(|| extract_json_object(&cleaned).and_then(|s| serde_json::from_str(&s).ok()));

    if let Some(v) = &v_opt {
        // Try to locate a nested JSON inside `message.content` or `response`
        if let Some(msg) = v.get("message") {
            if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                // Try to parse content as JSON directly
                if let Ok(plan) = serde_json::from_str::<PlanIntent>(content.trim()) {
                    validate_plan(&plan, reg)?;
                    return Ok(plan);
                }
                // Try to extract JSON from the content string
                if let Some(obj2) = extract_json_object(content) {
                    if let Ok(plan) = serde_json::from_str::<PlanIntent>(obj2.trim()) {
                        validate_plan(&plan, reg)?;
                        return Ok(plan);
                    }
                }
            }
        }

        if let Some(resp_txt) = v.get("response").and_then(|r| r.as_str()) {
            if let Some(obj2) = extract_json_object(resp_txt) {
                if let Ok(plan) = serde_json::from_str::<PlanIntent>(obj2.trim()) {
                    validate_plan(&plan, reg)?;
                    return Ok(plan);
                }
            }
        }
    }

    // Coerce tolerant PlanIntent: accept alternative keys and ensure steps exist
    if let Some(v) = v_opt {
        let plan_id = (|| {
            // common accepted keys
            let candidates = [
                "plan_id",
                "plan_eid",
                "id",
                "plan_no",
                "plan_num",
                "planNumber",
                "plan_n°",
                "plan_n",
            ];
            for &k in &candidates {
                if let Some(vv) = v.get(k) {
                    if let Some(s) = vv.as_str() {
                        return Some(s.to_string());
                    }
                }
            }

            // Try to find any key that, when normalized, matches "planid" or similar
            if let Some(obj) = v.as_object() {
                for (k, vv) in obj.iter() {
                    let norm: String = k
                        .chars()
                        .filter(|c| c.is_alphanumeric())
                        .collect::<String>()
                        .to_lowercase();
                    if norm == "planid"
                        || norm == "plann"
                        || norm == "planno"
                        || norm == "plannumber"
                    {
                        if let Some(s) = vv.as_str() {
                            return Some(s.to_string());
                        }
                    }
                }
            }

            None
        })();

        let plan_id = match plan_id {
            Some(id) => id,
            None => return Err(anyhow::anyhow!("No valid plan_id found in LLM response")),
        };

        let steps_val = v
            .get("steps")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![]));
        let steps_json = serde_json::to_string(&steps_val)?;
        let steps: Vec<ActionStep> = serde_json::from_str(&steps_json)?;

        let plan = PlanIntent { plan_id, steps };
        validate_plan(&plan, reg)?;
        return Ok(plan);
    }

    Err(anyhow::anyhow!(
        "Failed to parse LLM plan from text (no valid JSON object found)"
    ))
}

fn strip_code_fences(s: &str) -> String {
    // Remove triple backtick code fences and return inner content if found
    if let Some(start) = s.find("```") {
        if let Some(end_rel) = s[start + 3..].find("```") {
            let inner = &s[start + 3..start + 3 + end_rel];
            return inner.trim().to_string();
        }
    }
    s.to_string()
}

fn validate_plan(plan: &PlanIntent, reg: &ToolRegistry) -> Result<()> {
    for s in &plan.steps {
        match s {
            ActionStep::MoveTo { .. } => {
                if !reg.tools.iter().any(|t| t.name == "MoveTo") {
                    bail!("LLM used disallowed tool MoveTo");
                }
            }
            ActionStep::Throw { .. } => {
                if !reg.tools.iter().any(|t| t.name == "Throw") {
                    bail!("LLM used disallowed tool Throw");
                }
            }
            ActionStep::CoverFire { .. } => {
                if !reg.tools.iter().any(|t| t.name == "CoverFire") {
                    bail!("LLM used disallowed tool CoverFire");
                }
            }
            ActionStep::Revive { .. } => {
                if !reg.tools.iter().any(|t| t.name == "Revive") {
                    bail!("LLM used disallowed tool Revive");
                }
            }
            // The four arms above stay. Every other variant is the same registry check.
            _ => {
                let name = s.action_name();
                if !reg.tools.iter().any(|t| t.name == name) {
                    bail!("LLM used disallowed tool {name}");
                }
            }
        }
    }
    Ok(())
}

/// Maximum allowed coordinate value for plan sanitization
const MAX_COORD_BOUND: i32 = 100;

/// `i32::abs` panics on `i32::MIN` in debug and wraps in release.
/// The bound uses the unsigned magnitude so both profiles drop that coordinate.
fn coord_within_bound(value: i32) -> bool {
    value.unsigned_abs() <= MAX_COORD_BOUND as u32
}

/// Sanitize and validate plan for safety
pub fn sanitize_plan(
    plan: &mut PlanIntent,
    snap: &WorldSnapshot,
    reg: &ToolRegistry,
) -> Result<()> {
    // Remove any steps that exceed bounds or use invalid targets
    plan.steps.retain(|step| match step {
        ActionStep::MoveTo { x, y, speed: _ } => {
            // Check bounds (example: within 100 units)
            coord_within_bound(*x)
                && coord_within_bound(*y)
                && reg.tools.iter().any(|t| t.name == "MoveTo")
        }
        ActionStep::Throw { item, x, y } => {
            // Check item is allowed
            matches!(item.as_str(), "smoke" | "grenade")
                && coord_within_bound(*x)
                && coord_within_bound(*y)
                && reg.tools.iter().any(|t| t.name == "Throw")
        }
        ActionStep::CoverFire {
            target_id,
            duration,
        } => {
            // Check target exists and duration reasonable
            snap.enemies.iter().any(|e| e.id == *target_id)
                && *duration > 0.0
                && *duration <= 10.0
                && reg.tools.iter().any(|t| t.name == "CoverFire")
        }
        ActionStep::Revive { ally_id: _ } => {
            // Check ally exists (simplified: allow any ally for now, or validate against known ally IDs)
            reg.tools.iter().any(|t| t.name == "Revive")
        }
        // Unknown actions stay out unless the registry names them.
        _ => reg.tools.iter().any(|t| t.name == step.action_name()),
    });
    Ok(())
}

/// Attempt to extract the first JSON object from a text blob by finding a balanced
/// '{' ... '}' region, respecting string literals and escapes.
fn extract_json_object(s: &str) -> Option<String> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut start = None;

    for (i, c) in s.char_indices() {
        if in_string {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
        } else {
            match c {
                '{' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    if depth > 0 {
                        depth -= 1;
                        if depth == 0 {
                            if let Some(start_idx) = start {
                                return Some(s[start_idx..=i].to_string());
                            }
                        }
                    }
                }
                '"' => in_string = true,
                _ => {}
            }
        }
    }
    None
}

/// Char-safe 200-byte start/end previews for long assembled Ollama stream text.
#[cfg(feature = "ollama")]
pub(crate) fn ollama_assembled_debug_snippets(candidate: &str) -> Option<(String, String)> {
    if candidate.len() <= 500 {
        return None;
    }
    Some((
        byte_prefix(candidate, 200).to_string(),
        byte_suffix(candidate, 200).to_string(),
    ))
}

/// Extract the last balanced JSON object in a string, if any.
fn extract_last_json_object(s: &str) -> Option<String> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut start = None;
    let mut last_match = None;

    for (i, c) in s.char_indices() {
        if in_string {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
        } else {
            match c {
                '{' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    if depth > 0 {
                        depth -= 1;
                        if depth == 0 {
                            if let Some(start_idx) = start {
                                last_match = Some(s[start_idx..=i].to_string());
                                // Reset start to find next object
                                start = None;
                            }
                        }
                    }
                }
                '"' => in_string = true,
                _ => {}
            }
        }
    }
    last_match
}

/// Find JSON inside fenced code blocks (```json ... ``` or ``` ... ```) and return inner content
fn extract_json_from_fenced(s: &str) -> Option<String> {
    // look for ```json first
    if let Some(start) = s.find("```json") {
        if let Some(end_rel) = s[start + 7..].find("```") {
            let inner = &s[start + 7..start + 7 + end_rel];
            return Some(inner.trim().to_string());
        }
    }
    // fallback to any ``` block
    if let Some(start) = s.find("```") {
        if let Some(end_rel) = s[start + 3..].find("```") {
            let inner = &s[start + 3..start + 3 + end_rel];
            return Some(inner.trim().to_string());
        }
    }
    None
}

/// Generate a plan using LLM with Phase 7 multi-tier fallback system
///
/// This function uses a 4-tier fallback chain:
/// 1. Full LLM with all 37 tools
/// 2. Simplified LLM with 10 most common tools
/// 3. Heuristic rule-based planning
/// 4. Emergency safe default (Scan + Wait)
///
/// Cache integration is preserved for performance.
pub async fn plan_from_llm(
    client: &dyn LlmClient,
    snap: &WorldSnapshot,
    reg: &ToolRegistry,
) -> PlanSource {
    #[cfg(feature = "llm_cache")]
    {
        // Build cache key for fast lookup
        let prompt = build_prompt(snap, reg);
        let tool_names: Vec<&str> = reg.tools.iter().map(|t| t.name.as_str()).collect();
        let cache_key = PromptKey::new(
            &prompt,
            "default", // TODO: Extract from client
            0.7,       // TODO: Extract temperature from client
            &tool_names,
        );

        // Check cache first
        if let Some((cached_plan, _decision)) = GLOBAL_CACHE.get(&cache_key) {
            debug!(
                "[plan_from_llm] Cache HIT - returning cached plan: {}",
                cached_plan.plan.plan_id
            );

            return PlanSource::Llm(cached_plan.plan);
        }

        debug!("[plan_from_llm] Cache MISS - calling fallback orchestrator");
    }

    // Cache miss or disabled - use Phase 7 multi-tier fallback
    use crate::fallback_system::FallbackOrchestrator;

    let orchestrator = FallbackOrchestrator::new();
    let result = orchestrator.plan_with_fallback(client, snap, reg).await;

    info!(
        "[plan_from_llm] Fallback orchestrator succeeded at tier {} after {} attempts ({} ms)",
        result.tier.as_str(),
        result.attempts.len(),
        result.total_duration_ms
    );

    // Cache successful LLM plans (Tier 1 or Tier 2)
    #[cfg(feature = "llm_cache")]
    {
        if result.tier <= fallback_system::FallbackTier::SimplifiedLlm {
            let prompt = build_prompt(snap, reg);
            let tool_names: Vec<&str> = reg.tools.iter().map(|t| t.name.as_str()).collect();
            let cache_key = PromptKey::new(&prompt, "default", 0.7, &tool_names);
            let cached_plan = CachedPlan {
                plan: result.plan.clone(),
                created_at: std::time::Instant::now(),
                tokens_saved: estimate_tokens(&prompt),
            };
            GLOBAL_CACHE.put(cache_key, cached_plan);

            debug!("[plan_from_llm] Cached new plan: {}", result.plan.plan_id);
        }
    }

    // Return appropriate PlanSource based on tier
    match result.tier {
        fallback_system::FallbackTier::FullLlm | fallback_system::FallbackTier::SimplifiedLlm => {
            PlanSource::Llm(result.plan)
        }
        fallback_system::FallbackTier::Heuristic | fallback_system::FallbackTier::Emergency => {
            PlanSource::Fallback {
                plan: result.plan,
                reason: format!(
                    "Used {} tier after {} attempts",
                    result.tier.as_str(),
                    result.attempts.len()
                ),
            }
        }
    }
}

/// Estimate token count from prompt (rough approximation: 4 chars per token)
fn estimate_tokens(prompt: &str) -> u32 {
    (prompt.len() / 4) as u32
}

/// Fallback heuristic plan when LLM fails - simple move towards objective or enemies
pub fn fallback_heuristic_plan(snap: &WorldSnapshot, reg: &ToolRegistry) -> PlanIntent {
    let mut steps = Vec::new();

    // If there's an objective, try to move towards it (simplified)
    if let Some(obj) = &snap.objective {
        if obj == "extract" {
            // Move towards player if far
            let dist = (snap.me.pos.x - snap.player.pos.x).abs()
                + (snap.me.pos.y - snap.player.pos.y).abs();
            // Check for both PascalCase (ActionStep) and snake_case (legacy) tool names
            if dist > 3
                && reg
                    .tools
                    .iter()
                    .any(|t| t.name == "MoveTo" || t.name == "move_to")
            {
                steps.push(ActionStep::MoveTo {
                    x: snap.player.pos.x,
                    y: snap.player.pos.y,
                    speed: None,
                });
            }
        }
    }

    // If enemies nearby, provide cover fire
    // Check for both PascalCase and snake_case tool names
    if !snap.enemies.is_empty()
        && reg
            .tools
            .iter()
            .any(|t| t.name == "CoverFire" || t.name == "cover_fire")
    {
        let enemy = &snap.enemies[0];
        steps.push(ActionStep::CoverFire {
            target_id: enemy.id,
            duration: 2.0,
        });
    }

    PlanIntent {
        plan_id: "heuristic-fallback".to_string(),
        steps,
    }
}

/// Get cache statistics (if caching is enabled)
#[cfg(feature = "llm_cache")]
pub fn get_cache_stats() -> cache::CacheStats {
    GLOBAL_CACHE.stats()
}

#[cfg(not(feature = "llm_cache"))]
pub fn get_cache_stats() -> () {
    ()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_json_over_64_kib_is_rejected() {
        let text = "x".repeat(MAX_MODEL_JSON_BYTES + 1);
        let err = parse_bounded_model_json::<serde_json::Value>(&text).expect_err("over cap");
        let msg = format!("{err:#}");
        assert!(msg.contains("64 KiB"), "{msg}");
    }

    #[test]
    fn ollama_client_source_does_not_dump_the_prompt() {
        let src = include_str!("lib.rs");
        let (prod, _) = src.split_once("mod tests").expect("test module");
        let prompt_banner = format!("{}{}", "PROMPT SENT TO ", "LOCAL LLM");
        let response_banner = format!("{}{}", "LOCAL LLM RAW ", "RESPONSE");
        assert!(!prod.contains(prompt_banner.as_str()));
        assert!(!prod.contains(response_banner.as_str()));
        assert!(!prod.contains("eprintln!"));
    }

    #[cfg(feature = "ollama")]
    #[test]
    fn other_ollama_clients_keep_only_loopback_urls() {
        let remote = "https://api.openai.com";
        assert!(qwen3_ollama::Qwen3Ollama::new(remote, "qwen").is_err());
        assert!(phi3_ollama::Phi3Ollama::new(remote, "phi3").is_err());
        assert!(hermes2pro_ollama::Hermes2ProOllama::new(remote, "hermes").is_err());
        assert!(OllamaChatClient::new(remote.to_string(), "chat".to_string()).is_err());
        let opted = qwen3_ollama::Qwen3Ollama::allowing_remote(remote, "qwen").expect("client");
        assert_eq!(opted.url(), remote);
    }

    #[test]
    fn loopback_model_url_rejects_userinfo_encoding_and_non_loopback_literals() {
        const FALLBACK: &str = "http://127.0.0.1:11434";
        // Decimal, octal, hex, and short IPv4 forms are kept only when the
        // WHATWG parser (the same one reqwest uses) resolves them inside
        // 127.0.0.0/8. `0177.0.0.1` is octal for 127.0.0.1. `0127.0.0.1` is
        // octal for 87.0.0.1.
        let kept = [
            "http://127.0.0.1:11434",
            "https://127.0.0.1/api",
            "http://127.0.0.2:11434",
            "http://127.255.255.255/",
            "HTTP://127.0.0.1:11434",
            "http://localhost:11434",
            "http://LOCALHOST:11434",
            "http://localhost.:11434",
            "https://localhost/v1",
            "http://[::1]:11434/api",
            "http://[0:0:0:0:0:0:0:1]/",
            "http://2130706433/",
            "http://2130706434:11434/",
            "http://0177.0.0.1:11434/",
            "http://017700000001/",
            "http://0x7f.0.0.1/",
            "http://0x7f000001/",
            "http://127.1/",
            "http://127.0.1/",
            "http://127%2e0%2e0%2e1:11434/",
            "http://%6c%6f%63%61%6c%68%6f%73%74/",
            "http://127.0.0.1:11434/api?q=@evil.example",
            "http://127.0.0.1:11434/api#@evil.example",
            "http://127.0.0.1:11434\\@evil.example",
        ];
        for url in kept {
            assert_eq!(
                loopback_model_url(url.to_string(), false),
                url,
                "expected to keep {url}"
            );
        }

        let rejected = [
            "http://127.0.0.1:11434@evil.example/api/chat",
            "http://127.0.0.1:11434@evil.example",
            "https://127.0.0.1:11434@evil.example/v1",
            "https://user:pass@127.0.0.1:11434/",
            "http://user@127.0.0.1/",
            "http://:secret@127.0.0.1/",
            "http://@127.0.0.1:11434/",
            "http://:@127.0.0.1/",
            "http://127.0.0.1%40evil.example/",
            "http://127.0.0.1%2eevil.example/",
            "http://localhost%2eevil.example/",
            "http://%6c%6f%63%61%6c%68%6f%73%74%2eevil.example/",
            "http://127.0.0.1%00.evil.example/",
            "http://127%2e0%2e0%2e1.evil.example/",
            "http://[::2]/",
            "http://[::ffff:127.0.0.1]/",
            "http://[::ffff:7f00:1]/",
            "http://[::1%25eth0]/",
            "http://[2001:db8::1]/",
            "http://0127.0.0.1/",
            "http://012700000001/",
            "http://167772161/",
            "http://134744072/",
            "http://0/",
            "http://10.0.0.1/",
            "http://0x08080808/",
            "http://2130706433@evil.example/",
            "http://0177.0.0.1@evil.example/",
            "http://127.0.0.1.evil.example/",
            "http://sub.localhost/",
            "http://evil.example/",
            "file:///etc/passwd",
            "127.0.0.1:11434",
            "",
        ];
        for url in rejected {
            assert_eq!(
                loopback_model_url(url.to_string(), false),
                FALLBACK,
                "expected to replace {url}"
            );
        }

        let remote = "http://127.0.0.1:11434@evil.example/api";
        assert_eq!(loopback_model_url(remote.to_string(), true), FALLBACK);
        assert!(model_url_has_userinfo(remote));
        assert!(model_url_has_userinfo("http://@127.0.0.1:11434/"));
        assert!(model_url_has_userinfo("http://:@127.0.0.1/"));
        assert!(!model_url_has_userinfo("https://api.openai.com"));
        assert!(!model_url_has_userinfo("http://127.0.0.1:11434"));
        assert_eq!(
            loopback_model_url("https://api.openai.com".to_string(), true),
            "https://api.openai.com"
        );
    }

    #[cfg(feature = "ollama")]
    #[test]
    fn ensure_model_request_rejects_userinfo_even_when_remote() {
        assert!(ensure_model_request("http://127.0.0.1:11434", false).is_ok());
        assert!(ensure_model_request("https://api.openai.com", true).is_ok());
        let userinfo =
            ensure_model_request("http://user:pass@api.openai.com/v1", true).expect_err("userinfo");
        assert!(userinfo.to_string().contains("userinfo"), "{userinfo}");
        let remote = ensure_model_request("https://api.openai.com", false).expect_err("remote");
        assert!(remote.to_string().contains("not loopback"), "{remote}");
    }

    #[cfg(feature = "ollama")]
    #[tokio::test]
    async fn request_rechecks_the_stored_url() {
        let mut client = OllamaClient::new("http://127.0.0.1:11434", "m").expect("client");
        client.url = "http://user:pass@evil.example/api".to_string();
        client.allow_remote = true;
        let err = client.complete("hi").await.expect_err("userinfo");
        assert!(err.to_string().contains("userinfo"), "{err}");

        let mut local = LocalHttpClient::new("http://127.0.0.1:11434".to_string(), "m".to_string())
            .expect("client");
        local.url = "https://api.openai.com".to_string();
        let err = local.complete("hi").await.expect_err("remote");
        assert!(err.to_string().contains("not loopback"), "{err}");
    }

    #[cfg(feature = "ollama")]
    #[test]
    fn loopback_http_client_rejects_remote_redirects() {
        use std::io::{Read, Write};

        fn spawn(response: String) -> std::net::SocketAddr {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
            let addr = listener.local_addr().expect("addr");
            std::thread::spawn(move || {
                if let Ok((mut sock, _)) = listener.accept() {
                    let _ = sock.set_read_timeout(Some(std::time::Duration::from_secs(2)));
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf);
                    let _ = sock.write_all(response.as_bytes());
                }
            });
            addr
        }

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let remote = spawn(
            "HTTP/1.1 302 Found\r\nLocation: http://example.com/steal\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_string(),
        );
        let redirect_err = runtime.block_on(async {
            loopback_http_client()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .expect("redirect client")
                .get(format!("http://{remote}/"))
                .send()
                .await
        });
        let redirect_err = redirect_err.expect_err("remote redirect must fail");
        let mut redirect_msg = redirect_err.to_string();
        let mut source = std::error::Error::source(&redirect_err);
        while let Some(err) = source {
            redirect_msg.push_str(" | ");
            redirect_msg.push_str(&err.to_string());
            source = err.source();
        }
        assert!(
            redirect_msg.contains("redirect is not loopback"),
            "{redirect_msg}"
        );

        let next = spawn(
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_string(),
        );
        let hop = spawn(format!(
            "HTTP/1.1 302 Found\r\nLocation: http://{next}/ok\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ));
        let followed = runtime.block_on(async {
            loopback_http_client()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .expect("follow client")
                .get(format!("http://{hop}/"))
                .send()
                .await
                .expect("loopback redirect")
                .text()
                .await
                .expect("followed body")
        });
        assert_eq!(followed, "ok");
    }

    #[cfg(feature = "ollama")]
    #[test]
    fn ollama_generate_client_keeps_only_loopback_urls() {
        let remote = "https://api.openai.com";
        assert!(OllamaClient::new(remote, "llama").is_err());
        let opted = OllamaClient::allowing_remote(remote, "llama").expect("client");
        assert_eq!(opted.url(), remote);
        let userinfo = "http://127.0.0.1:11434@evil.example/api";
        let err = match OllamaClient::allowing_remote(userinfo, "llama") {
            Ok(_) => panic!("userinfo URL was stored"),
            Err(err) => err.to_string(),
        };
        assert!(err.contains("rejected"), "{err}");
        assert!(!err.contains("evil.example"), "{err}");
    }
    use astraweave_core::{
        CompanionState, Constraints, EnemyState, IVec2, PlayerState, ToolSpec, WorldSnapshot,
    };

    fn create_test_registry() -> ToolRegistry {
        ToolRegistry {
            tools: vec![
                ToolSpec {
                    name: "MoveTo".into(),
                    args: [("x", "i32"), ("y", "i32")]
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                },
                ToolSpec {
                    name: "Throw".into(),
                    args: [("item", "enum[smoke,grenade]"), ("x", "i32"), ("y", "i32")]
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                },
                ToolSpec {
                    name: "ThrowSmoke".into(),
                    args: [("x", "i32"), ("y", "i32")]
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                },
                ToolSpec {
                    name: "Attack".into(),
                    args: [("target_id", "u32")]
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                },
                ToolSpec {
                    name: "CoverFire".into(),
                    args: [("target_id", "u32"), ("duration", "f32")]
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                },
                ToolSpec {
                    name: "Revive".into(),
                    args: [("ally_id", "u32")]
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.into()))
                        .collect(),
                },
            ],
            constraints: Constraints {
                enforce_cooldowns: true,
                enforce_los: true,
                enforce_stamina: true,
            },
        }
    }

    fn create_test_world_snapshot() -> WorldSnapshot {
        WorldSnapshot {
            t: 1.0,
            player: PlayerState {
                hp: 100,
                pos: IVec2 { x: 2, y: 2 },
                stance: "stand".into(),
                orders: vec![],
            },
            me: CompanionState {
                ammo: 30,
                cooldowns: Default::default(),
                morale: 0.9,

                pos: IVec2 { x: 3, y: 2 },
            },
            enemies: vec![EnemyState {
                id: 99,
                pos: IVec2 { x: 12, y: 2 },
                hp: 60,
                cover: "low".into(),
                last_seen: 1.0,
            }],
            pois: vec![],
            obstacles: vec![],
            objective: Some("extract".into()),

            hazards: [astraweave_core::WeaveHazard::EMPTY; astraweave_core::MAX_WEAVE_HAZARDS],
            hazard_count: 0,
        }
    }

    // Mock client that returns custom JSON
    struct TestLlmClient {
        response: String,
    }

    #[async_trait::async_trait]
    impl LlmClient for TestLlmClient {
        async fn complete(&self, _prompt: &str) -> Result<String> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn test_build_prompt() {
        let snap = create_test_world_snapshot();
        let reg = create_test_registry();

        let prompt = build_prompt(&snap, &reg);

        // Check that prompt contains expected elements
        assert!(prompt.contains("AI game companion planner"));
        // Tools are in PascalCase in the prompt
        assert!(prompt.contains("MoveTo") || prompt.contains("move_to"));
        assert!(prompt.contains("Throw") || prompt.contains("throw"));
        assert!(prompt.contains("CoverFire") || prompt.contains("cover_fire"));
        assert!(prompt.contains("Return ONLY JSON"));
        assert!(prompt.contains("\"t\": 1.0"));
    }

    #[test]
    fn test_parse_llm_plan_valid() {
        let reg = create_test_registry();
        let json = r#"{
            "plan_id": "test-plan",
            "steps": [
                {"act": "MoveTo", "x": 5, "y": 5},
                {"act": "Throw", "item": "smoke", "x": 7, "y": 3}
            ]
        }"#;

        let result = parse_llm_plan(json, &reg);
        assert!(result.is_ok());

        let plan = result.unwrap();
        assert_eq!(plan.plan_id, "test-plan");
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn test_parse_llm_plan_invalid_json() {
        let reg = create_test_registry();
        let invalid_json = "not json";

        let result = parse_llm_plan(invalid_json, &reg);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_llm_plan_disallowed_tool() {
        let mut reg = create_test_registry();
        // Remove the MoveTo tool to test disallowed tool detection
        reg.tools.retain(|t| t.name != "MoveTo");

        let json = r#"{
            "plan_id": "test-plan",
            "steps": [
                {"act": "MoveTo", "x": 5, "y": 5}
            ]
        }"#;

        let result = parse_llm_plan(json, &reg);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("disallowed tool MoveTo"));
    }

    #[tokio::test]
    async fn test_mock_llm_client() {
        let client = MockLlm;
        let result = client.complete("test prompt").await;

        assert!(result.is_ok());
        let response = result.unwrap();

        // Should be valid JSON
        assert!(serde_json::from_str::<serde_json::Value>(&response).is_ok());
        assert!(response.contains("llm-mock"));
    }

    #[tokio::test]
    async fn test_plan_from_llm_success() {
        let mut snap = create_test_world_snapshot();
        snap.t = 100.1; // Unique for cache
        let reg = create_test_registry();
        let client = MockLlm;

        let result = plan_from_llm(&client, &snap, &reg).await;
        match result {
            PlanSource::Llm(plan) => {
                assert_eq!(plan.plan_id, "llm-mock");
                assert!(!plan.steps.is_empty());
            }
            PlanSource::Fallback { .. } => panic!("Expected LLM plan"),
        }
    }

    #[tokio::test]
    async fn test_plan_from_llm_invalid_response() {
        #[cfg(feature = "llm_cache")]
        super::clear_global_cache();

        let mut snap = create_test_world_snapshot();
        snap.t = 100.2; // Unique for cache
        let reg = create_test_registry();
        let client = TestLlmClient {
            response: "invalid json".to_string(),
        };

        let result = plan_from_llm(&client, &snap, &reg).await;
        // Should fallback to heuristic or emergency plan (Phase 7 multi-tier fallback)
        match result {
            PlanSource::Llm(_) => panic!("Expected fallback"),
            PlanSource::Fallback { plan, .. } => {
                // Phase 7: Plans are generated with UUID-based IDs
                assert!(
                    plan.plan_id.starts_with("heuristic-")
                        || plan.plan_id.starts_with("emergency-"),
                    "Expected heuristic or emergency plan, got: {}",
                    plan.plan_id
                );
            }
        }
    }

    #[tokio::test]
    async fn test_plan_from_llm_disallowed_tool() {
        #[cfg(feature = "llm_cache")]
        super::clear_global_cache();

        let mut snap = create_test_world_snapshot();
        snap.t = 100.3; // Unique for cache
        let mut reg = create_test_registry();
        // Remove all tools
        reg.tools.clear();

        let client = MockLlm;

        let result = plan_from_llm(&client, &snap, &reg).await;
        // Should fallback to heuristic or emergency plan (which uses no tools when registry is empty)
        match result {
            PlanSource::Llm(_) => panic!("Expected fallback"),
            PlanSource::Fallback { plan, .. } => {
                // Phase 7: Plans are generated with UUID-based IDs
                assert!(
                    plan.plan_id.starts_with("heuristic-")
                        || plan.plan_id.starts_with("emergency-"),
                    "Expected heuristic or emergency plan, got: {}",
                    plan.plan_id
                );
                // Note: Emergency plan always returns Scan + Wait even with empty registry
                // Heuristic with empty registry returns empty steps
            }
        }
    }

    #[test]
    fn test_parse_llm_plan_empty_steps() {
        let reg = create_test_registry();
        let json = r#"{
            "plan_id": "empty-plan",
            "steps": []
        }"#;

        let result = parse_llm_plan(json, &reg);
        assert!(result.is_ok());

        let plan = result.unwrap();
        assert_eq!(plan.plan_id, "empty-plan");
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_parse_llm_plan_all_action_types() {
        let mut reg = create_test_registry();
        // Add revive tool
        reg.tools.push(ToolSpec {
            name: "revive".into(),
            args: [("ally_id", "u32")]
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        });

        let json = r#"{
            "plan_id": "all-actions",
            "steps": [
                {"act": "MoveTo", "x": 5, "y": 5},
                {"act": "Throw", "item": "grenade", "x": 7, "y": 3},
                {"act": "CoverFire", "target_id": 42, "duration": 3.5},
                {"act": "Revive", "ally_id": 123}
            ]
        }"#;

        let result = parse_llm_plan(json, &reg);
        assert!(result.is_ok());

        let plan = result.unwrap();
        assert_eq!(plan.steps.len(), 4);
    }

    /// SEC-POC SEC-LLM-1. Constructors must refuse a URL they would rewrite.
    /// A bearer token is not stored for that URL.
    #[cfg(feature = "ollama")]
    #[test]
    fn poc_local_http_client_rejects_non_loopback_url() {
        let key = "sk-test";
        let remote = "https://api.openai.com/v1";
        let userinfo = "http://user:secret@evil.example/v1";
        let cases = [
            LocalHttpClient::new(remote.to_string(), "gpt-4".to_string()),
            LocalHttpClient::with_api_key(remote.to_string(), "gpt-4".to_string(), key.to_string()),
            LocalHttpClient::allowing_remote_with_api_key(
                userinfo.to_string(),
                "gpt-4".to_string(),
                key.to_string(),
            ),
        ];
        for result in cases {
            let err = match result {
                Ok(_) => panic!("rewritten URL was stored"),
                Err(err) => err.to_string(),
            };
            assert!(err.contains("rejected"), "{err}");
            assert!(!err.contains(key), "{err}");
            assert!(!err.contains("secret"), "{err}");
        }
        let kept = LocalHttpClient::with_api_key(
            "http://127.0.0.1:11434".to_string(),
            "gpt-4".to_string(),
            key.to_string(),
        )
        .expect("loopback");
        assert_eq!(kept.api_key.as_deref(), Some(key));
        let remote_key = LocalHttpClient::allowing_remote_with_api_key(
            remote.to_string(),
            "gpt-4".to_string(),
            key.to_string(),
        )
        .expect("remote opt-in");
        assert_eq!(remote_key.url(), remote);
        assert_eq!(remote_key.api_key.as_deref(), Some(key));
    }

    #[cfg(feature = "ollama")]
    #[test]
    fn test_ollama_client_creation() {
        let client = OllamaClient::new("http://localhost:11434", "llama2").expect("client");
        assert_eq!(client.url(), "http://localhost:11434");
        assert_eq!(client.model, "llama2");
    }

    #[cfg(feature = "ollama")]
    #[test]
    fn test_local_http_client_creation() {
        let client = LocalHttpClient::new(
            "http://localhost:5000".to_string(),
            "test-model".to_string(),
        )
        .expect("client");
        assert_eq!(client.url(), "http://localhost:5000");
        assert_eq!(client.model, "test-model");
        assert!(client.api_key.is_none());

        let client_with_key = LocalHttpClient::with_api_key(
            "http://localhost:5000".to_string(),
            "test-model".to_string(),
            "test-key".to_string(),
        )
        .expect("client");
        assert_eq!(client_with_key.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_prompt_includes_constraints() {
        let snap = create_test_world_snapshot();
        let reg = create_test_registry();

        let prompt = build_prompt(&snap, &reg);

        // Check that prompt mentions validation
        assert!(prompt.contains("engine will validate"));
        assert!(prompt.contains("Do not exceed cooldown or LOS checks"));
    }

    #[test]
    fn test_parse_llm_plan_malformed_step() {
        let reg = create_test_registry();
        let json = r#"{
            "plan_id": "malformed",
            "steps": [
                {"act": "MoveTo", "x": "not_a_number", "y": 5}
            ]
        }"#;

        let result = parse_llm_plan(json, &reg);
        // Should fail to parse due to type mismatch
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_llm_plan_missing_plan_id() {
        let reg = create_test_registry();
        let json = r#"{
            "steps": [
                {"act": "MoveTo", "x": 5, "y": 5}
            ]
        }"#;

        let result = parse_llm_plan(json, &reg);
        // Should fail due to missing plan_id
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_llm_plan_from_fenced_json() {
        let reg = create_test_registry();
        let text = r#"```json
        {
          "plan_id": "fenced",
          "steps": [ {"act":"MoveTo","x":1,"y":2} ]
        }
        ```"#;
        let res = parse_llm_plan(text, &reg).unwrap();
        assert_eq!(res.plan_id, "fenced");
        assert_eq!(res.steps.len(), 1);
    }

    #[test]
    fn test_parse_llm_plan_with_nonstandard_planid_key() {
        let reg = create_test_registry();
        // Simulate the model returning plan_n° key
        let text = r#"{
          "plan_n°": "7",
          "steps": [ {"act":"MoveTo","x":3,"y":4} ]
        }"#;
        let res = parse_llm_plan(text, &reg).unwrap();
        assert_eq!(res.plan_id, "7");
        assert_eq!(res.steps.len(), 1);
    }

    #[test]
    fn test_parse_llm_plan_from_envelope_message_content() {
        let reg = create_test_registry();
        // Simulate Ollama envelope with nested message.content carrying JSON
        let text = r#"{
          "message": { "role": "assistant", "content": "{\n  \"plan_id\": \"env\",\n  \"steps\": [ {\"act\":\"MoveTo\",\"x\":5,\"y\":6 } ]\n}" }
        }"#;
        let res = parse_llm_plan(text, &reg).unwrap();
        assert_eq!(res.plan_id, "env");
        assert_eq!(res.steps.len(), 1);
    }

    #[test]
    fn test_parse_llm_plan_pick_last_json_object() {
        let reg = create_test_registry();
        // Test picking the last complete JSON object
        let text = r#"{"plan_id": "first", "steps": []}
{"plan_id": "final", "steps": [ {"act":"MoveTo","x":1,"y":1} ] }"#;
        let res = parse_llm_plan(text, &reg).unwrap();
        assert_eq!(res.plan_id, "final");
        assert_eq!(res.steps.len(), 1);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Additional Coverage Tests
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_plan_source_llm_variant() {
        let plan = PlanIntent {
            plan_id: "test-llm".to_string(),
            steps: vec![ActionStep::Reload],
        };
        let source = PlanSource::Llm(plan.clone());

        if let PlanSource::Llm(p) = &source {
            assert_eq!(p.plan_id, "test-llm");
            assert_eq!(p.steps.len(), 1);
        } else {
            panic!("Expected LLM variant");
        }
    }

    #[test]
    fn test_plan_source_fallback_variant() {
        let plan = PlanIntent {
            plan_id: "test-fallback".to_string(),
            steps: vec![ActionStep::Scan { radius: 10.0 }],
        };
        let source = PlanSource::Fallback {
            plan: plan.clone(),
            reason: "LLM unavailable".to_string(),
        };

        if let PlanSource::Fallback { plan: p, reason } = &source {
            assert_eq!(p.plan_id, "test-fallback");
            assert_eq!(reason, "LLM unavailable");
        } else {
            panic!("Expected Fallback variant");
        }
    }

    #[test]
    fn test_plan_source_clone() {
        let plan = PlanIntent {
            plan_id: "clone-test".to_string(),
            steps: vec![],
        };
        let source = PlanSource::Llm(plan);
        let cloned = source.clone();

        if let PlanSource::Llm(p) = cloned {
            assert_eq!(p.plan_id, "clone-test");
        } else {
            panic!("Clone should preserve variant");
        }
    }

    #[test]
    fn test_plan_source_debug() {
        let plan = PlanIntent {
            plan_id: "debug-test".to_string(),
            steps: vec![],
        };
        let source = PlanSource::Llm(plan);
        let debug = format!("{:?}", source);
        assert!(debug.contains("Llm"));
        assert!(debug.contains("debug-test"));

        let fallback = PlanSource::Fallback {
            plan: PlanIntent {
                plan_id: "fb".to_string(),
                steps: vec![],
            },
            reason: "test".to_string(),
        };
        let debug = format!("{:?}", fallback);
        assert!(debug.contains("Fallback"));
    }

    #[tokio::test]
    async fn test_mock_llm_returns_valid_steps() {
        let client = MockLlm;
        let response = client.complete("test").await.unwrap();

        // Parse as JSON to verify structure
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert!(json.get("plan_id").is_some());
        assert!(json.get("steps").is_some());

        let steps = json.get("steps").unwrap().as_array().unwrap();
        assert!(!steps.is_empty());
    }

    #[tokio::test]
    async fn test_always_err_mock() {
        let client = AlwaysErrMock;
        let result = client.complete("test").await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("AlwaysErrMock"));
    }

    #[tokio::test]
    async fn test_llm_client_streaming_default() {
        let client = MockLlm;
        use futures_util::StreamExt;

        // MockLlm uses default streaming implementation
        let mut stream = client.complete_streaming("test").await.unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk.unwrap());
        }

        // Default implementation returns single chunk
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].contains("llm-mock"));
    }

    #[test]
    fn test_create_test_registry_content() {
        let reg = create_test_registry();

        // Verify registry has expected tools
        assert!(reg.tools.iter().any(|t| t.name == "MoveTo"));
        assert!(reg.tools.iter().any(|t| t.name == "Attack"));
        assert!(reg.tools.iter().any(|t| t.name == "CoverFire"));

        // Verify constraints
        assert!(reg.constraints.enforce_cooldowns);
        assert!(reg.constraints.enforce_los);
    }

    #[test]
    fn test_create_test_world_snapshot_content() {
        let snap = create_test_world_snapshot();

        assert_eq!(snap.t, 1.0);
        assert_eq!(snap.player.hp, 100);
        assert_eq!(snap.me.ammo, 30);
        assert_eq!(snap.enemies.len(), 1);
        assert_eq!(snap.enemies[0].id, 99);
        assert_eq!(snap.objective, Some("extract".into()));
    }

    #[tokio::test]
    async fn test_test_llm_client_returns_configured_response() {
        let client = TestLlmClient {
            response: r#"{"plan_id": "configured", "steps": []}"#.to_string(),
        };

        let result = client.complete("ignored prompt").await.unwrap();
        assert!(result.contains("configured"));
    }

    #[test]
    fn test_parse_llm_plan_with_extra_fields() {
        let reg = create_test_registry();
        let json = r#"{
            "plan_id": "extra-fields",
            "steps": [{"act": "MoveTo", "x": 1, "y": 2}],
            "extra_field": "ignored",
            "another": 42
        }"#;

        let result = parse_llm_plan(json, &reg);
        // Should still parse successfully, ignoring extra fields
        assert!(result.is_ok());
        assert_eq!(result.unwrap().plan_id, "extra-fields");
    }

    #[tokio::test]
    async fn test_plan_from_llm_with_valid_tool() {
        let mut snap = create_test_world_snapshot();
        snap.t = 100.4; // Unique for cache
        let reg = create_test_registry();
        let client = MockLlm;

        let result = plan_from_llm(&client, &snap, &reg).await;

        // MockLlm returns valid plan with ThrowSmoke, MoveTo, Attack
        if let PlanSource::Llm(plan) = result {
            assert!(!plan.steps.is_empty());
            // Verify at least one step exists (already checked with is_empty)
        } else {
            panic!("Expected LLM plan from MockLlm");
        }
    }

    #[test]
    fn test_parse_llm_plan_with_case_variations() {
        let reg = create_test_registry();

        // Test with different JSON formatting
        let compact = r#"{"plan_id":"compact","steps":[{"act":"MoveTo","x":1,"y":1}]}"#;
        assert!(parse_llm_plan(compact, &reg).is_ok());

        // Test with extra whitespace
        let spaced = r#"  {  "plan_id" : "spaced" , "steps" : [ { "act" : "MoveTo" , "x" : 1 , "y" : 1 } ] }  "#;
        assert!(parse_llm_plan(spaced, &reg).is_ok());
    }

    #[cfg(feature = "llm_cache")]
    #[test]
    fn test_clear_global_cache_does_not_panic() {
        // Just ensure it doesn't panic
        super::clear_global_cache();
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Coverage Tests for Helper Functions
    // ═══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_strip_code_fences_with_fence() {
        let text = r#"```json
{"plan_id": "test", "steps": []}
```"#;
        let result = strip_code_fences(text);
        assert!(result.contains("plan_id"));
        assert!(!result.contains("```"));
    }

    #[test]
    fn test_strip_code_fences_without_fence() {
        let text = r#"{"plan_id": "test", "steps": []}"#;
        let result = strip_code_fences(text);
        assert_eq!(result, text);
    }

    #[test]
    fn test_strip_code_fences_incomplete_fence() {
        // Only opening fence, no closing
        let text = r#"```json
{"plan_id": "test"}"#;
        let result = strip_code_fences(text);
        // Should return original since no closing fence
        assert_eq!(result, text);
    }

    #[test]
    fn test_extract_json_object_simple() {
        let text = r#"Here is the plan: {"plan_id": "test", "steps": []}"#;
        let result = extract_json_object(text);
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("plan_id"));
    }

    #[test]
    fn test_extract_json_object_nested() {
        let text = r#"{"outer": {"inner": "value"}, "array": [1, 2, 3]}"#;
        let result = extract_json_object(text);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), text);
    }

    #[test]
    fn test_extract_json_object_no_json() {
        let text = "This is plain text without JSON";
        let result = extract_json_object(text);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_json_object_with_string_braces() {
        // Braces inside strings should not confuse parser
        let text = r#"{"message": "Use { braces } in text", "value": 1}"#;
        let result = extract_json_object(text);
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.contains("braces"));
    }

    #[test]
    fn test_extract_last_json_object_single() {
        let text = r#"{"plan_id": "only", "steps": []}"#;
        let result = extract_last_json_object(text);
        assert!(result.is_some());
        assert!(result.unwrap().contains("only"));
    }

    #[test]
    fn test_extract_last_json_object_multiple() {
        let text = r#"{"plan_id": "first", "steps": []}
Some text
{"plan_id": "second", "steps": [{"act": "Scan"}]}"#;
        let result = extract_last_json_object(text);
        assert!(result.is_some());
        assert!(result.unwrap().contains("second"));
    }

    #[test]
    fn test_extract_last_json_object_none() {
        let text = "No JSON here";
        let result = extract_last_json_object(text);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_json_from_fenced_json_tag() {
        let text = r#"Here's the plan:
```json
{"plan_id": "fenced", "steps": []}
```
Done!"#;
        let result = extract_json_from_fenced(text);
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.contains("fenced"));
    }

    #[test]
    fn test_extract_json_from_fenced_no_tag() {
        let text = r#"Here:
```
{"plan_id": "no-tag", "steps": []}
```"#;
        let result = extract_json_from_fenced(text);
        assert!(result.is_some());
        assert!(result.unwrap().contains("no-tag"));
    }

    #[test]
    fn test_extract_json_from_fenced_none() {
        let text = r#"No code fences here"#;
        let result = extract_json_from_fenced(text);
        assert!(result.is_none());
    }

    #[test]
    fn test_estimate_tokens() {
        // 4 chars per token estimate
        assert_eq!(estimate_tokens("test"), 1);
        assert_eq!(estimate_tokens("12345678"), 2);
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a".repeat(100).as_str()), 25);
    }

    #[test]
    fn test_fallback_heuristic_plan_with_objective() {
        let mut snap = create_test_world_snapshot();
        snap.objective = Some("extract".to_string());
        snap.me.pos = IVec2 { x: 100, y: 100 }; // Far from player

        let mut reg = create_test_registry();
        reg.tools.push(ToolSpec {
            name: "move_to".into(),
            args: std::collections::BTreeMap::new(),
        });
        reg.tools.push(ToolSpec {
            name: "cover_fire".into(),
            args: std::collections::BTreeMap::new(),
        });

        let plan = fallback_heuristic_plan(&snap, &reg);
        assert_eq!(plan.plan_id, "heuristic-fallback");
        // Should have steps since we have tools and conditions
    }

    #[test]
    fn test_fallback_heuristic_plan_empty_tools() {
        let snap = create_test_world_snapshot();
        let reg = ToolRegistry {
            tools: vec![],
            constraints: Constraints {
                enforce_cooldowns: false,
                enforce_los: false,
                enforce_stamina: false,
            },
        };

        let plan = fallback_heuristic_plan(&snap, &reg);
        assert_eq!(plan.plan_id, "heuristic-fallback");
        // No tools means empty steps
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_fallback_heuristic_plan_with_enemies() {
        let snap = create_test_world_snapshot();
        let mut reg = ToolRegistry {
            tools: vec![],
            constraints: Constraints {
                enforce_cooldowns: false,
                enforce_los: false,
                enforce_stamina: false,
            },
        };
        reg.tools.push(ToolSpec {
            name: "cover_fire".into(),
            args: std::collections::BTreeMap::new(),
        });

        let plan = fallback_heuristic_plan(&snap, &reg);
        // Should have CoverFire step since we have enemies and the tool
        assert!(!plan.steps.is_empty());
        assert!(plan
            .steps
            .iter()
            .any(|s| matches!(s, ActionStep::CoverFire { .. })));
    }

    #[test]
    fn test_sanitize_plan_removes_out_of_bounds() {
        let snap = create_test_world_snapshot();
        let reg = create_test_registry();

        let mut plan = PlanIntent {
            plan_id: "sanitize-test".to_string(),
            steps: vec![
                ActionStep::MoveTo {
                    x: 5,
                    y: 5,
                    speed: None,
                },
                ActionStep::MoveTo {
                    x: 999,
                    y: 999,
                    speed: None,
                }, // Out of bounds
            ],
        };

        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 1); // Out of bounds step removed
    }

    #[test]
    fn sanitize_plan_drops_i32_min_and_max() {
        use astraweave_core::default_tool_registry;

        let reg = default_tool_registry();
        let snap = create_test_world_snapshot();
        let extremes = [i32::MIN, i32::MAX];

        for extreme in extremes {
            for (x, y) in [(extreme, 0), (0, extreme)] {
                let move_json = format!(
                    r#"{{"plan_id":"bounds","steps":[{{"act":"MoveTo","x":{x},"y":{y}}},{{"act":"MoveTo","x":0,"y":0}}]}}"#
                );
                let mut plan =
                    parse_llm_plan(&move_json, &reg).expect("default registry accepts MoveTo");
                assert_eq!(plan.steps.len(), 2);
                sanitize_plan(&mut plan, &snap, &reg).unwrap();
                assert_eq!(
                    plan.steps,
                    vec![ActionStep::MoveTo {
                        x: 0,
                        y: 0,
                        speed: None,
                    }],
                    "MoveTo ({x},{y}) exceeds MAX_COORD_BOUND"
                );

                let throw_json = format!(
                    r#"{{"plan_id":"bounds","steps":[{{"act":"Throw","item":"smoke","x":{x},"y":{y}}},{{"act":"Throw","item":"smoke","x":0,"y":0}}]}}"#
                );
                let mut plan =
                    parse_llm_plan(&throw_json, &reg).expect("default registry accepts Throw");
                assert_eq!(plan.steps.len(), 2);
                sanitize_plan(&mut plan, &snap, &reg).unwrap();
                assert_eq!(
                    plan.steps,
                    vec![ActionStep::Throw {
                        item: "smoke".to_string(),
                        x: 0,
                        y: 0,
                    }],
                    "Throw ({x},{y}) exceeds MAX_COORD_BOUND"
                );
            }
        }
    }

    #[test]
    fn test_sanitize_plan_validates_throw_items() {
        let snap = create_test_world_snapshot();
        let reg = create_test_registry();

        let mut plan = PlanIntent {
            plan_id: "throw-test".to_string(),
            steps: vec![
                ActionStep::Throw {
                    item: "smoke".to_string(),
                    x: 5,
                    y: 5,
                },
                ActionStep::Throw {
                    item: "invalid".to_string(),
                    x: 5,
                    y: 5,
                }, // Invalid item
            ],
        };

        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 1); // Invalid item removed
    }

    #[test]
    fn test_sanitize_plan_validates_cover_fire() {
        let snap = create_test_world_snapshot();
        let reg = create_test_registry();

        let mut plan = PlanIntent {
            plan_id: "cover-test".to_string(),
            steps: vec![
                ActionStep::CoverFire {
                    target_id: 99,
                    duration: 3.0,
                }, // Valid target
                ActionStep::CoverFire {
                    target_id: 999,
                    duration: 3.0,
                }, // Invalid target
                ActionStep::CoverFire {
                    target_id: 99,
                    duration: 0.0,
                }, // Invalid duration
                ActionStep::CoverFire {
                    target_id: 99,
                    duration: 100.0,
                }, // Duration too long
            ],
        };

        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 1); // Only valid CoverFire remains
    }

    #[test]
    fn test_sanitize_plan_allows_other_steps() {
        let snap = create_test_world_snapshot();
        let mut reg = create_test_registry();

        let mut plan = PlanIntent {
            plan_id: "other-test".to_string(),
            steps: vec![
                ActionStep::Reload,
                ActionStep::Scan { radius: 10.0 },
                ActionStep::Wait { duration: 1.0 },
            ],
        };

        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert!(
            plan.steps.is_empty(),
            "Reload, Scan, and Wait are not in this registry"
        );

        for name in ["Reload", "Scan", "Wait"] {
            reg.tools.push(ToolSpec {
                name: name.into(),
                args: std::collections::BTreeMap::new(),
            });
        }
        plan.steps = vec![
            ActionStep::Reload,
            ActionStep::Scan { radius: 10.0 },
            ActionStep::Wait { duration: 1.0 },
        ];
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 3);
    }

    #[test]
    fn default_registry_core_steps_validate() {
        use astraweave_core::default_tool_registry;

        let reg = default_tool_registry();
        let json = r#"{"plan_id":"core","steps":[
            {"act":"MoveTo","x":1,"y":2},
            {"act":"Throw","item":"smoke","x":3,"y":4},
            {"act":"CoverFire","target_id":99,"duration":2.0},
            {"act":"Revive","ally_id":1}
        ]}"#;

        let mut plan = parse_llm_plan(json, &reg).expect("core steps against the default registry");
        assert_eq!(plan.steps.len(), 4);

        let parsed = plan_parser::parse_llm_response(json, &reg).expect("runtime parser");
        assert_eq!(parsed.plan.steps.len(), 4);

        let schema = plan_format::plan_format_schema(&reg);
        let acts: Vec<&str> = schema["properties"]["steps"]["items"]["oneOf"]
            .as_array()
            .expect("oneOf")
            .iter()
            .filter_map(|branch| branch["properties"]["act"]["enum"][0].as_str())
            .collect();
        assert_eq!(acts, ["MoveTo", "Throw", "CoverFire", "Revive"]);

        plan.steps.push(ActionStep::Attack { target_id: 99 });
        let snap = create_test_world_snapshot();
        sanitize_plan(&mut plan, &snap, &reg).unwrap();
        assert_eq!(plan.steps.len(), 4, "Attack is dropped; core steps stay");

        let rejected = parse_llm_plan(
            r#"{"plan_id":"no","steps":[{"act":"Attack","target_id":99}]}"#,
            &reg,
        );
        assert!(rejected.is_err());
    }

    #[test]
    fn test_validate_plan_disallowed_move_to() {
        let mut reg = create_test_registry();
        reg.tools.retain(|t| t.name != "MoveTo");

        let plan = PlanIntent {
            plan_id: "validate-test".to_string(),
            steps: vec![ActionStep::MoveTo {
                x: 5,
                y: 5,
                speed: None,
            }],
        };

        let result = validate_plan(&plan, &reg);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("disallowed tool MoveTo"));
    }

    #[test]
    fn test_validate_plan_disallowed_throw() {
        let mut reg = create_test_registry();
        reg.tools.retain(|t| t.name != "Throw");

        let plan = PlanIntent {
            plan_id: "validate-throw".to_string(),
            steps: vec![ActionStep::Throw {
                item: "smoke".to_string(),
                x: 1,
                y: 1,
            }],
        };

        let result = validate_plan(&plan, &reg);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("disallowed tool Throw"));
    }

    #[test]
    fn test_validate_plan_disallowed_cover_fire() {
        let mut reg = create_test_registry();
        reg.tools.retain(|t| t.name != "CoverFire");

        let plan = PlanIntent {
            plan_id: "validate-cover".to_string(),
            steps: vec![ActionStep::CoverFire {
                target_id: 1,
                duration: 2.0,
            }],
        };

        let result = validate_plan(&plan, &reg);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("disallowed tool CoverFire"));
    }

    #[test]
    fn test_validate_plan_disallowed_revive() {
        let mut reg = create_test_registry();
        reg.tools.retain(|t| t.name != "Revive");

        let plan = PlanIntent {
            plan_id: "validate-revive".to_string(),
            steps: vec![ActionStep::Revive { ally_id: 1 }],
        };

        let result = validate_plan(&plan, &reg);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("disallowed tool Revive"));
    }

    #[test]
    fn test_validate_plan_rejects_unregistered_generic_steps() {
        let reg = create_test_registry();

        let plan = PlanIntent {
            plan_id: "generic-test".to_string(),
            steps: vec![
                ActionStep::Reload,
                ActionStep::Scan { radius: 15.0 },
                ActionStep::Wait { duration: 2.0 },
            ],
        };

        let result = validate_plan(&plan, &reg);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("disallowed tool Reload"));

        let mut allowed = create_test_registry();
        for name in ["Reload", "Scan", "Wait"] {
            allowed.tools.push(ToolSpec {
                name: name.into(),
                args: std::collections::BTreeMap::new(),
            });
        }
        assert!(validate_plan(&plan, &allowed).is_ok());
    }

    #[test]
    fn test_max_coord_bound_constant() {
        assert_eq!(MAX_COORD_BOUND, 100);
    }

    #[test]
    fn test_build_prompt_includes_enemies() {
        let snap = create_test_world_snapshot();
        let reg = create_test_registry();

        let prompt = build_prompt(&snap, &reg);
        // Should include enemy information
        assert!(prompt.contains("99") || prompt.contains("enemy"));
    }

    #[test]
    fn test_build_prompt_includes_world_state() {
        let snap = create_test_world_snapshot();
        let reg = create_test_registry();

        let prompt = build_prompt(&snap, &reg);
        // Should include position and state info
        assert!(prompt.contains("pos"));
    }

    #[cfg(not(feature = "llm_cache"))]
    #[test]
    fn test_get_cache_stats_without_feature() {
        let stats = get_cache_stats();
        // Returns () when cache feature disabled
        assert_eq!(stats, ());
    }
}
