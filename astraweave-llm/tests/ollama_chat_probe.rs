#![cfg(feature = "ollama")]

//! Closed-port checks for `OllamaChatClient`.
//!
//! `CARGO_BIN_EXE_ollama_chat_probe` is an absolute path on the machine that
//! compiled the test. Windows partition jobs only have the nextest archive, so
//! profile `ci-pr-archive` skips this target and the archive build job runs it.

/// `CARGO_BIN_EXE_*` is absolute and already ends in `.exe` on Windows.
/// If a toolchain omits that suffix, try it before giving up.
fn probe_exe(compiled: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(compiled);
    if path.is_file() {
        return path;
    }
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        return path;
    }
    let mut with_suffix = path.clone().into_os_string();
    with_suffix.push(".exe");
    let exe = std::path::PathBuf::from(with_suffix);
    if exe.is_file() {
        exe
    } else {
        path
    }
}

fn ollama_chat_probe(mode: &str) -> std::process::Output {
    // The default client is low-latency, so `complete` skips the non-stream
    // attempts and reads `OLLAMA_TIMEOUT_SECS`. `Command::env` does not call `set_var`.
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    std::process::Command::new(&exe)
        .arg(mode)
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "1")
        .output()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()))
}

#[test]
fn test_ollama_client_connection_error() {
    let out = ollama_chat_probe("complete");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "closed port should fail: {stdout} {stderr}"
    );
    let err = stdout.to_lowercase();
    assert!(
        err.contains("error") || err.contains("fail") || err.contains("timeout"),
        "{stdout} {stderr}"
    );
}

#[test]
fn test_ollama_batch_execution() {
    let out = ollama_chat_probe("batch");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "closed port should fail: {stdout} {stderr}"
    );
}

/// Cap on each blocking `read` while draining a request or the post-close socket.
///
/// This is not a pause inserted to win a race. A `read` that receives nothing
/// returns after 2s, which unblocks that call. Bytes that keep arriving inside
/// the cap keep the read loop running; there is no separate deadline for the
/// whole loop.
const MOCK_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Header and body reads share this buffer. Bytes past `Content-Length` that
/// arrived in the same read are dropped with it.
const READ_CHUNK: usize = 1024;

/// A mock thread is joined within this bound after shutdown. It covers one
/// request read already inside `MOCK_READ_TIMEOUT` and the half-close read
/// after it. It is not a client timeout.
const MOCK_JOIN_BOUND: std::time::Duration = std::time::Duration::from_secs(6);

fn read_once<R: std::io::Read + ?Sized>(reader: &mut R, buf: &mut [u8]) -> std::io::Result<usize> {
    loop {
        match reader.read(buf) {
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            other => return other,
        }
    }
}

fn header_block_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn field_value<'a>(line: &'a [u8], name: &[u8]) -> Option<&'a [u8]> {
    let colon = line.iter().position(|byte| *byte == b':')?;
    if !line[..colon].eq_ignore_ascii_case(name) {
        return None;
    }
    Some(line.get(colon + 1..)?.trim_ascii())
}

fn content_length(headers: &[u8]) -> Option<usize> {
    let mut start = 0;
    let mut steps = 0usize;
    while start < headers.len() {
        // A normal pass advances `start` every iteration. Cap the steps at the
        // byte length so a loop that stops advancing returns instead of spinning.
        steps += 1;
        if steps > headers.len() {
            return None;
        }
        let rest = &headers[start..];
        let line_len = rest
            .windows(2)
            .position(|window| window == b"\r\n")
            .unwrap_or(rest.len());
        let line = &rest[..line_len];
        start += line_len;
        if headers
            .get(start..)
            .is_some_and(|rest| rest.starts_with(b"\r\n"))
        {
            start += 2;
        } else {
            start = headers.len();
        }
        let Some(value) = field_value(line, b"content-length") else {
            continue;
        };
        let text = std::str::from_utf8(value).ok()?.trim();
        return text.parse().ok();
    }
    None
}

fn discard<R: std::io::Read>(reader: &mut R, mut remaining: usize) {
    let mut tmp = [0u8; READ_CHUNK];
    while remaining > 0 {
        let want = remaining.min(tmp.len());
        match read_once(reader, &mut tmp[..want]) {
            Ok(0) | Err(_) => return,
            Ok(n) => remaining -= n,
        }
    }
}

/// Read the request headers and, when `Content-Length` is present, that many body bytes.
///
/// Closing while unread request bytes are still in the receive buffer makes
/// Windows abort the connection (`WSAECONNRESET`) instead of delivering the
/// response. Mocks call this before writing a status line. POST bodies from
/// this harness are sent with reqwest `.json()`, which sets `Content-Length`.
/// Health-check modes send GET and no body. A request with no `Content-Length`
/// returns here; this function does not keep reading to find a body.
fn read_http_request(sock: &mut std::net::TcpStream) {
    let _ = sock.set_read_timeout(Some(MOCK_READ_TIMEOUT));
    read_http_request_from(sock);
}

fn read_http_request_from<R: std::io::Read>(reader: &mut R) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; READ_CHUNK];
    let header_end = loop {
        if let Some(end) = header_block_end(&buf) {
            break end;
        }
        if buf.len() >= 8192 {
            return;
        }
        match read_once(reader, &mut tmp) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
    };
    let Some(len) = content_length(&buf[..header_end]) else {
        return;
    };
    let already = buf.len().saturating_sub(header_end);
    if already < len {
        discard(reader, len - already);
    }
}

/// Half-close a finished response, then read until the client closes or one read stays silent.
///
/// `shutdown(Write)` sends FIN after the bytes already written. The returned
/// thread takes bytes the client still sends, so the socket is not dropped
/// while those bytes are unread. Windows turns a drop with unread received
/// data into `WSAECONNRESET`. The read runs off the accept loop: a 404 has to
/// be able to accept `/api/generate` while the first client is still closing.
/// Each of those reads uses `MOCK_READ_TIMEOUT`. The caller joins the thread.
fn finish_response(mut sock: std::net::TcpStream) -> std::thread::JoinHandle<()> {
    let _ = sock.shutdown(std::net::Shutdown::Write);
    std::thread::spawn(move || {
        let _ = sock.set_read_timeout(Some(MOCK_READ_TIMEOUT));
        let mut tmp = [0u8; READ_CHUNK];
        loop {
            match read_once(&mut sock, &mut tmp) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    })
}

fn join_bounded(handle: std::thread::JoinHandle<()>) {
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome = handle.join();
        let _ = done_tx.send(outcome);
    });
    match done_rx.recv_timeout(MOCK_JOIN_BOUND) {
        Ok(Ok(())) => {}
        Ok(Err(payload)) => {
            if !std::thread::panicking() {
                std::panic::resume_unwind(payload);
            }
        }
        Err(_) => {
            if !std::thread::panicking() {
                panic!("mock server thread did not exit within {MOCK_JOIN_BOUND:?}");
            }
        }
    }
}

fn shutdown_requested(shutdown: &std::sync::mpsc::Receiver<()>) -> bool {
    match shutdown.try_recv() {
        Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => true,
        Err(std::sync::mpsc::TryRecvError::Empty) => false,
    }
}

/// A loopback mock. Drop signals the thread, unblocks `accept`, and joins it.
///
/// The join has a bound and runs when the test returns or panics, so a mock
/// cannot outlive the test that started it.
#[must_use = "dropping the server shuts the mock down"]
struct MockServer {
    addr: std::net::SocketAddr,
    shutdown: Option<std::sync::mpsc::Sender<()>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl MockServer {
    fn addr(&self) -> std::net::SocketAddr {
        self.addr
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        // `accept` blocks. Connecting to this listener unblocks it after the
        // shutdown signal, including when the test fails before the client connects.
        let _ = std::net::TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            join_bounded(handle);
        }
    }
}

fn start_server<F>(body: F) -> MockServer
where
    F: FnOnce(std::net::TcpListener, std::sync::mpsc::Receiver<()>) + Send + 'static,
{
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();
    let handle = std::thread::spawn(move || body(listener, shutdown_rx));
    MockServer {
        addr,
        shutdown: Some(shutdown_tx),
        handle: Some(handle),
    }
}

fn accept_client(
    listener: &std::net::TcpListener,
    shutdown: &std::sync::mpsc::Receiver<()>,
) -> Option<std::net::TcpStream> {
    if shutdown_requested(shutdown) {
        return None;
    }
    let Ok((sock, _)) = listener.accept() else {
        return None;
    };
    if shutdown_requested(shutdown) {
        return None;
    }
    Some(sock)
}

fn join_drains(drains: Vec<std::thread::JoinHandle<()>>) {
    for drain in drains {
        join_bounded(drain);
    }
}

/// Probe child. Drop kills and reaps it, including when the test panics.
/// A child that already exited is not signalled again.
struct ReapedChild {
    child: Option<std::process::Child>,
    exited: bool,
}

impl Drop for ReapedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if !self.exited {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

impl ReapedChild {
    fn spawn(cmd: &mut std::process::Command) -> Self {
        let child = cmd
            .spawn()
            .unwrap_or_else(|err| panic!("spawn {}: {err}", cmd.get_program().to_string_lossy()));
        Self {
            child: Some(child),
            exited: false,
        }
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        let status = self.child.as_mut().expect("probe child").try_wait()?;
        if status.is_some() {
            self.exited = true;
        }
        Ok(status)
    }

    fn wait_with_output(mut self) -> std::io::Result<std::process::Output> {
        self.exited = true;
        self.child.take().expect("probe child").wait_with_output()
    }

    fn kill_and_reap(&mut self) {
        if let Some(mut child) = self.child.take() {
            if !self.exited {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }

    fn stdout(&mut self) -> std::process::ChildStdout {
        self.child
            .as_mut()
            .expect("probe child")
            .stdout
            .take()
            .expect("stdout")
    }

    fn stderr(&mut self) -> std::process::ChildStderr {
        self.child
            .as_mut()
            .expect("probe child")
            .stderr
            .take()
            .expect("stderr")
    }
}

fn gap_elapsed(shutdown: &std::sync::mpsc::Receiver<()>) -> bool {
    match shutdown.recv_timeout(std::time::Duration::from_millis(50)) {
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => true,
        Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => false,
    }
}

/// `false` means shutdown won and the caller must not wait on it again.
fn wait_or_stop<T>(
    value: &std::sync::mpsc::Receiver<T>,
    shutdown: &std::sync::mpsc::Receiver<()>,
) -> Option<T> {
    loop {
        match value.try_recv() {
            Ok(item) => return Some(item),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => return None,
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                match shutdown.recv_timeout(std::time::Duration::from_millis(20)) {
                    Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return None,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                }
            }
        }
    }
}

fn wait_until_peek(
    sock: &mut std::net::TcpStream,
    shutdown: &std::sync::mpsc::Receiver<()>,
    mut ready: impl FnMut(&[u8]) -> bool,
) -> bool {
    let _ = sock.set_read_timeout(Some(std::time::Duration::from_millis(50)));
    let mut buf = [0u8; 8192];
    loop {
        if shutdown_requested(shutdown) {
            return false;
        }
        match sock.peek(&mut buf) {
            Ok(0) => return false,
            Ok(n) if ready(&buf[..n]) => return true,
            Ok(_) => match shutdown.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return false,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            },
            Err(err)
                if err.kind() == std::io::ErrorKind::Interrupted
                    || err.kind() == std::io::ErrorKind::TimedOut
                    || err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return false,
        }
    }
}

/// True only when a non-blocking read returns one or more bytes.
///
/// `Ok(0)` and any error, including `WouldBlock` and `TimedOut`, are not a
/// queued byte. A peer that has shut its write side with nothing buffered
/// reads as `Ok(0)`.
fn unread_byte(sock: &mut std::net::TcpStream) -> bool {
    sock.set_nonblocking(true).expect("nonblocking");
    let mut one = [0u8; 1];
    let pending = matches!(read_once(sock, &mut one), Ok(n) if n > 0);
    let _ = sock.set_nonblocking(false);
    pending
}

struct InterruptOnce {
    delivered: bool,
    rest: &'static [u8],
}

impl std::io::Read for InterruptOnce {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if !self.delivered {
            self.delivered = true;
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "interrupted",
            ));
        }
        if self.rest.is_empty() {
            return Ok(0);
        }
        let n = self.rest.len().min(buf.len());
        buf[..n].copy_from_slice(&self.rest[..n]);
        self.rest = &self.rest[n..];
        Ok(n)
    }
}

struct TimeoutAfterFirst {
    first: Option<Vec<u8>>,
    timeouts: u32,
}

impl std::io::Read for TimeoutAfterFirst {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(data) = self.first.take() {
            let n = data.len().min(buf.len());
            buf[..n].copy_from_slice(&data[..n]);
            if n < data.len() {
                self.first = Some(data[n..].to_vec());
            }
            return Ok(n);
        }
        self.timeouts += 1;
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "declared body stalled",
        ))
    }
}

#[test]
fn content_length_reads_the_decimal_field() {
    // A parser loop that never advances must fail this test, not hang the suite.
    let finished = |headers: &'static [u8]| {
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _ = tx.send(content_length(headers));
        });
        match rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok(value) => {
                worker.join().expect("content_length thread");
                value
            }
            Err(_) => panic!("content_length did not return within 1s"),
        }
    };
    let headers = b"POST /api/chat HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 15\r\n\r\n";
    assert_eq!(finished(headers), Some(15));
    let lower = b"POST /api/chat HTTP/1.1\r\ncontent-length: 0\r\n\r\n";
    assert_eq!(finished(lower), Some(0));
    let absent = b"POST /api/chat HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
    assert_eq!(finished(absent), None);
}

#[test]
fn read_once_retries_a_single_interrupted_read() {
    let mut reader = InterruptOnce {
        delivered: false,
        rest: b"abc",
    };
    let mut buf = [0u8; 8];
    let n = read_once(&mut reader, &mut buf).expect("read after interrupt");
    assert_eq!(&buf[..n], b"abc");
    assert!(reader.delivered);
}

#[test]
fn read_http_request_drains_the_post_body() {
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let server = start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        use std::io::Read;
        let mut buf = [0u8; 8];
        let n = sock.read(&mut buf).expect("read after body");
        let _ = done_tx.send(n);
        let _ = shutdown.recv();
    });
    let mut client = std::net::TcpStream::connect(server.addr()).expect("connect");
    let body = vec![b'x'; 3000];
    let header = format!(
        "POST /api/chat HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    use std::io::Write;
    client.write_all(header.as_bytes()).expect("headers");
    client.write_all(&body).expect("body");
    client.flush().expect("flush");
    client
        .shutdown(std::net::Shutdown::Write)
        .expect("shutdown");
    let n = done_rx
        .recv_timeout(MOCK_READ_TIMEOUT)
        .expect("server finished the drain");
    assert_eq!(n, 0, "POST body was still unread");
}

#[test]
fn read_http_request_without_content_length_returns_before_the_read_cap() {
    let elapsed = time_buffered_request(
        b"POST /api/chat HTTP/1.1\r\nHost: 127.0.0.1\r\nTransfer-Encoding: chunked\r\n\r\n",
        |buf| header_block_end(buf).is_some(),
    )
    .0;
    assert!(
        elapsed < MOCK_READ_TIMEOUT / 2,
        "missing Content-Length took {elapsed:?}; per-read cap is {MOCK_READ_TIMEOUT:?}"
    );
}

#[test]
fn read_http_request_returns_when_the_declared_body_stalls() {
    // The socket cap is `MOCK_READ_TIMEOUT`. This reader reports that timeout
    // directly, so the test does not wait on the clock and does not compare a
    // measured sleep with the cap.
    let headers =
        b"POST /api/chat HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 32\r\n\r\n".to_vec();
    let (tx, rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut reader = TimeoutAfterFirst {
            first: Some(headers),
            timeouts: 0,
        };
        read_http_request_from(&mut reader);
        let _ = tx.send(reader.timeouts);
    });
    match rx.recv_timeout(std::time::Duration::from_secs(1)) {
        Ok(timeouts) => {
            worker.join().expect("stalled read");
            assert_eq!(timeouts, 1, "stalled body was not one timed-out read");
        }
        Err(_) => panic!("read_http_request did not return within 1s"),
    }
}

#[test]
fn read_http_request_stops_at_the_header_cap() {
    let (elapsed, still_buffered) =
        time_buffered_request(&vec![b'h'; 8192], |buf| buf.len() >= 8192);
    assert!(
        elapsed < MOCK_READ_TIMEOUT / 2,
        "header cap took {elapsed:?}; per-read cap is {MOCK_READ_TIMEOUT:?}"
    );
    assert!(
        !still_buffered,
        "bytes were still in the socket after the 8192 cap"
    );
}

fn request_with_body(body_len: usize, sent: usize) -> Vec<u8> {
    let mut bytes =
        format!("POST /api/chat HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {body_len}\r\n\r\n")
            .into_bytes();
    bytes.extend(std::iter::repeat_n(b'x', sent));
    bytes
}

/// Time `read_http_request` once `ready` is true of the bytes already buffered.
/// The connection stays open until the function returns. The bool is true when
/// a further socket read returns `Ok(n)` with `n > 0`.
fn time_buffered_request(
    bytes: &[u8],
    ready: impl Fn(&[u8]) -> bool + Send + 'static,
) -> (std::time::Duration, bool) {
    time_buffered_request_split(bytes, ready, None)
}

/// `split_at` writes that prefix, pauses 50ms, then writes the rest.
///
/// The server peeks once before the client writes, so the pause is a gap in
/// the request rather than a delay inserted to win a race. `None` writes the
/// bytes in one `write_all`.
fn time_buffered_request_split(
    bytes: &[u8],
    ready: impl Fn(&[u8]) -> bool + Send + 'static,
    split_at: Option<usize>,
) -> (std::time::Duration, bool) {
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let split_at_for_server = split_at;
    let server = start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        if split_at_for_server.is_some() {
            let _ = sock.set_read_timeout(Some(std::time::Duration::from_millis(50)));
            let mut probe = [0u8; 1];
            if matches!(sock.peek(&mut probe), Ok(0)) {
                return;
            }
            if entered_tx.send(()).is_err() {
                return;
            }
        }
        if !wait_until_peek(&mut sock, &shutdown, ready) {
            return;
        }
        let started = std::time::Instant::now();
        read_http_request(&mut sock);
        let elapsed = started.elapsed();
        let still_buffered = unread_byte(&mut sock);
        let _ = done_tx.send((elapsed, still_buffered));
        let _ = shutdown.recv();
    });
    let mut client = std::net::TcpStream::connect(server.addr()).expect("connect");
    use std::io::Write;
    if let Some(split) = split_at {
        client.set_nodelay(true).expect("nodelay");
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("server is peeking");
        let n = split.min(bytes.len());
        client.write_all(&bytes[..n]).expect("request head");
        client.flush().expect("flush head");
        std::thread::sleep(std::time::Duration::from_millis(50));
        client.write_all(&bytes[n..]).expect("request tail");
        client.flush().expect("flush tail");
    } else {
        client.write_all(bytes).expect("request");
        client.flush().expect("flush");
    }
    let result = done_rx
        .recv_timeout(MOCK_READ_TIMEOUT + std::time::Duration::from_secs(1))
        .expect("read_http_request returned");
    drop(client);
    drop(server);
    result
}

#[test]
fn read_http_request_drops_the_read_tail_past_content_length() {
    // The header read fills `READ_CHUNK` and drops whatever in that buffer is
    // past `Content-Length`. A short excess disappears with the buffer. An
    // excess that does not fit in the same read stays in the socket. The
    // 4000-byte case waits until the whole request is buffered, the same way
    // the 200-byte case waits until its excess is visible.
    let (elapsed, still_buffered) = time_buffered_request(&request_with_body(3000, 3000), |buf| {
        header_block_end(buf).is_some_and(|end| buf.len() - end >= 3000)
    });
    assert!(
        elapsed < MOCK_READ_TIMEOUT / 2,
        "exact body took {elapsed:?}; per-read cap is {MOCK_READ_TIMEOUT:?}"
    );
    assert!(
        !still_buffered,
        "bytes were still in the socket after an exact Content-Length"
    );

    let small = request_with_body(4, 200);
    let small_end = header_block_end(&small).expect("small header");
    assert!(
        small_end + 200 <= READ_CHUNK,
        "the 200-byte excess must fit in the header read"
    );
    let (elapsed, still_buffered) = time_buffered_request(&small, |buf| {
        header_block_end(buf).is_some_and(|end| buf.len() - end >= 200)
    });
    assert!(
        elapsed < MOCK_READ_TIMEOUT / 2,
        "small excess took {elapsed:?}"
    );
    assert!(
        !still_buffered,
        "a 200-byte excess fits in the {READ_CHUNK}-byte header read and is dropped with that buffer"
    );

    let large = request_with_body(4, 4000);
    let large_end = header_block_end(&large).expect("large header");
    let large_len = large.len();
    assert!(
        large_end + 4000 > READ_CHUNK,
        "the 4000-byte excess must spill past one header read"
    );
    let (elapsed, still_buffered) =
        time_buffered_request(&large, move |buf| buf.len() >= large_len);
    assert!(
        elapsed < MOCK_READ_TIMEOUT / 2,
        "bytes already past Content-Length took {elapsed:?}"
    );
    assert!(
        still_buffered,
        "an excess past the {READ_CHUNK}-byte header read stays in the socket"
    );
}

#[test]
fn read_http_request_keeps_the_excess_when_the_write_is_split() {
    // 1024 bytes, a 50ms gap, then the rest. Waiting for `READ_CHUNK` returns
    // during that gap, before the excess is written. This waits for the whole
    // request, so the excess past one header read is still in the socket.
    let large = request_with_body(4, 4000);
    let large_len = large.len();
    assert!(
        large_len > READ_CHUNK,
        "the split request must be longer than one header read"
    );
    let (elapsed, still_buffered) =
        time_buffered_request_split(&large, move |buf| buf.len() >= large_len, Some(READ_CHUNK));
    assert!(
        elapsed < MOCK_READ_TIMEOUT / 2,
        "split excess took {elapsed:?}"
    );
    assert!(
        still_buffered,
        "an excess past the {READ_CHUNK}-byte header read stays in the socket after a split write"
    );
}

#[test]
fn unread_byte_is_false_when_the_peer_shuts_write_with_nothing_queued() {
    let (accepted_tx, accepted_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let server = start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        if accepted_tx.send(()).is_err() {
            return;
        }
        let _ = sock.set_read_timeout(Some(std::time::Duration::from_millis(50)));
        let mut buf = [0u8; 1];
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let saw_eof = loop {
            if shutdown_requested(&shutdown) || std::time::Instant::now() >= deadline {
                break false;
            }
            match sock.peek(&mut buf) {
                Ok(0) => break true,
                Ok(_) => break false,
                Err(err)
                    if err.kind() == std::io::ErrorKind::Interrupted
                        || err.kind() == std::io::ErrorKind::TimedOut
                        || err.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => break false,
            }
        };
        let pending = unread_byte(&mut sock);
        let _ = done_tx.send((saw_eof, pending));
        let _ = shutdown.recv();
    });
    let client = std::net::TcpStream::connect(server.addr()).expect("connect");
    accepted_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("accepted");
    client
        .shutdown(std::net::Shutdown::Write)
        .expect("shutdown write");
    let (saw_eof, pending) = done_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("unread_byte returned");
    assert!(saw_eof, "peer shutdown was not visible");
    assert!(
        !pending,
        "a shut write with nothing queued is not a buffered byte"
    );
    drop(client);
    drop(server);
}

#[test]
fn finish_response_half_close_delivers_with_trailing_bytes() {
    let (drained_tx, drained_rx) = std::sync::mpsc::channel::<()>();
    let (extra_tx, extra_rx) = std::sync::mpsc::channel::<()>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let server = start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        if drained_tx.send(()).is_err() {
            return;
        }
        if wait_or_stop(&extra_rx, &shutdown).is_none() {
            return;
        }
        let _ = sock.set_read_timeout(Some(MOCK_READ_TIMEOUT));
        let mut peek = [0u8; 1];
        let n = sock.peek(&mut peek).expect("trailing byte is buffered");
        assert!(n >= 1, "trailing byte was not buffered");
        use std::io::Write;
        let response = b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 4\r\nConnection: close\r\n\r\nnope";
        sock.write_all(response).expect("write response");
        sock.flush().expect("flush response");
        let drain = finish_response(sock);
        let _ = done_tx.send(());
        let _ = shutdown.recv();
        join_bounded(drain);
    });
    let mut client = std::net::TcpStream::connect(server.addr()).expect("connect");
    client.set_nodelay(true).expect("nodelay");
    let body = b"ping";
    let header = format!(
        "POST /api/chat HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    use std::io::{Read, Write};
    client.write_all(header.as_bytes()).expect("headers");
    client.write_all(body).expect("body");
    client.flush().expect("flush body");
    drained_rx.recv().expect("server drained the declared body");
    client.write_all(b"trailing").expect("trailing");
    client.flush().expect("flush trailing");
    extra_tx.send(()).expect("extra sent");
    done_rx
        .recv_timeout(MOCK_READ_TIMEOUT)
        .expect("server finished the response");
    client
        .set_read_timeout(Some(MOCK_READ_TIMEOUT / 2))
        .expect("client timeout");
    let started = std::time::Instant::now();
    let mut got = Vec::new();
    let mut tmp = [0u8; 256];
    loop {
        match client.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => got.extend_from_slice(&tmp[..n]),
            Err(err) => panic!(
                "response read ended with {err} after {} bytes: {}",
                got.len(),
                String::from_utf8_lossy(&got)
            ),
        }
    }
    assert!(
        started.elapsed() < MOCK_READ_TIMEOUT / 2,
        "clean EOF took {:?}",
        started.elapsed()
    );
    let text = String::from_utf8(got).expect("response utf-8");
    assert!(text.contains("500") && text.contains("nope"), "{text}");
    // This write fails when the mock drops the socket with unread bytes still
    // queued (RST). A peer that has already closed can still accept the write,
    // so success does not show that the background read is blocked in recv.
    client
        .write_all(b"MORE")
        .expect("MORE write failed; the mock reset the connection");
    client
        .shutdown(std::net::Shutdown::Write)
        .expect("shutdown after MORE");
    drop(client);
    drop(server);
}

fn write_chunk(sock: &mut std::net::TcpStream, data: &str) {
    use std::io::Write;
    let _ = write!(sock, "{:X}\r\n{data}\r\n", data.len());
    let _ = sock.flush();
}

/// NDJSON chat chunks, with a 50 ms gap after each one. `hang_after` stops
/// writing and leaves the socket open so the client must idle out.
///
/// `TCP_NODELAY` keeps macOS delayed ACK from holding a flushed chunk. The gap
/// is the shutdown wait: the mock leaves the loop when the test drops it,
/// instead of sleeping out a fixed hang. The client keeps chunks whose gaps
/// stay inside the idle window.
fn serve_slow_chunks(count: usize, hang_after: Option<usize>) -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        sock.set_nodelay(true).expect("nodelay");
        read_http_request(&mut sock);
        use std::io::Write;
        let _ = sock.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        );
        let _ = sock.flush();
        let stop = hang_after.unwrap_or(count);
        for i in 0..count.min(stop) {
            write_chunk(
                &mut sock,
                &format!("{{\"message\":{{\"content\":\"n{i:02}\"}},\"done\":false}}\n"),
            );
            if gap_elapsed(&shutdown) {
                continue;
            }
            let drain = finish_response(sock);
            join_bounded(drain);
            return;
        }
        if hang_after.is_none() {
            write_chunk(
                &mut sock,
                "{\"message\":{\"content\":\"\"},\"done\":true}\n",
            );
            let _ = sock.write_all(b"0\r\n\r\n");
            let _ = sock.flush();
        } else {
            // Keep the write side open until the client has finished. A FIN
            // here would end the stream before the idle timeout or the lifetime.
            let _ = shutdown.recv();
        }
        let drain = finish_response(sock);
        join_bounded(drain);
    })
}

fn run_against(
    mode: &str,
    addr: std::net::SocketAddr,
) -> (std::time::Duration, std::process::Output) {
    let started = std::time::Instant::now();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let out = std::process::Command::new(&exe)
        .args([mode, &format!("http://{addr}")])
        .env("OLLAMA_LOW_LATENCY", "1")
        .env("OLLAMA_TIMEOUT_SECS", "2")
        .env("OLLAMA_EARLY_EXIT", "0")
        .output()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()));
    (started.elapsed(), out)
}

#[test]
fn slow_chunks_longer_than_the_timeout_are_kept() {
    let server = serve_slow_chunks(60, None);
    let (elapsed, out) = run_against("complete", server.addr());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "slow stream failed after {:?}: {stdout} {stderr}",
        elapsed
    );
    for i in 0..60 {
        assert!(
            stdout.contains(&format!("n{i:02}")),
            "dropped token n{i:02} after {:?}: {stdout}",
            elapsed
        );
    }
    assert!(
        elapsed > std::time::Duration::from_secs(2),
        "stream finished inside the idle timeout ({elapsed:?})"
    );
    // macOS Test full delivered about thirty of these tokens in 4.01 s, so
    // sixty of them need more than the old 6 s ceiling. Fifteen seconds still
    // fails a client that waits out a harness hang or the default 180 s
    // timeout. A missing token fails above, on its own.
    assert!(
        elapsed < std::time::Duration::from_secs(15),
        "stream spun for {elapsed:?}"
    );
    drop(server);

    let server = serve_slow_chunks(60, None);
    let (elapsed, out) = run_against("qwen-stream", server.addr());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "qwen stream failed after {:?}: {stdout} {stderr}",
        elapsed
    );
    for i in 0..60 {
        assert!(
            stdout.contains(&format!("n{i:02}")),
            "qwen dropped token n{i:02} after {:?}: {stdout}",
            elapsed
        );
    }
    assert!(
        elapsed > std::time::Duration::from_secs(2),
        "qwen stream finished inside the idle timeout ({elapsed:?})"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(15),
        "qwen stream spun for {elapsed:?}"
    );
    drop(server);
}

/// Accept the TCP connection, read the request, and never send headers.
/// `first_not_found` answers `/api/chat` with 404 so the chat client falls
/// through to `/api/generate`.
fn serve_no_headers(first_not_found: bool) -> MockServer {
    start_server(move |listener, shutdown| {
        let mut drains = Vec::new();
        let mut first = true;
        loop {
            let Some(mut sock) = accept_client(&listener, &shutdown) else {
                break;
            };
            read_http_request(&mut sock);
            if first && first_not_found {
                first = false;
                use std::io::Write;
                let _ = sock.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                let _ = sock.flush();
                drains.push(finish_response(sock));
                continue;
            }
            // No status line until the client is done. The test drops the
            // server after the probe exits, and that is what releases this wait.
            let _ = shutdown.recv();
            drains.push(finish_response(sock));
            break;
        }
        join_drains(drains);
    })
}

fn assert_headers_fail_near_timeout(mode: &str, first_not_found: bool) {
    let server = serve_no_headers(first_not_found);
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args([mode, &format!("http://{}", server.addr())])
            .env("OLLAMA_LOW_LATENCY", "1")
            .env("OLLAMA_TIMEOUT_SECS", "1")
            .env("OLLAMA_EARLY_EXIT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            child.kill_and_reap();
            panic!(
                "{mode} still waiting for headers after {:?}",
                started.elapsed()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let elapsed = started.elapsed();
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!status.success(), "{mode} succeeded: {stdout} {stderr}");
    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "{mode} took {elapsed:?}: {stdout} {stderr}"
    );
    let msg = format!("{stdout} {stderr}").to_lowercase();
    assert!(
        msg.contains("headers")
            || msg.contains("timeout")
            || msg.contains("timed out")
            || msg.contains("error sending request"),
        "{mode}: {stdout} {stderr}"
    );
    drop(server);
}

/// Headers arrive, `Content-Length: 64` is declared, and no body bytes follow.
fn serve_declared_body(status_line: &'static str) -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        use std::io::Write;
        let _ = sock.write_all(
            format!("{status_line}\r\nContent-Length: 64\r\nConnection: close\r\n\r\n").as_bytes(),
        );
        let _ = sock.flush();
        let _ = shutdown.recv();
        let drain = finish_response(sock);
        join_bounded(drain);
    })
}

fn assert_body_fails_near_timeout(
    mode: &str,
    status_line: &'static str,
    low_latency: &str,
    needles: &[&str],
    absent: &[&str],
) {
    let server = serve_declared_body(status_line);
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args([mode, &format!("http://{}", server.addr())])
            .env("OLLAMA_LOW_LATENCY", low_latency)
            .env("OLLAMA_TIMEOUT_SECS", "1")
            .env("OLLAMA_EARLY_EXIT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            child.kill_and_reap();
            panic!(
                "{mode} still reading a missing body after {:?}",
                started.elapsed()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let elapsed = started.elapsed();
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!status.success(), "{mode} succeeded: {stdout} {stderr}");
    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "{mode} took {elapsed:?}: {stdout} {stderr}"
    );
    let msg = format!("{stdout} {stderr}").to_lowercase();
    for needle in needles {
        assert!(
            msg.contains(needle),
            "{mode} missing {needle}: {stdout} {stderr}"
        );
    }
    for needle in absent {
        assert!(
            !msg.contains(needle),
            "{mode} still contains {needle}: {stdout} {stderr}"
        );
    }
    drop(server);
}

#[test]
fn declared_body_that_never_arrives_fails_near_the_read_timeout() {
    assert_body_fails_near_timeout(
        "complete",
        "HTTP/1.1 500 Internal Server Error",
        "1",
        &["body read failed", "operation timed out"],
        &[],
    );
    assert_body_fails_near_timeout(
        "complete",
        "HTTP/1.1 500 Internal Server Error",
        "0",
        &["body read failed", "operation timed out"],
        &["initiating streaming post"],
    );
    assert_body_fails_near_timeout(
        "complete",
        "HTTP/1.1 200 OK",
        "1",
        &["operation timed out"],
        &[],
    );
    assert_body_fails_near_timeout(
        "complete",
        "HTTP/1.1 200 OK",
        "0",
        &["body read failed", "operation timed out"],
        &["initiating streaming post"],
    );
    assert_body_fails_near_timeout(
        "warmup",
        "HTTP/1.1 200 OK",
        "1",
        &["body read failed", "operation timed out"],
        &[],
    );
    for mode in ["qwen-think", "qwen-health", "phi-health", "hermes-health"] {
        assert_body_fails_near_timeout(mode, "HTTP/1.1 200 OK", "1", &["operation timed out"], &[]);
    }
    assert_body_fails_near_timeout(
        "qwen-preload",
        "HTTP/1.1 200 OK",
        "1",
        &["body read failed", "operation timed out"],
        &[],
    );
}

/// A readable error body must be the `complete` result. `OLLAMA_LOW_LATENCY=0`
/// used to log it and then start the streaming POST.
fn serve_readable_error(body: &'static str) -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        use std::io::Write;
        let _ = sock.write_all(
            format!(
                "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        );
        let _ = sock.flush();
        let drain = finish_response(sock);
        let _ = shutdown.recv();
        join_bounded(drain);
    })
}

#[test]
fn readable_error_status_does_not_start_streaming() {
    let server = serve_readable_error("nope");
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args(["complete", &format!("http://{}", server.addr())])
            .env("OLLAMA_LOW_LATENCY", "0")
            .env("OLLAMA_TIMEOUT_SECS", "1")
            .env("OLLAMA_EARLY_EXIT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            child.kill_and_reap();
            panic!(
                "complete still running after a readable 500: {:?}",
                started.elapsed()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let msg = format!("{stdout} {stderr}").to_lowercase();
    assert!(!status.success(), "complete succeeded: {stdout} {stderr}");
    assert!(
        msg.contains("nope") && msg.contains("500"),
        "missing the error body: {stdout} {stderr}"
    );
    assert!(
        !msg.contains("initiating streaming post"),
        "readable 500 started streaming: {stdout} {stderr}"
    );
    drop(server);
}

fn serve_body(body: Vec<u8>) -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_response(sock, header.as_bytes(), &body);
        let _ = shutdown.recv();
    })
}

fn run_complete(addr: std::net::SocketAddr, early_exit: &str) -> std::process::Output {
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args(["complete", &format!("http://{addr}")])
            .env("OLLAMA_LOW_LATENCY", "1")
            .env("OLLAMA_TIMEOUT_SECS", "5")
            .env("OLLAMA_EARLY_EXIT", early_exit)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    // Drain while the child runs. A 64 KiB success fills the pipe and blocks
    // if the parent waits to read until after the child exits.
    let mut stdout = child.stdout();
    let mut stderr = child.stderr();
    let stdout_task = std::thread::spawn(move || {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut stdout, &mut buf).expect("read stdout");
        buf
    });
    let stderr_task = std::thread::spawn(move || {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut stderr, &mut buf).expect("read stderr");
        buf
    });
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            child.kill_and_reap();
            let _ = stdout_task.join();
            let _ = stderr_task.join();
            panic!("complete was still open after 4s");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    std::process::Output {
        status,
        stdout: stdout_task.join().expect("stdout thread"),
        stderr: stderr_task.join().expect("stderr thread"),
    }
}

fn serve_oversized_ok() -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        let body = vec![b'a'; astraweave_llm::MAX_MODEL_STREAM_BYTES + 8];
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_response(sock, header.as_bytes(), &body);
        let _ = shutdown.recv();
    })
}

/// Write a complete response, then half-close until the client closes or a read times out.
fn write_response(mut sock: std::net::TcpStream, header: &[u8], body: &[u8]) {
    use std::io::Write;
    let _ = sock.write_all(header);
    let _ = sock.write_all(body);
    let _ = sock.flush();
    let drain = finish_response(sock);
    join_bounded(drain);
}

#[test]
fn oversized_chat_body_stops_at_the_stream_budget() {
    let server = serve_oversized_ok();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args(["complete", &format!("http://{}", server.addr())])
            .env("OLLAMA_LOW_LATENCY", "1")
            .env("OLLAMA_TIMEOUT_SECS", "5")
            .env("OLLAMA_EARLY_EXIT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            child.kill_and_reap();
            panic!("complete kept reading past the stream budget");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !status.success(),
        "oversized body succeeded: {stdout} {stderr}"
    );
    assert!(
        format!("{stdout} {stderr}").contains("model stream exceeds"),
        "{stdout} {stderr}"
    );
    drop(server);
}

fn serve_empty_lines() -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        // Empty NDJSON lines do not grow the kept text. The byte counter must
        // still stop the read.
        let body = vec![b'\n'; astraweave_llm::MAX_MODEL_STREAM_BYTES + 8];
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_response(sock, header.as_bytes(), &body);
        let _ = shutdown.recv();
    })
}

#[test]
fn empty_ndjson_lines_stop_at_the_stream_budget() {
    let server = serve_empty_lines();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args(["complete", &format!("http://{}", server.addr())])
            .env("OLLAMA_LOW_LATENCY", "1")
            .env("OLLAMA_TIMEOUT_SECS", "5")
            .env("OLLAMA_EARLY_EXIT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            child.kill_and_reap();
            panic!("empty NDJSON lines were still streaming");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !status.success(),
        "empty lines succeeded: {stdout} {stderr}"
    );
    assert!(
        format!("{stdout} {stderr}").contains("model stream exceeds"),
        "{stdout} {stderr}"
    );
    drop(server);
}

fn hermes_token_line(token: &str, done: bool) -> String {
    format!(
        r#"{{"model":"adrienbrault/nous-hermes2pro:Q4_K_M","created_at":"2026-10-23T00:00:00.000000000Z","message":{{"role":"assistant","content":"{token}"}},"done":{done}}}"#
    )
}

fn serve_512_token_reply() -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        let mut body = String::new();
        for i in 0..512 {
            let done = i == 511;
            body.push_str(&hermes_token_line(&format!("t{i:03}"), done));
            body.push('\n');
        }
        let bytes = body.into_bytes();
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        );
        write_response(sock, header.as_bytes(), &bytes);
        let _ = shutdown.recv();
    })
}

#[test]
fn a_512_token_reply_is_kept() {
    let sample = hermes_token_line("t000", false);
    assert!(
        sample.len() >= 140 && sample.len() <= 180,
        "envelope drifted from a Hermes line: {}",
        sample.len()
    );
    assert!(
        512 * sample.len() > 64 * 1024,
        "512 tokens fit in the old 64 KiB raw cap"
    );
    assert!(512 * (sample.len() + 1) < astraweave_llm::MAX_MODEL_STREAM_BYTES);
    let server = serve_512_token_reply();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args(["complete", &format!("http://{}", server.addr())])
            .env("OLLAMA_LOW_LATENCY", "1")
            .env("OLLAMA_TIMEOUT_SECS", "5")
            .env("OLLAMA_EARLY_EXIT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(8) {
            child.kill_and_reap();
            panic!("512-token reply was still open");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        status.success(),
        "512-token reply was capped: {stdout} {stderr}"
    );
    assert!(stdout.contains("t000"), "missing first token: {stdout}");
    assert!(stdout.contains("t511"), "missing last token: {stdout}");
    drop(server);
}

#[test]
fn plain_text_over_64kib_is_rejected() {
    for early in ["0", "1"] {
        let server = serve_body(vec![b'a'; 80 * 1024]);
        let output = run_complete(server.addr(), early);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "80 KiB plain text succeeded with early_exit={early}: {stdout} {stderr}"
        );
        assert!(
            format!("{stdout} {stderr}").contains("64 KiB"),
            "early_exit={early}: {stdout} {stderr}"
        );
        assert!(
            stdout.len() < 4 * 1024,
            "80 KiB plain text was returned with early_exit={early}: {} bytes",
            stdout.len()
        );
        drop(server);
    }
}

#[test]
fn plain_text_of_exactly_64kib_is_kept() {
    let server = serve_body(vec![b'a'; astraweave_llm::MAX_MODEL_JSON_BYTES]);
    let output = run_complete(server.addr(), "1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "exactly 64 KiB was rejected: {stderr} stdout_len={}",
        stdout.len()
    );
    let body = stdout.rsplit_once('\n').expect("log line").1;
    assert_eq!(body.len(), astraweave_llm::MAX_MODEL_JSON_BYTES, "{stderr}");
    assert!(body.bytes().all(|byte| byte == b'a'), "{stderr}");
    drop(server);
}

#[test]
fn message_content_over_64kib_without_a_newline_is_rejected() {
    for early in ["0", "1"] {
        let mut body = br#"{"message":{"content":""#.to_vec();
        body.extend(std::iter::repeat(b'q').take(70 * 1024));
        body.extend(br#""}}"#);
        assert!(!body.contains(&b'\n'));
        let server = serve_body(body);
        let output = run_complete(server.addr(), early);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "70 KiB content succeeded with early_exit={early}: {} bytes {stderr}",
            stdout.len()
        );
        assert!(
            format!("{stdout} {stderr}").contains("64 KiB"),
            "early_exit={early}: {stdout} {stderr}"
        );
        assert!(
            stdout.len() < 4 * 1024,
            "70 KiB content was returned with early_exit={early}: {} bytes",
            stdout.len()
        );
        drop(server);
    }
}

#[test]
fn a_mebibyte_of_newlines_finishes() {
    let server = serve_body(vec![b'\n'; astraweave_llm::MAX_MODEL_STREAM_BYTES]);
    let started = std::time::Instant::now();
    let output = run_complete(server.addr(), "0");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(4),
        "newline drain took {:?}",
        started.elapsed()
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "newline body succeeded: {stdout} {stderr}"
    );
    assert!(
        !format!("{stdout} {stderr}").contains("model stream exceeds"),
        "the inclusive raw budget rejected an exact 1 MiB body: {stdout} {stderr}"
    );
    assert!(stdout.len() < 4 * 1024, "{}", stdout.len());
    drop(server);
}

fn serve_bad_utf8_line() -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        let body = b"\xff\n";
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_response(sock, header.as_bytes(), body);
        let _ = shutdown.recv();
    })
}

#[test]
fn invalid_utf8_line_is_an_error() {
    let server = serve_bad_utf8_line();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args(["complete", &format!("http://{}", server.addr())])
            .env("OLLAMA_LOW_LATENCY", "1")
            .env("OLLAMA_TIMEOUT_SECS", "2")
            .env("OLLAMA_EARLY_EXIT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            child.kill_and_reap();
            panic!("invalid utf-8 line was still open");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !status.success(),
        "invalid utf-8 succeeded: {stdout} {stderr}"
    );
    assert!(
        format!("{stdout} {stderr}")
            .to_lowercase()
            .contains("utf-8"),
        "{stdout} {stderr}"
    );
    drop(server);
}

fn serve_good_line_then_bad_utf8() -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        let body = b"{\"message\":{\"content\":\"hello\"}}\n\xff\n";
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        write_response(sock, header.as_bytes(), body);
        let _ = shutdown.recv();
    })
}

#[test]
fn good_line_before_bad_utf8_is_kept() {
    let server = serve_good_line_then_bad_utf8();
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args(["complete", &format!("http://{}", server.addr())])
            .env("OLLAMA_LOW_LATENCY", "1")
            .env("OLLAMA_TIMEOUT_SECS", "2")
            .env("OLLAMA_EARLY_EXIT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(4) {
            child.kill_and_reap();
            panic!("good line then bad utf-8 was still open");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let output = child.wait_with_output().expect("output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        status.success(),
        "good token was dropped: {stdout} {stderr}"
    );
    assert!(
        stdout.contains("hello"),
        "kept token missing: {stdout} {stderr}"
    );
    drop(server);
}

#[test]
fn silent_accept_fails_near_the_header_timeout() {
    assert_headers_fail_near_timeout("complete", false);
    assert_headers_fail_near_timeout("complete", true);
    assert_headers_fail_near_timeout("qwen-stream", false);
    assert_headers_fail_near_timeout("hermes-stream", false);
}

fn run_until(
    mode: &str,
    addr: std::net::SocketAddr,
    idle_secs: &str,
    lifetime_ms: &str,
    kill_after: std::time::Duration,
) -> (std::time::Duration, std::process::Output) {
    let exe = probe_exe(env!("CARGO_BIN_EXE_ollama_chat_probe"));
    let started = std::time::Instant::now();
    let mut child = ReapedChild::spawn(
        std::process::Command::new(&exe)
            .args([mode, &format!("http://{addr}")])
            .env("OLLAMA_LOW_LATENCY", "1")
            .env("OLLAMA_TIMEOUT_SECS", idle_secs)
            .env("OLLAMA_STREAM_LIFETIME_MS", lifetime_ms)
            .env("OLLAMA_EARLY_EXIT", "0")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()),
    );
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > kill_after {
            child.kill_and_reap();
            panic!("{mode} still running after {kill_after:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let elapsed = started.elapsed();
    let mut output = child.wait_with_output().expect("output");
    output.status = status;
    (elapsed, output)
}

/// Headers arrive and the declared body never does.
/// `first_not_found` answers `/api/chat` with an empty 404 so `complete`
/// reads the withheld body from `/api/generate`.
fn serve_withheld_body(status_line: &'static str, first_not_found: bool) -> MockServer {
    start_server(move |listener, shutdown| {
        let mut drains = Vec::new();
        let mut first = true;
        loop {
            let Some(mut sock) = accept_client(&listener, &shutdown) else {
                break;
            };
            read_http_request(&mut sock);
            use std::io::Write;
            if first && first_not_found {
                first = false;
                let _ = sock.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                let _ = sock.flush();
                drains.push(finish_response(sock));
                continue;
            }
            let _ = write!(
                sock,
                "{status_line}\r\nContent-Length: 64\r\nConnection: close\r\n\r\n"
            );
            let _ = sock.flush();
            let _ = shutdown.recv();
            drains.push(finish_response(sock));
            break;
        }
        join_drains(drains);
    })
}

#[test]
fn withheld_error_body_ends_at_the_lifetime() {
    // Thinking mode reads the JSON body only after a success status, so
    // that path withholds a 200. `complete` withholds the 500, including
    // the `/api/generate` fallback.
    for (mode, status_line, first_not_found) in [
        ("complete", "HTTP/1.1 500 Internal Server Error", false),
        ("complete", "HTTP/1.1 500 Internal Server Error", true),
        ("qwen-think", "HTTP/1.1 200 OK", false),
    ] {
        let server = serve_withheld_body(status_line, first_not_found);
        let (elapsed, out) = run_until(
            mode,
            server.addr(),
            "20",
            "400",
            std::time::Duration::from_secs(3),
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "{mode} first_not_found={first_not_found} succeeded: {stdout} {stderr}"
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(250),
            "{mode} returned before the lifetime ({elapsed:?}): {stdout} {stderr}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "{mode} waited out the idle timeout ({elapsed:?}): {stdout} {stderr}"
        );
        let msg = format!("{stdout} {stderr}");
        assert!(msg.contains("streaming lifetime deadline"), "{mode}: {msg}");
        assert!(
            !msg.contains("operation timed out"),
            "{mode} used the idle read timeout: {msg}"
        );
        drop(server);
    }
}

#[test]
fn withheld_headers_end_at_the_lifetime() {
    for (mode, first_not_found) in [
        ("complete", false),
        ("complete", true),
        ("hermes-stream", false),
        ("qwen-stream", false),
    ] {
        let server = serve_no_headers(first_not_found);
        let (elapsed, out) = run_until(
            mode,
            server.addr(),
            "20",
            "400",
            std::time::Duration::from_secs(3),
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "{mode} first_not_found={first_not_found} succeeded: {stdout} {stderr}"
        );
        assert!(
            elapsed >= std::time::Duration::from_millis(250),
            "{mode} returned before the lifetime ({elapsed:?}): {stdout} {stderr}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "{mode} waited out the idle timeout ({elapsed:?}): {stdout} {stderr}"
        );
        let msg = format!("{stdout} {stderr}");
        assert!(msg.contains("streaming lifetime deadline"), "{mode}: {msg}");
        drop(server);
    }
}

/// Headers and a token are written only after the probe has already returned.
/// The call fails on the header wait and does not accept the body.
fn serve_late_headers() -> MockServer {
    start_server(move |listener, shutdown| {
        let Some(mut sock) = accept_client(&listener, &shutdown) else {
            return;
        };
        read_http_request(&mut sock);
        let _ = shutdown.recv();
        use std::io::Write;
        let body = "{\"message\":{\"content\":\"late-token\"},\"done\":true}\n";
        let _ = write!(
            sock,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = sock.flush();
        let drain = finish_response(sock);
        join_bounded(drain);
    })
}

#[test]
fn late_headers_fail_cleanly() {
    for mode in ["complete", "qwen-stream"] {
        let server = serve_late_headers();
        let (elapsed, out) = run_until(
            mode,
            server.addr(),
            "10",
            "300",
            std::time::Duration::from_secs(3),
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "{mode} accepted late headers: {stdout} {stderr}"
        );
        assert!(
            !stdout.contains("late-token"),
            "{mode} kept the late body: {stdout} {stderr}"
        );
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "{mode} waited for the late headers ({elapsed:?}): {stdout} {stderr}"
        );
        let msg = format!("{stdout} {stderr}");
        assert!(msg.contains("streaming lifetime deadline"), "{mode}: {msg}");
        drop(server);
    }
}

#[test]
fn lifetime_after_a_token_returns_the_token() {
    let server = serve_slow_chunks(1, Some(1));
    let (elapsed, out) = run_until(
        "complete",
        server.addr(),
        "10",
        "400",
        std::time::Duration::from_secs(3),
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "token before the lifetime was dropped: {stdout} {stderr}"
    );
    assert!(
        stdout.contains("n00"),
        "kept token missing: {stdout} {stderr}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "lifetime after a token waited out the idle timeout ({elapsed:?})"
    );
    drop(server);
}

#[test]
fn idle_timeout_keeps_partial_chunks() {
    let server = serve_slow_chunks(5, Some(5));
    let (elapsed, out) = run_against("complete", server.addr());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "idle wait spun for {elapsed:?}: {stdout} {stderr}"
    );
    assert!(
        out.status.success(),
        "partial stream failed: {stdout} {stderr}"
    );
    for i in 0..5 {
        assert!(
            stdout.contains(&format!("n{i:02}")),
            "dropped partial token n{i:02}: {stdout}"
        );
    }
    drop(server);
}
