//! Crash log, stage timings, and a native error dialog.
//!
//! A windowed double-click has no console. Fatal failures write
//! `aravath_crash.log` under the user data directory and, on Windows, open a
//! message box. The demo does not open a network connection or update itself.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);
static START: Mutex<Option<Instant>> = Mutex::new(None);

pub fn install() {
    let dir = aravath_runtime::assets::log_dir();
    match aravath_runtime::assets::open_capped_log(
        &dir,
        "aravath.log",
        aravath_runtime::assets::MAX_LOG_BYTES,
    ) {
        Ok(file) => {
            if let Ok(mut slot) = LOG.lock() {
                *slot = Some(file);
            }
        }
        Err(err) => eprintln!("aravath: could not open log in {}: {err}", dir.display()),
    }
    if let Ok(mut start) = START.lock() {
        *start = Some(Instant::now());
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = format!(
            "panic: {info}\n{}",
            std::backtrace::Backtrace::force_capture()
        );
        record_crash(&message);
        message_box("Aravath crashed", &message);
        previous(info);
    }));
    let _ = log::set_logger(&FILE_LOGGER);
    log::set_max_level(log::LevelFilter::Info);
    line("startup log ready");
}

struct FileLogger;

static FILE_LOGGER: FileLogger = FileLogger;

impl log::Log for FileLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        line(&format!("{} {}", record.level(), record.args()));
    }

    fn flush(&self) {}
}

pub fn exe_dir() -> PathBuf {
    aravath_runtime::assets::exe_dir()
}

pub fn stage(name: &str) {
    let elapsed = START
        .lock()
        .ok()
        .and_then(|slot| slot.map(|start| start.elapsed().as_millis()))
        .unwrap_or(0);
    line(&format!("stage {name} at {elapsed} ms"));
}

pub fn line(text: &str) {
    let text = aravath_runtime::assets::bounded_text(text, 16 * 1024);
    eprintln!("{text}");
    if let Ok(mut slot) = LOG.lock() {
        if let Some(file) = slot.as_mut() {
            if let Err(err) = aravath_runtime::assets::write_log_line(file, &text) {
                eprintln!("aravath: log write failed: {err}");
            }
        }
    }
}

pub fn fatal(title: &str, message: &str) -> ! {
    record_crash(message);
    message_box(title, message);
    std::process::exit(1);
}

fn record_crash(message: &str) {
    let message = aravath_runtime::assets::bounded_text(message, 64 * 1024);
    line(&message);
    let dir = aravath_runtime::assets::log_dir();
    match aravath_runtime::assets::open_capped_log(
        &dir,
        "aravath_crash.log",
        aravath_runtime::assets::MAX_LOG_BYTES,
    ) {
        Ok(mut file) => {
            if let Err(err) = aravath_runtime::assets::write_log_line(&mut file, &message) {
                eprintln!("aravath: crash log write failed: {err}");
            }
        }
        Err(err) => eprintln!("aravath: could not open crash log: {err}"),
    }
}

pub fn message_box(title: &str, message: &str) {
    #[cfg(windows)]
    {
        win_message_box(title, message);
    }
    #[cfg(not(windows))]
    {
        let _ = (title, message);
    }
}

#[cfg(windows)]
fn win_message_box(title: &str, message: &str) {
    use std::os::windows::ffi::OsStrExt;
    fn wide(text: &str) -> Vec<u16> {
        std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
    let title = wide(&aravath_runtime::assets::bounded_text(title, 120));
    let body = wide(&aravath_runtime::assets::bounded_text(message, 1200));
    // SAFETY: MessageBoxW is the Win32 modal dialog. Pointers are null-terminated
    // UTF-16 buffers that live for the duration of the call. A null hwnd is the desktop.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            0x00000010,
        );
    }
}

#[cfg(windows)]
unsafe extern "system" {
    fn MessageBoxW(
        hwnd: *mut std::ffi::c_void,
        text: *const u16,
        caption: *const u16,
        kind: u32,
    ) -> i32;
}

/// Paint the window client area before the renderer has presented a frame.
#[cfg(windows)]
pub fn paint_loading(window: &winit::window::Window, text: &str) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::os::windows::ffi::OsStrExt;
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return;
    };
    let hwnd = win32.hwnd.get() as *mut std::ffi::c_void;
    // SAFETY: hwnd comes from the winit Win32 handle. GDI calls use that window
    // and a DC obtained from it, released before return. The text buffer outlives the call.
    unsafe {
        let hdc = GetDC(hwnd);
        if hdc.is_null() {
            return;
        }
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetClientRect(hwnd, &mut rect);
        let brush = CreateSolidBrush(0x006E4628);
        FillRect(hdc, &rect, brush);
        DeleteObject(brush);
        SetBkMode(hdc, 1);
        SetTextColor(hdc, 0x00F2E6D8);
        let wide: Vec<u16> = std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        DrawTextW(hdc, wide.as_ptr(), -1, &mut rect, 0x25);
        ReleaseDC(hwnd, hdc);
    }
}

#[cfg(not(windows))]
pub fn paint_loading(_window: &winit::window::Window, _text: &str) {}

#[cfg(windows)]
#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(windows)]
unsafe extern "system" {
    fn GetDC(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn ReleaseDC(hwnd: *mut std::ffi::c_void, hdc: *mut std::ffi::c_void) -> i32;
    fn GetClientRect(hwnd: *mut std::ffi::c_void, rect: *mut Rect) -> i32;
    fn FillRect(hdc: *mut std::ffi::c_void, rect: *const Rect, brush: *mut std::ffi::c_void)
        -> i32;
    fn CreateSolidBrush(color: u32) -> *mut std::ffi::c_void;
    fn DeleteObject(obj: *mut std::ffi::c_void) -> i32;
    fn SetBkMode(hdc: *mut std::ffi::c_void, mode: i32) -> i32;
    fn SetTextColor(hdc: *mut std::ffi::c_void, color: u32) -> u32;
    fn DrawTextW(
        hdc: *mut std::ffi::c_void,
        text: *const u16,
        len: i32,
        rect: *mut Rect,
        format: u32,
    ) -> i32;
}
