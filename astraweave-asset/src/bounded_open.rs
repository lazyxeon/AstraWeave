//! Size-bounded read from a directory handle.
//!
//! The caller-supplied root is resolved once. On Unix each URI component is
//! then opened from the parent directory handle with `O_NOFOLLOW`. On Windows
//! the resolved root is opened without reparse-point semantics, each URI
//! component is reopened by path and any reparse point is rejected, and the
//! file's final path must stay under the root's final path. Those two paths
//! compare as exact UTF-16 units. A caller-supplied model file is resolved
//! once before its leaf is opened. URI components are not. File type and
//! size come from the opened handle, and the read stops at the caller's byte cap.

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

pub(crate) fn read_path(path: &Path, max_bytes: u64, noun: &str) -> Result<Vec<u8>> {
    // Resolve the caller-supplied file once, then open that leaf. A symlink
    // at the file itself loads. `canonicalize` uses `O_PATH` here, so a FIFO
    // returns without a blocking read; the leaf open still rejects it.
    let resolved =
        std::fs::canonicalize(path).with_context(|| format!("open {}", subject(noun, path)))?;
    open_leaf(&resolved, max_bytes, noun, path)
}

/// Open `path` without resolving a symlink at the file.
pub(crate) fn read_unresolved(path: &Path, max_bytes: u64, noun: &str) -> Result<Vec<u8>> {
    open_leaf(path, max_bytes, noun, path)
}

fn open_leaf(path: &Path, max_bytes: u64, noun: &str, display: &Path) -> Result<Vec<u8>> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let name = path
        .file_name()
        .with_context(|| format!("open {}", subject(noun, display)))?;
    read_under(parent, Path::new(name), max_bytes, noun, display)
}

pub(crate) fn read_under(
    root: &Path,
    relative: &Path,
    max_bytes: u64,
    noun: &str,
    display: &Path,
) -> Result<Vec<u8>> {
    let mut file = match open_under(root, relative) {
        Ok(file) => file,
        Err(err) if not_regular_open(&err) => {
            bail!("{} is not a regular file", subject(noun, display));
        }
        Err(err) => {
            return Err(err).with_context(|| format!("open {}", subject(noun, display)));
        }
    };
    let meta = file
        .metadata()
        .with_context(|| format!("stat {}", subject(noun, display)))?;
    if !regular_file(&meta) {
        bail!("{} is not a regular file", subject(noun, display));
    }
    if meta.len() > max_bytes {
        bail!(
            "{} is {} bytes; limit is {max_bytes}",
            subject(noun, display),
            meta.len()
        );
    }
    let mut bytes = Vec::new();
    Read::take(&mut file, max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .with_context(|| format!("read {}", subject(noun, display)))?;
    if bytes.len() as u64 > max_bytes {
        bail!("{} exceeds {max_bytes} bytes", subject(noun, display));
    }
    Ok(bytes)
}

fn subject(noun: &str, path: &Path) -> String {
    if noun.is_empty() {
        path.display().to_string()
    } else {
        format!("{noun} {}", path.display())
    }
}

fn not_regular_open(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::InvalidInput && err.to_string().contains("not a regular file")
}

fn regular_file(meta: &std::fs::Metadata) -> bool {
    if reparse_point(meta) {
        return false;
    }
    meta.is_file()
}

#[cfg(windows)]
fn reparse_point(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn reparse_point(_meta: &std::fs::Metadata) -> bool {
    false
}

fn relative_names(relative: &Path) -> std::io::Result<Vec<std::ffi::OsString>> {
    let mut names = Vec::new();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(name) => names.push(name.to_os_string()),
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "unsupported path component",
                ));
            }
        }
    }
    if names.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "empty relative path",
        ));
    }
    Ok(names)
}

#[cfg(unix)]
fn open_under(root: &Path, relative: &Path) -> std::io::Result<File> {
    use rustix::fs::{openat, Mode, OFlags, CWD};

    // The root is resolved once, then opened as a directory. URI components
    // keep NOFOLLOW so a link under the root is not followed.
    let root_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
    let directory = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file_flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
    let names = relative_names(relative)?;
    let resolved = std::fs::canonicalize(root)?;
    let mut dir = opened(openat(CWD, &resolved, root_flags, Mode::empty()))?;
    for (index, name) in names.iter().enumerate() {
        let last = index + 1 == names.len();
        let flags = if last { file_flags } else { directory };
        dir = opened(openat(&dir, name.as_os_str(), flags, Mode::empty()))?;
    }
    Ok(File::from(dir))
}

#[cfg(unix)]
fn opened(result: rustix::io::Result<rustix::fd::OwnedFd>) -> std::io::Result<rustix::fd::OwnedFd> {
    match result {
        Ok(fd) => Ok(fd),
        Err(rustix::io::Errno::LOOP) | Err(rustix::io::Errno::NOTDIR) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        )),
        Err(err) => Err(err.into()),
    }
}

#[cfg(all(windows, test))]
thread_local! {
    static FILE_FINAL_OVERRIDE: std::cell::Cell<Option<std::path::PathBuf>> =
        std::cell::Cell::new(None);
}

#[cfg(windows)]
fn open_under(root: &Path, relative: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    fn open_path(path: &Path, reparse: bool) -> std::io::Result<File> {
        let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
        if reparse {
            flags |= FILE_FLAG_OPEN_REPARSE_POINT;
        }
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(flags)
            .open(path)
    }

    fn ensure_directory(file: &File) -> std::io::Result<()> {
        let meta = file.metadata()?;
        if reparse_point(&meta) || !meta.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        Ok(())
    }

    let names = relative_names(relative)?;
    // Resolve the caller-supplied root once, then open that directory by
    // handle without reparse-point semantics. URI components below keep it.
    let resolved = std::fs::canonicalize(root)?;
    let root_file = open_path(&resolved, false)?;
    ensure_directory(&root_file)?;
    let root_final = final_path(&root_file)?;
    let mut held = vec![root_file];

    let mut cursor = resolved;
    for (index, name) in names.iter().enumerate() {
        cursor.push(name);
        let file = open_path(&cursor, true)?;
        let last = index + 1 == names.len();
        if !last {
            ensure_directory(&file)?;
            held.push(file);
            continue;
        }
        let mut file_final = final_path(&file)?;
        #[cfg(test)]
        if let Some(overridden) = FILE_FINAL_OVERRIDE.with(|cell| cell.take()) {
            file_final = overridden;
        }
        if !path_is_under(&root_final, &file_final) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        drop(held);
        return Ok(file);
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "empty relative path",
    ))
}

#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "GetFinalPathNameByHandleW has no safe wrapper in this crate"
)]
fn final_path(file: &File) -> std::io::Result<std::path::PathBuf> {
    use std::ffi::{c_void, OsString};
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFinalPathNameByHandleW(
            file: *mut c_void,
            buffer: *mut u16,
            length: u32,
            flags: u32,
        ) -> u32;
    }

    const VOLUME_NAME_DOS: u32 = 0;
    let handle = file.as_raw_handle();
    let mut buf = vec![0u16; 512];
    let mut grew = false;
    loop {
        // The handle is open and the buffer length is the capacity passed in.
        let n = unsafe {
            GetFinalPathNameByHandleW(handle, buf.as_mut_ptr(), buf.len() as u32, VOLUME_NAME_DOS)
        };
        if n == 0 {
            return Err(std::io::Error::last_os_error());
        }
        let n = n as usize;
        if n < buf.len() {
            buf.truncate(n);
            return Ok(std::path::PathBuf::from(OsString::from_wide(&buf)));
        }
        if grew || n > 32_768 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "path is too long",
            ));
        }
        buf.resize(n, 0);
        grew = true;
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn path_is_under(root: &Path, file: &Path) -> bool {
    windows_final_path_is_under_units(&os_units(root.as_os_str()), &os_units(file.as_os_str()))
}

fn os_units(os: &std::ffi::OsStr) -> Vec<u16> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        // `encode_wide` keeps unpaired surrogates. `to_string_lossy` would
        // fold those names onto one U+FFFD string.
        os.encode_wide().collect()
    }
    #[cfg(not(windows))]
    {
        // Production compares Windows final paths. A non-Unicode test path
        // fails closed instead of substituting U+FFFD.
        match os.to_str() {
            Some(text) => text.encode_utf16().collect(),
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
fn windows_final_path_is_under(root: &str, file: &str) -> bool {
    windows_final_path_is_under_units(
        &root.encode_utf16().collect::<Vec<_>>(),
        &file.encode_utf16().collect::<Vec<_>>(),
    )
}

/// Component prefix check for two Windows final paths.
///
/// Both slices are the UTF-16 `GetFinalPathNameByHandleW` returns, including
/// unpaired surrogates. A component matches only when its units are equal.
/// A stream suffix, a `.` or `..` component, or a different device is not
/// under the root.
fn windows_final_path_is_under_units(root: &[u16], file: &[u16]) -> bool {
    let root_parts = match windows_final_components(root) {
        Some(parts) => parts,
        None => return false,
    };
    let file_parts = match windows_final_components(file) {
        Some(parts) => parts,
        None => return false,
    };
    if file_parts.len() <= root_parts.len() {
        return false;
    }
    root_parts
        .iter()
        .zip(&file_parts)
        .all(|(root_part, file_part)| root_part == file_part)
}

fn windows_final_components(path: &[u16]) -> Option<Vec<Vec<u16>>> {
    if path.is_empty() || path.contains(&0) {
        return None;
    }
    let mut parts = Vec::new();
    let rest = if is_extended_prefix(path) {
        let body = &path[4..];
        if starts_with_ascii(body, b"UNC\\") || starts_with_ascii(body, b"UNC/") {
            let after = &body[4..];
            let (server, after) = take_component(after)?;
            let (share, after) = take_component(after)?;
            if server.is_empty() || share.is_empty() {
                return None;
            }
            parts.push(utf16(r"\\?\UNC"));
            parts.push(server.to_vec());
            parts.push(share.to_vec());
            after
        } else if is_drive_prefix(body) {
            let mut drive = utf16(r"\\?\");
            drive.extend_from_slice(&body[..2]);
            parts.push(drive);
            strip_sep(&body[2..])
        } else {
            return None;
        }
    } else if is_unc_prefix(path) {
        let after = &path[2..];
        let (server, after) = take_component(after)?;
        let (share, after) = take_component(after)?;
        if server.is_empty() || share.is_empty() {
            return None;
        }
        let mut unc = vec![b'\\' as u16, b'\\' as u16];
        unc.extend_from_slice(server);
        parts.push(unc);
        parts.push(share.to_vec());
        after
    } else if is_drive_prefix(path) {
        parts.push(path[..2].to_vec());
        strip_sep(&path[2..])
    } else {
        path
    };
    push_relative(&mut parts, rest)?;
    if parts.is_empty() {
        return None;
    }
    if parts.iter().any(|part| component_has_stream(part)) {
        return None;
    }
    Some(parts)
}

fn push_relative(parts: &mut Vec<Vec<u16>>, rest: &[u16]) -> Option<()> {
    if rest.is_empty() {
        return Some(());
    }
    let mut start = 0;
    for (index, unit) in rest.iter().enumerate() {
        if is_sep(*unit) {
            push_component(parts, &rest[start..index])?;
            start = index + 1;
        }
    }
    push_component(parts, &rest[start..])?;
    Some(())
}

fn push_component(parts: &mut Vec<Vec<u16>>, component: &[u16]) -> Option<()> {
    if component.is_empty() || component == [b'.' as u16] || component == [b'.' as u16, b'.' as u16]
    {
        return None;
    }
    parts.push(component.to_vec());
    Some(())
}

fn is_sep(unit: u16) -> bool {
    unit == b'\\' as u16 || unit == b'/' as u16
}

fn is_extended_prefix(path: &[u16]) -> bool {
    path.len() >= 4
        && is_sep(path[0])
        && path[1] == path[0]
        && path[2] == b'?' as u16
        && is_sep(path[3])
}

fn is_unc_prefix(path: &[u16]) -> bool {
    path.len() >= 2 && is_sep(path[0]) && path[1] == path[0]
}

fn is_drive_prefix(path: &[u16]) -> bool {
    path.len() >= 2
        && u8::try_from(path[0]).is_ok_and(|byte| byte.is_ascii_alphabetic())
        && path[1] == b':' as u16
}

fn component_has_stream(part: &[u16]) -> bool {
    let prefix = [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
    let tail = part.strip_prefix(&prefix[..]).unwrap_or(part);
    if tail.len() == 2
        && u8::try_from(tail[0]).is_ok_and(|byte| byte.is_ascii_alphabetic())
        && tail[1] == b':' as u16
    {
        return false;
    }
    part.contains(&(b':' as u16))
}

fn starts_with_ascii(text: &[u16], prefix: &[u8]) -> bool {
    text.len() >= prefix.len()
        && text[..prefix.len()]
            .iter()
            .zip(prefix)
            .all(|(unit, expect)| *unit == u16::from(*expect))
}

fn take_component(path: &[u16]) -> Option<(&[u16], &[u16])> {
    if path.is_empty() {
        return None;
    }
    let end = path
        .iter()
        .position(|unit| is_sep(*unit))
        .unwrap_or(path.len());
    let component = &path[..end];
    if component.is_empty() || component == [b'.' as u16] || component == [b'.' as u16, b'.' as u16]
    {
        return None;
    }
    let rest = if end == path.len() {
        &[]
    } else {
        &path[end + 1..]
    };
    Some((component, rest))
}

fn strip_sep(path: &[u16]) -> &[u16] {
    if path.first().is_some_and(|unit| is_sep(*unit)) {
        &path[1..]
    } else {
        path
    }
}

fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opened_path_must_stay_under_root() {
        assert!(path_is_under(Path::new("jail"), Path::new("jail/a.bin")));
        assert!(path_is_under(Path::new("Jail"), Path::new("Jail/A.bin")));
        assert!(!path_is_under(Path::new("Jail"), Path::new("jail/A.bin")));
        assert!(!path_is_under(Path::new("jail"), Path::new("jail2/a.bin")));
        assert!(!path_is_under(Path::new("jail"), Path::new("jail")));
        assert!(!path_is_under(
            Path::new("jail/sub"),
            Path::new("jail/a.bin")
        ));
    }

    #[test]
    fn final_path_comparison_rejects_outside_prefixes() {
        assert!(windows_final_path_is_under(
            r"\\?\C:\jail",
            r"\\?\C:\jail\a.bin"
        ));
        assert!(windows_final_path_is_under(
            r"\\?\C:\Jail",
            r"\\?\C:\Jail\A.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\Jail",
            r"\\?\C:\jail\A.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\Jail",
            r"\\?\c:\Jail\A.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\jail",
            r"\\?\C:\other\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\jail",
            r"\\?\C:\jail2\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\jail",
            r"\\?\D:\jail\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\jail\sub",
            r"\\?\C:\jail\a.bin"
        ));
        assert!(!windows_final_path_is_under(r"\\?\C:\jail", r"\\?\C:\jail"));
        assert!(windows_final_path_is_under(
            r"\\?\UNC\server\share\jail",
            r"\\?\UNC\server\share\jail\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\jail",
            r"\\?\UNC\server\share\jail\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\UNC\server\share\jail",
            r"\\?\UNC\server\share2\jail\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\UNC\server\share\jail",
            r"\\server\share\jail\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\jail",
            r"C:\jail\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"C:\jail",
            r"\\?\C:\jail\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\jail",
            r"\\?\C:\jail\..\windows\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\Program Files\jail",
            r"\\?\C:\PROGRA~1\jail\a.bin"
        ));
        assert!(windows_final_path_is_under(
            r"\\?\C:\PROGRA~1\jail",
            r"\\?\C:\PROGRA~1\jail\a.bin"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\jail",
            r"\\?\C:\jail\a.bin:stream"
        ));
        assert!(!windows_final_path_is_under(
            r"\\?\C:\jail",
            r"\\?\C:\jail\a.bin:stream:$DATA"
        ));
    }

    #[test]
    fn parent_component_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("box.bin"), b"box!").expect("bin");
        let err = read_under(
            dir.path(),
            Path::new("../box.bin"),
            64,
            "",
            Path::new("../box.bin"),
        )
        .expect_err("parent");
        let msg = format!("{err:#}");
        assert!(msg.contains("unsupported"), "{msg}");
    }

    #[cfg(unix)]
    #[test]
    fn linked_intermediate_directory_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("loot.bin"), b"loot").expect("loot");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("sub")).expect("link");
        let err = read_under(
            dir.path(),
            Path::new("sub/loot.bin"),
            64,
            "",
            Path::new("sub/loot.bin"),
        )
        .expect_err("link");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_root_loads() {
        let real = tempfile::tempdir().expect("real");
        std::fs::write(real.path().join("box.bin"), b"box!").expect("bin");
        let parent = tempfile::tempdir().expect("parent");
        let link = parent.path().join("models");
        std::os::unix::fs::symlink(real.path(), &link).expect("symlink");
        let bytes = read_under(&link, Path::new("box.bin"), 64, "", Path::new("box.bin"))
            .expect("linked root");
        assert_eq!(bytes, b"box!");
        let via_path = read_path(&link.join("box.bin"), 64, "").expect("linked parent");
        assert_eq!(via_path, b"box!");
    }

    #[cfg(windows)]
    fn junction(link: &Path, target: &Path) {
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .expect("mklink");
        assert!(status.success(), "mklink");
    }

    #[cfg(windows)]
    #[test]
    fn windows_junction_root_loads() {
        let real = tempfile::tempdir().expect("real");
        std::fs::write(real.path().join("box.bin"), b"box!").expect("bin");
        let parent = tempfile::tempdir().expect("parent");
        let link = parent.path().join("models");
        junction(&link, real.path());
        let bytes = read_under(&link, Path::new("box.bin"), 64, "", Path::new("box.bin"))
            .expect("junction root");
        assert_eq!(bytes, b"box!");
        let via_path = read_path(&link.join("box.bin"), 64, "").expect("junction parent");
        assert_eq!(via_path, b"box!");
    }

    #[cfg(windows)]
    #[test]
    fn windows_linked_directory_is_rejected() {
        let root = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("note.bin"), b"outside").expect("note");
        let sub = root.path().join("sub");
        junction(&sub, outside.path());
        let err = read_under(
            root.path(),
            Path::new("sub/note.bin"),
            64,
            "",
            Path::new("sub/note.bin"),
        )
        .expect_err("link");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn windows_linked_file_is_rejected() {
        let root = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        let real = outside.path().join("real.bin");
        std::fs::write(&real, b"real").expect("real");
        let link = root.path().join("link.bin");
        std::os::windows::fs::symlink_file(&real, &link).expect("link");
        let err = read_under(
            root.path(),
            Path::new("link.bin"),
            64,
            "",
            Path::new("link.bin"),
        )
        .expect_err("link");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[test]
    fn unpaired_surrogates_compare_as_utf16_units() {
        let mut root_a = utf16(r"\\?\C:\");
        root_a.push(0xD800);
        let mut root_b = utf16(r"\\?\C:\");
        root_b.push(0xD801);
        let mut file_a = root_a.clone();
        file_a.extend(utf16(r"\a.bin"));
        let mut file_b = root_b.clone();
        file_b.extend(utf16(r"\a.bin"));
        assert!(windows_final_path_is_under_units(&root_a, &file_a));
        assert!(!windows_final_path_is_under_units(&root_a, &file_b));
        // `from_utf16_lossy` folds both names onto one string. The compare must not.
        assert_eq!(
            String::from_utf16_lossy(&root_a),
            String::from_utf16_lossy(&root_b)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_model_file_loads_and_a_uri_link_does_not() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real = dir.path().join("real.gltf");
        std::fs::write(&real, b"gltf").expect("write");
        let link = dir.path().join("model.gltf");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");
        let bytes = read_path(&link, 64, "").expect("model link");
        assert_eq!(bytes, b"gltf");
        let container = crate::gltf_loader::read_gltf_container(&link).expect("container");
        assert_eq!(container, b"gltf");

        let bin = dir.path().join("real.bin");
        std::fs::write(&bin, b"bin!").expect("bin");
        let bin_link = dir.path().join("link.bin");
        std::os::unix::fs::symlink(&bin, &bin_link).expect("bin link");
        let err = read_unresolved(&bin_link, 64, "").expect_err("uri link");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn path_is_under_keeps_unpaired_surrogates() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        let mut root_a = utf16(r"\\?\C:\");
        root_a.push(0xD800);
        let mut root_b = utf16(r"\\?\C:\");
        root_b.push(0xD801);
        let mut file_a = root_a.clone();
        file_a.extend(utf16(r"\a.bin"));
        let mut file_b = root_b.clone();
        file_b.extend(utf16(r"\a.bin"));
        let root = std::path::PathBuf::from(OsString::from_wide(&root_a));
        let same = std::path::PathBuf::from(OsString::from_wide(&file_a));
        let other = std::path::PathBuf::from(OsString::from_wide(&file_b));
        assert!(path_is_under(&root, &same));
        assert!(!path_is_under(&root, &other));
    }

    #[cfg(windows)]
    #[test]
    fn final_path_outside_the_root_is_refused() {
        struct ClearOverride;
        impl Drop for ClearOverride {
            fn drop(&mut self) {
                super::FILE_FINAL_OVERRIDE.with(|cell| cell.take());
            }
        }
        let _guard = ClearOverride;
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::write(root.path().join("box.bin"), b"box!").expect("bin");
        super::FILE_FINAL_OVERRIDE.with(|cell| {
            cell.set(Some(std::path::PathBuf::from(r"\\?\D:\outside\box.bin")));
        });
        let err = read_under(
            root.path(),
            Path::new("box.bin"),
            64,
            "",
            Path::new("box.bin"),
        )
        .expect_err("outside final path");
        assert!(err.to_string().contains("not a regular file"), "{err}");
        let bytes = read_under(
            root.path(),
            Path::new("box.bin"),
            64,
            "",
            Path::new("box.bin"),
        )
        .expect("real final path");
        assert_eq!(bytes, b"box!");
    }

    #[cfg(windows)]
    #[test]
    fn windows_parent_junction_loads() {
        let real = tempfile::tempdir().expect("real");
        let models = real.path().join("models");
        std::fs::create_dir(&models).expect("models");
        std::fs::write(models.join("box.bin"), b"box!").expect("bin");
        let parent = tempfile::tempdir().expect("parent");
        let link = parent.path().join("linked");
        junction(&link, real.path());
        let model_dir = link.join("models");
        let bytes = read_under(
            &model_dir,
            Path::new("box.bin"),
            64,
            "",
            Path::new("box.bin"),
        )
        .expect("parent junction");
        assert_eq!(bytes, b"box!");
        let via_path = read_path(&model_dir.join("box.bin"), 64, "").expect("parent junction file");
        assert_eq!(via_path, b"box!");
    }

    #[cfg(windows)]
    #[test]
    fn windows_symlinked_model_file_loads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real = dir.path().join("real.gltf");
        std::fs::write(&real, b"gltf").expect("write");
        let link = dir.path().join("model.gltf");
        std::os::windows::fs::symlink_file(&real, &link).expect("symlink");
        let bytes = read_path(&link, 64, "").expect("model link");
        assert_eq!(bytes, b"gltf");
        let container = crate::gltf_loader::read_gltf_container(&link).expect("container");
        assert_eq!(container, b"gltf");
    }
}
