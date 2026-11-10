//! Size-bounded read from a directory handle.
//!
//! The directory is opened first. Each component under it is opened from that
//! handle. File type and size come from the opened handle, and the read stops
//! at the caller's byte cap.

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
        Err(rustix::io::Errno::LOOP) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        )),
        Err(err) => Err(err.into()),
    }
}

#[cfg(windows)]
fn open_under(root: &Path, relative: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    fn open_handle(path: &Path) -> std::io::Result<File> {
        std::fs::OpenOptions::new()
            .read(true)
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
    drop(root_file);

    let mut cursor = root.to_path_buf();
    let mut opened = None;
    for (index, name) in names.iter().enumerate() {
        cursor.push(name);
        let file = open_handle(&cursor)?;
        if index + 1 == names.len() {
            opened = Some(file);
            break;
        }
        ensure_directory(&file)?;
    }
    opened
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty relative path"))
}
