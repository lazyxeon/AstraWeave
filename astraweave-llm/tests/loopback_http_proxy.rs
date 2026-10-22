#![cfg(feature = "ollama")]

//! Proves `loopback_http_client` ignores proxy environment variables.
//!
//! The variables are set on the child with `Command::env`. The library crate
//! is `forbid(unsafe_code)` and does not call `set_var` or `remove_var`.
//!
//! `CARGO_BIN_EXE_loopback_http_probe` is an absolute path on the machine that
//! compiled the test. Windows partition jobs only have the nextest archive, so
//! profile `ci-pr-archive` skips this target and the archive build job runs it.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

struct Shot {
    addr: std::net::SocketAddr,
    hit: Arc<AtomicBool>,
}

fn serve(body: &'static str) -> Shot {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let hit = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&hit);
    thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        flag.store(true, Ordering::SeqCst);
        let _ = sock.set_read_timeout(Some(Duration::from_secs(2)));
        let mut buf = [0u8; 8192];
        let _ = sock.read(&mut buf);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = sock.write_all(resp.as_bytes());
        let _ = sock.flush();
    });
    Shot { addr, hit }
}

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

fn probe(mode: &str, url: &str, proxy: std::net::SocketAddr) -> std::process::Output {
    let proxy_url = format!("http://{proxy}");
    let exe = probe_exe(env!("CARGO_BIN_EXE_loopback_http_probe"));
    Command::new(&exe)
        .args([mode, url])
        .env("HTTP_PROXY", &proxy_url)
        .env("http_proxy", &proxy_url)
        .env("HTTPS_PROXY", &proxy_url)
        .env("https_proxy", &proxy_url)
        .env("ALL_PROXY", &proxy_url)
        .env("all_proxy", &proxy_url)
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .env_remove("REQUEST_METHOD")
        .output()
        .unwrap_or_else(|err| panic!("spawn {}: {err}", exe.display()))
}

#[test]
fn loopback_http_client_ignores_proxy_env() {
    let target = serve("direct");
    let proxy = serve("proxied");
    let url = format!("http://localhost:{}/", target.addr.port());
    let out = probe("loopback", &url, proxy.addr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "loopback probe failed: status {:?} stdout {stdout} stderr {stderr}",
        out.status.code()
    );
    assert_eq!(stdout.trim(), "direct");
    assert!(
        !proxy.hit.load(Ordering::SeqCst),
        "loopback client contacted the proxy"
    );

    let target = serve("direct");
    let proxy = serve("proxied");
    let url = format!("http://localhost:{}/", target.addr.port());
    let out = probe("default", &url, proxy.addr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "default probe failed: status {:?} stdout {stdout} stderr {stderr}",
        out.status.code()
    );
    assert_eq!(stdout.trim(), "proxied", "stdout {stdout} stderr {stderr}");
    assert!(
        proxy.hit.load(Ordering::SeqCst),
        "default client skipped the proxy"
    );
}
