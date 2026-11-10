//! Size-bounded read from a directory handle.
//!
//! On Unix each component is opened from the parent directory handle.
//! On Windows each directory handle stays open until the file is opened, and
//! the opened file stays under the canonical root. File type and size come
//! from the opened handle, and the read stops at the caller's byte cap.

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

pub(crate) fn read_path(path: &Path, max_bytes: u64, noun: &str) -> Result<Vec<u8>> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let name = path
        .file_name()
        .with_context(|| format!("open {}", subject(noun, path)))?;
    read_under(parent, Path::new(name), max_bytes, noun, path)
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

    let directory = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file_flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
    let names = relative_names(relative)?;
    let mut dir = opened(openat(CWD, root, directory, Mode::empty()))?;
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

#[cfg(windows)]
fn open_under(root: &Path, relative: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    fn open_handle(path: &Path) -> std::io::Result<File> {
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
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
    let root_file = open_handle(root)?;
    ensure_directory(&root_file)?;
    let root_final = final_path(&root_file)?;
    let mut held = vec![root_file];

    let mut cursor = root.to_path_buf();
    for (index, name) in names.iter().enumerate() {
        cursor.push(name);
        let file = open_handle(&cursor)?;
        let last = index + 1 == names.len();
        if !last {
            ensure_directory(&file)?;
            held.push(file);
            continue;
        }
        let file_final = final_path(&file)?;
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
    let root_parts: Vec<std::ffi::OsString> = root
        .components()
        .map(|component| component.as_os_str().to_os_string())
        .collect();
    let file_parts: Vec<std::ffi::OsString> = file
        .components()
        .map(|component| component.as_os_str().to_os_string())
        .collect();
    if file_parts.len() <= root_parts.len() {
        return false;
    }
    root_parts
        .iter()
        .zip(&file_parts)
        .all(|(root_part, file_part)| root_part.eq_ignore_ascii_case(file_part))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opened_path_must_stay_under_root() {
        assert!(path_is_under(Path::new("jail"), Path::new("jail/a.bin")));
        assert!(path_is_under(Path::new("Jail"), Path::new("jail/A.bin")));
        assert!(!path_is_under(Path::new("jail"), Path::new("jail2/a.bin")));
        assert!(!path_is_under(Path::new("jail"), Path::new("jail")));
        assert!(!path_is_under(
            Path::new("jail/sub"),
            Path::new("jail/a.bin")
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

    #[cfg(windows)]
    #[test]
    fn windows_linked_directory_is_rejected() {
        let root = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("note.bin"), b"outside").expect("note");
        let sub = root.path().join("sub");
        std::fs::create_dir(&sub).expect("sub");
        std::fs::remove_dir(&sub).expect("remove");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&sub)
            .arg(outside.path())
            .status()
            .expect("mklink");
        assert!(status.success(), "mklink");
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
}
