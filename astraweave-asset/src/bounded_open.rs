//! Size-bounded read from a directory handle.
//!
//! One load opens the model directory once, and that handle is used for the
//! container read, every sibling URI, and every image URI. On Unix the handle
//! is a directory file descriptor and each URI component is opened from it
//! with `O_NOFOLLOW`. On Windows the handle is the directory-handle chain plus
//! the final path. Each URI component is reopened from that final path. A
//! cloud-file reparse tag is allowed. A symlink, a junction, and a mount point
//! are refused. The file's final path must stay under the root's. Those paths
//! compare as exact UTF-16 units.
//!
//! A caller-supplied model file is resolved once. Its parent is then opened
//! with that component walk and is not resolved again. URI components are not
//! resolved. A file with more than one link is refused, and the error names
//! the file. File type and size come from the opened handle, and the read
//! stops at the caller's byte cap. A hard-linked model is deliberately refused.

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::Read;
#[cfg(windows)]
use std::path::PathBuf;
use std::path::{Component, Path};

/// The model directory opened once for a load.
///
/// One load opens the model directory once, and that handle is used for the
/// container read, every sibling URI, and every image URI.
#[derive(Debug)]
pub(crate) struct OpenedRoot {
    #[cfg(unix)]
    dir: rustix::fd::OwnedFd,
    #[cfg(windows)]
    root_final: PathBuf,
    #[cfg(windows)]
    _held: Vec<File>,
}

impl OpenedRoot {
    /// Resolve `root` once, then open that absolute path without following it.
    pub(crate) fn open(root: &Path) -> std::io::Result<Self> {
        let resolved = std::fs::canonicalize(root)?;
        after_resolve();
        Self::open_resolved(&resolved)
    }

    pub(crate) fn read(
        &self,
        relative: &Path,
        max_bytes: u64,
        noun: &str,
        display: &Path,
    ) -> Result<Vec<u8>> {
        read_opened(self, relative, max_bytes, noun, display)
    }
}

pub(crate) fn read_path(path: &Path, max_bytes: u64, noun: &str) -> Result<(Vec<u8>, OpenedRoot)> {
    // Resolve the caller-supplied file once. `canonicalize` does not open the
    // leaf, so a FIFO returns without a blocking read. The parent is opened
    // from that resolved path and is not resolved again.
    let resolved =
        std::fs::canonicalize(path).with_context(|| format!("open {}", subject(noun, path)))?;
    after_resolve();
    let parent = directory_of(&resolved);
    let name = resolved
        .file_name()
        .with_context(|| format!("open {}", subject(noun, path)))?;
    let root = match OpenedRoot::open_resolved(parent) {
        Ok(root) => root,
        Err(err) if not_regular_open(&err) => {
            bail!("{} is not a regular file", subject(noun, path));
        }
        Err(err) => {
            return Err(err).with_context(|| format!("open {}", subject(noun, path)));
        }
    };
    let bytes = root.read(Path::new(name), max_bytes, noun, path)?;
    Ok((bytes, root))
}

/// Open `path` without resolving any component.
///
/// A relative parent is made absolute lexically. That walk does not follow a
/// link. An absolute path is left unchanged, so a `..` component still fails
/// the component walk. The parent is opened component by component. A link in
/// a parent directory is refused, and a link at the file is refused.
pub(crate) fn read_unresolved(path: &Path, max_bytes: u64, noun: &str) -> Result<Vec<u8>> {
    let parent = directory_of(path);
    let parent =
        lexical_absolute(parent).with_context(|| format!("open {}", subject(noun, path)))?;
    let name = path
        .file_name()
        .with_context(|| format!("open {}", subject(noun, path)))?;
    let root = match OpenedRoot::open_resolved(&parent) {
        Ok(root) => root,
        Err(err) if not_regular_open(&err) => {
            bail!("{} is not a regular file", subject(noun, path));
        }
        Err(err) => {
            return Err(err).with_context(|| format!("open {}", subject(noun, path)));
        }
    };
    root.read(Path::new(name), max_bytes, noun, path)
}

/// Absolute paths stay as given. A relative or empty path is joined to the
/// current directory and `.` / `..` are removed without touching the filesystem.
fn lexical_absolute(path: &Path) -> std::io::Result<std::path::PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let cwd = std::env::current_dir()?;
    if path.as_os_str().is_empty() || path == Path::new(".") {
        return Ok(cwd);
    }
    Ok(normalize_lexical(&cwd.join(path)))
}

fn normalize_lexical(path: &Path) -> std::path::PathBuf {
    let mut out = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn directory_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn read_under(
    root: &Path,
    relative: &Path,
    max_bytes: u64,
    noun: &str,
    display: &Path,
) -> Result<Vec<u8>> {
    let opened = match OpenedRoot::open(root) {
        Ok(opened) => opened,
        Err(err) if not_regular_open(&err) => {
            bail!("{} is not a regular file", subject(noun, display));
        }
        Err(err) => {
            return Err(err).with_context(|| format!("open {}", subject(noun, display)));
        }
    };
    opened.read(relative, max_bytes, noun, display)
}

fn read_opened(
    root: &OpenedRoot,
    relative: &Path,
    max_bytes: u64,
    noun: &str,
    display: &Path,
) -> Result<Vec<u8>> {
    let mut file = match open_relative(root, relative) {
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
    if !regular_file(&file, &meta) {
        bail!("{} is not a regular file", subject(noun, display));
    }
    let links = link_count(&file).with_context(|| format!("stat {}", subject(noun, display)))?;
    if links > 1 {
        bail!("{} has more than one link", subject(noun, display));
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

fn regular_file(file: &File, meta: &std::fs::Metadata) -> bool {
    if reparse_point(file, meta) {
        return false;
    }
    meta.is_file()
}

/// `IO_REPARSE_TAG_CLOUD` through `IO_REPARSE_TAG_CLOUD_F`.
///
/// A symlink, a junction, and a mount point use other tags and stay refused.
#[cfg_attr(not(windows), allow(dead_code))]
fn cloud_file_reparse_tag(tag: u32) -> bool {
    const CLOUD: u32 = 0x9000_001A;
    const CLOUD_F: u32 = 0x9000_F01A;
    const STEP: u32 = 0x0000_1000;
    (CLOUD..=CLOUD_F).contains(&tag) && (tag - CLOUD) % STEP == 0
}

/// A reparse point is refused unless its tag is a cloud-file placeholder.
#[cfg_attr(not(windows), allow(dead_code))]
fn reparse_is_refused(attributes: u32, tag: u32) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        return false;
    }
    !cloud_file_reparse_tag(tag)
}

#[cfg(windows)]
fn reparse_point(file: &File, meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    #[cfg(test)]
    if meta.is_dir() && FORCE_DIRECTORY_REPARSE.with(|cell| cell.replace(false)) {
        return true;
    }
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    let attributes = {
        let base = meta.file_attributes();
        #[cfg(test)]
        {
            if !meta.is_dir() && FORCE_FILE_REPARSE.with(|cell| cell.replace(false)) {
                base | FILE_ATTRIBUTE_REPARSE_POINT
            } else {
                base
            }
        }
        #[cfg(not(test))]
        {
            base
        }
    };
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        return false;
    }
    match windows_reparse_tag(file) {
        Ok(tag) => reparse_is_refused(attributes, tag),
        Err(_) => true,
    }
}

#[cfg(not(windows))]
fn reparse_point(_file: &File, _meta: &std::fs::Metadata) -> bool {
    false
}

fn link_count(file: &File) -> std::io::Result<u64> {
    #[cfg(unix)]
    {
        Ok(std::os::unix::fs::MetadataExt::nlink(&file.metadata()?))
    }
    #[cfg(windows)]
    {
        windows_link_count(file)
    }
}

#[cfg(test)]
thread_local! {
    static AFTER_RESOLVE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
}

fn after_resolve() {
    #[cfg(test)]
    if let Some(hook) = AFTER_RESOLVE.with(|cell| cell.borrow_mut().take()) {
        hook();
    }
}

#[cfg(test)]
thread_local! {
    static AFTER_ROOT_OPEN: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
    static ROOT_OPENS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

fn record_root_open() {
    #[cfg(test)]
    {
        ROOT_OPENS.with(|cell| cell.set(cell.get().saturating_add(1)));
        if let Some(hook) = AFTER_ROOT_OPEN.with(|cell| cell.borrow_mut().take()) {
            hook();
        }
    }
}

#[cfg(test)]
pub(crate) fn reset_root_opens() {
    ROOT_OPENS.with(|cell| cell.set(0));
    AFTER_ROOT_OPEN.with(|cell| *cell.borrow_mut() = None);
}

#[cfg(test)]
pub(crate) fn root_open_count() -> u32 {
    ROOT_OPENS.with(|cell| cell.get())
}

#[cfg(all(test, unix))]
pub(crate) fn after_root_open(hook: Box<dyn FnOnce()>) {
    AFTER_ROOT_OPEN.with(|cell| *cell.borrow_mut() = Some(hook));
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
fn directory_flags() -> rustix::fs::OFlags {
    use rustix::fs::OFlags;
    // Linux opens each directory component with O_PATH so a load still works
    // when an ancestor is not readable. Other Unix keeps a read-only directory
    // open, which is the portable equivalent.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
    }
}

#[cfg(unix)]
impl OpenedRoot {
    fn open_resolved(absolute: &Path) -> std::io::Result<Self> {
        let flags = directory_flags();
        let root = Self {
            dir: open_canonical(absolute, flags)?,
        };
        record_root_open();
        Ok(root)
    }
}

#[cfg(unix)]
fn open_relative(root: &OpenedRoot, relative: &Path) -> std::io::Result<File> {
    use rustix::fs::{openat, Mode, OFlags};

    let directory = directory_flags();
    let file_flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
    let names = relative_names(relative)?;
    let mut dir = root.dir.try_clone()?;
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

#[cfg(unix)]
fn open_canonical(
    absolute: &Path,
    flags: rustix::fs::OFlags,
) -> std::io::Result<rustix::fd::OwnedFd> {
    use rustix::fs::{openat, Mode, OFlags, CWD};

    let mut components = absolute.components();
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    let root_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
    let mut dir = opened(openat(CWD, "/", root_flags, Mode::empty()))?;
    for component in components {
        match component {
            Component::Normal(name) => {
                dir = opened(openat(&dir, name, flags, Mode::empty()))?;
            }
            Component::CurDir => {}
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "not a regular file",
                ));
            }
        }
    }
    Ok(dir)
}

#[cfg(all(windows, test))]
thread_local! {
    static FILE_FINAL_OVERRIDE: std::cell::Cell<Option<PathBuf>> = const { std::cell::Cell::new(None) };
    static COMPONENT_FINAL_OVERRIDE: std::cell::Cell<Option<PathBuf>> =
        const { std::cell::Cell::new(None) };
    static FORCE_DIRECTORY_REPARSE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FORCE_FILE_REPARSE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FORCE_CLOUD_REOPEN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static AFTER_LEAF_OPEN: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
    static TAG_RESULT: std::cell::RefCell<Option<std::io::Result<u32>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(windows)]
fn open_path(path: &Path, reparse: bool) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    // Share delete so a name can be replaced while this handle is held.
    // The file-id compare then refuses a cloud file replaced between the two opens.
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if reparse {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(flags)
        .open(path)
}

#[cfg(windows)]
fn ensure_directory(file: &File) -> std::io::Result<()> {
    let meta = file.metadata()?;
    if reparse_point(file, &meta) || !meta.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    Ok(())
}

#[cfg(windows)]
impl OpenedRoot {
    fn open_resolved(absolute: &Path) -> std::io::Result<Self> {
        let (root_final, held) = open_resolved_directory(absolute)?;
        let root = Self {
            root_final,
            _held: held,
        };
        record_root_open();
        Ok(root)
    }
}

#[cfg(windows)]
fn open_resolved_directory(resolved: &Path) -> std::io::Result<(PathBuf, Vec<File>)> {
    let mut cursor = PathBuf::new();
    let mut parent_final: Option<PathBuf> = None;
    let mut held = Vec::new();
    for component in resolved.components() {
        let open_this = matches!(component, Component::RootDir | Component::Normal(_));
        cursor.push(component.as_os_str());
        if !open_this {
            continue;
        }
        let file = open_path(&cursor, true)?;
        ensure_directory(&file)?;
        let component_final = final_path(&file)?;
        #[cfg(test)]
        let component_final = if parent_final.is_some() {
            COMPONENT_FINAL_OVERRIDE
                .with(|cell| cell.take())
                .unwrap_or(component_final)
        } else {
            component_final
        };
        if let Some(parent) = &parent_final {
            if !path_is_under(parent, &component_final) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "not a regular file",
                ));
            }
        }
        parent_final = Some(component_final);
        held.push(file);
    }
    match parent_final {
        Some(root_final) => Ok((root_final, held)),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        )),
    }
}

#[cfg(windows)]
fn open_relative(root: &OpenedRoot, relative: &Path) -> std::io::Result<File> {
    let names = relative_names(relative)?;
    let mut cursor = root.root_final.clone();
    let mut held = Vec::new();
    for (index, name) in names.iter().enumerate() {
        cursor.push(name);
        let last = index + 1 == names.len();
        if !last {
            let file = open_path(&cursor, true)?;
            ensure_directory(&file)?;
            held.push(file);
            continue;
        }
        let file = open_leaf_file(&cursor)?;
        let file_final = final_path(&file)?;
        #[cfg(test)]
        let file_final = FILE_FINAL_OVERRIDE
            .with(|cell| cell.take())
            .unwrap_or(file_final);
        if !path_is_under(&root.root_final, &file_final) {
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
fn open_leaf_file(path: &Path) -> std::io::Result<File> {
    let first = open_path(path, true)?;
    let meta = first.metadata()?;
    let cloud = cloud_placeholder(&first, &meta)?;
    #[cfg(test)]
    let reopen = cloud || FORCE_CLOUD_REOPEN.with(|cell| cell.replace(false));
    #[cfg(not(test))]
    let reopen = cloud;
    if !reopen {
        return Ok(first);
    }
    #[cfg(test)]
    if let Some(hook) = AFTER_LEAF_OPEN.with(|cell| cell.borrow_mut().take()) {
        hook();
    }
    let second = open_path(path, false)?;
    // The second open must be the same file. A cloud placeholder replaced
    // between the two opens is refused.
    if windows_file_id(&first)? != windows_file_id(&second)? {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    Ok(second)
}

#[cfg(windows)]
fn cloud_placeholder(file: &File, meta: &std::fs::Metadata) -> std::io::Result<bool> {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        return Ok(false);
    }
    Ok(cloud_file_reparse_tag(windows_reparse_tag(file)?))
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
        // SAFETY: `handle` is the live file handle. `buf` is writable for
        // `buf.len()` UTF-16 units, and that length is the size passed in.
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

#[cfg(windows)]
struct WindowsFileInfo {
    links: u64,
    volume: u32,
    index_high: u32,
    index_low: u32,
}

#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "GetFileInformationByHandle has no safe wrapper in this crate"
)]
fn windows_by_handle(file: &File) -> std::io::Result<WindowsFileInfo> {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;

    #[repr(C)]
    struct ByHandleFileInformation {
        file_attributes: u32,
        creation_low: u32,
        creation_high: u32,
        last_access_low: u32,
        last_access_high: u32,
        last_write_low: u32,
        last_write_high: u32,
        volume_serial_number: u32,
        file_size_high: u32,
        file_size_low: u32,
        number_of_links: u32,
        file_index_high: u32,
        file_index_low: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandle(
            file: *mut c_void,
            information: *mut ByHandleFileInformation,
        ) -> i32;
    }

    let mut info = ByHandleFileInformation {
        file_attributes: 0,
        creation_low: 0,
        creation_high: 0,
        last_access_low: 0,
        last_access_high: 0,
        last_write_low: 0,
        last_write_high: 0,
        volume_serial_number: 0,
        file_size_high: 0,
        file_size_low: 0,
        number_of_links: 0,
        file_index_high: 0,
        file_index_low: 0,
    };
    // SAFETY: `file` is the live handle. `info` is writable for one
    // `ByHandleFileInformation`, and the call writes that struct.
    let ok = unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(WindowsFileInfo {
        links: u64::from(info.number_of_links),
        volume: info.volume_serial_number,
        index_high: info.file_index_high,
        index_low: info.file_index_low,
    })
}

#[cfg(windows)]
fn windows_link_count(file: &File) -> std::io::Result<u64> {
    Ok(windows_by_handle(file)?.links)
}

#[cfg(windows)]
fn windows_file_id(file: &File) -> std::io::Result<(u32, u32, u32)> {
    let info = windows_by_handle(file)?;
    Ok((info.volume, info.index_high, info.index_low))
}

#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "GetFileInformationByHandleEx has no safe wrapper in this crate"
)]
fn windows_reparse_tag(file: &File) -> std::io::Result<u32> {
    #[cfg(test)]
    if let Some(preset) = TAG_RESULT.with(|cell| cell.borrow_mut().take()) {
        return preset;
    }
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;

    #[repr(C)]
    struct FileAttributeTagInfo {
        file_attributes: u32,
        reparse_tag: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandleEx(
            file: *mut c_void,
            class: i32,
            info: *mut c_void,
            size: u32,
        ) -> i32;
    }

    // `FileAttributeTagInfo` in `FILE_INFO_BY_HANDLE_CLASS`.
    const FILE_ATTRIBUTE_TAG_INFO: i32 = 9;
    let mut info = FileAttributeTagInfo {
        file_attributes: 0,
        reparse_tag: 0,
    };
    // SAFETY: `file` is the live handle. `info` is writable for one
    // `FileAttributeTagInfo`, and that size is the size passed in.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FILE_ATTRIBUTE_TAG_INFO,
            &mut info as *mut FileAttributeTagInfo as *mut c_void,
            std::mem::size_of::<FileAttributeTagInfo>() as u32,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(info.reparse_tag)
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
        // This arm is compiled only off Windows. Production final-path
        // compares use `OsStrExt::encode_wide` on Windows. A non-Unicode
        // test path fails closed (an empty unit list) instead of substituting
        // U+FFFD. A lossy conversion here would not change the Windows compare.
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
            // `take_component` already returns None for an empty component, so
            // an empty server or share does not reach this check. It stays so
            // a later change to `take_component` cannot accept that name.
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
        // Same as the extended-UNC check: `take_component` already refuses an
        // empty component, and this check stays as the second refusal.
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

    fn read_path_bytes(path: &Path, max_bytes: u64, noun: &str) -> Result<Vec<u8>> {
        read_path(path, max_bytes, noun).map(|(bytes, _root)| bytes)
    }

    #[test]
    fn absolute_component_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("secret"), b"nope").expect("write");
        let err = read_under(
            dir.path(),
            Path::new("/secret"),
            64,
            "",
            Path::new("/secret"),
        )
        .expect_err("root");
        let msg = format!("{err:#}");
        assert!(msg.contains("unsupported path component"), "{msg}");
    }

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
        let via_path = read_path_bytes(&link.join("box.bin"), 64, "").expect("linked parent");
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
        let via_path = read_path_bytes(&link.join("box.bin"), 64, "").expect("junction parent");
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
        let bytes = read_path_bytes(&link, 64, "").expect("model link");
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
        let via_path =
            read_path_bytes(&model_dir.join("box.bin"), 64, "").expect("parent junction file");
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
        let bytes = read_path_bytes(&link, 64, "").expect("model link");
        assert_eq!(bytes, b"gltf");
        let container = crate::gltf_loader::read_gltf_container(&link).expect("container");
        assert_eq!(container, b"gltf");
    }

    struct ClearResolveHook;
    impl Drop for ClearResolveHook {
        fn drop(&mut self) {
            AFTER_RESOLVE.with(|cell| {
                cell.borrow_mut().take();
            });
        }
    }

    #[cfg(unix)]
    #[test]
    fn resolved_directory_replaced_before_open_is_refused() {
        let _guard = ClearResolveHook;
        let parent = tempfile::tempdir().expect("parent");
        let jail = parent.path().join("jail");
        std::fs::create_dir(&jail).expect("jail");
        std::fs::write(jail.join("box.bin"), b"box!").expect("bin");
        let outside = parent.path().join("outside");
        std::fs::create_dir(&outside).expect("outside");
        std::fs::write(outside.join("box.bin"), b"out!").expect("out");
        let jail_hook = jail.clone();
        let outside_hook = outside.clone();
        let parked = parent.path().join("parked");
        AFTER_RESOLVE.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(move || {
                std::fs::rename(&jail_hook, &parked).expect("park");
                std::os::unix::fs::symlink(&outside_hook, &jail_hook).expect("swap");
            }));
        });
        let err = read_under(&jail, Path::new("box.bin"), 64, "", Path::new("box.bin"))
            .expect_err("replaced directory");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn resolved_directory_replaced_before_open_is_refused() {
        let _guard = ClearResolveHook;
        let parent = tempfile::tempdir().expect("parent");
        let jail = parent.path().join("jail");
        std::fs::create_dir(&jail).expect("jail");
        std::fs::write(jail.join("box.bin"), b"box!").expect("bin");
        let outside = parent.path().join("outside");
        std::fs::create_dir(&outside).expect("outside");
        std::fs::write(outside.join("box.bin"), b"out!").expect("out");
        let jail_hook = jail.clone();
        let outside_hook = outside.clone();
        let parked = parent.path().join("parked");
        AFTER_RESOLVE.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(move || {
                std::fs::rename(&jail_hook, &parked).expect("park");
                junction(&jail_hook, &outside_hook);
            }));
        });
        let err = read_under(&jail, Path::new("box.bin"), 64, "", Path::new("box.bin"))
            .expect_err("replaced directory");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn hard_linked_file_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("box.bin");
        std::fs::write(&file, b"box!").expect("bin");
        std::fs::hard_link(&file, dir.path().join("link.bin")).expect("link");
        let err = read_under(
            dir.path(),
            Path::new("box.bin"),
            64,
            "",
            Path::new("box.bin"),
        )
        .expect_err("link count");
        let msg = format!("{err:#}");
        assert!(msg.contains("box.bin"), "{msg}");
        assert!(msg.contains("more than one link"), "{msg}");
    }

    #[cfg(unix)]
    #[test]
    fn hard_linked_model_file_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model = dir.path().join("model.gltf");
        std::fs::write(&model, b"gltf").expect("write");
        std::fs::hard_link(&model, dir.path().join("other.gltf")).expect("link");
        let err = read_path_bytes(&model, 64, "").expect_err("links");
        let msg = format!("{err:#}");
        assert!(msg.contains("model.gltf"), "{msg}");
        assert!(msg.contains("more than one link"), "{msg}");
    }

    #[cfg(windows)]
    #[test]
    fn hard_linked_file_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("box.bin");
        std::fs::write(&file, b"box!").expect("bin");
        std::fs::hard_link(&file, dir.path().join("link.bin")).expect("link");
        let err = read_under(
            dir.path(),
            Path::new("box.bin"),
            64,
            "",
            Path::new("box.bin"),
        )
        .expect_err("link count");
        let msg = format!("{err:#}");
        assert!(msg.contains("box.bin"), "{msg}");
        assert!(msg.contains("more than one link"), "{msg}");
    }

    #[cfg(windows)]
    #[test]
    fn hard_linked_model_file_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let model = dir.path().join("model.gltf");
        std::fs::write(&model, b"gltf").expect("write");
        std::fs::hard_link(&model, dir.path().join("other.gltf")).expect("link");
        let err = read_path_bytes(&model, 64, "").expect_err("links");
        let msg = format!("{err:#}");
        assert!(msg.contains("model.gltf"), "{msg}");
        assert!(msg.contains("more than one link"), "{msg}");
    }

    #[test]
    fn final_path_parser_refuses_closed_forms() {
        let with_nul = utf16(r"\\?\C:\jail\a.bin")
            .into_iter()
            .map(|unit| if unit == u16::from(b'j') { 0 } else { unit })
            .collect::<Vec<_>>();
        assert!(windows_final_components(&with_nul).is_none());

        assert!(windows_final_components(&utf16(r"\\?\1:\jail\a.bin")).is_none());
        assert!(windows_final_components(&utf16(r"1:\jail\a.bin")).is_none());

        for path in [
            r"\\?\UNC\.\share\jail\a.bin",
            r"\\?\UNC\..\share\jail\a.bin",
            r"\\?\UNC\server\.\jail\a.bin",
            r"\\?\UNC\server\..\jail\a.bin",
            r"\\.\share\jail\a.bin",
            r"\\server\..\jail\a.bin",
        ] {
            assert!(windows_final_components(&utf16(path)).is_none(), "{path}");
        }

        let mixed = utf16(r"\/?\C:\jail\a.bin");
        assert!(windows_final_components(&mixed).is_none());
        assert!(!windows_final_path_is_under_units(
            &utf16(r"\\?\C:\jail"),
            &mixed
        ));
        let other_mixed = utf16(r"/\?\C:\jail\a.bin");
        assert!(windows_final_components(&other_mixed).is_none());
    }

    #[test]
    fn cloud_file_reparse_tags_stay_and_link_tags_do_not() {
        const REPARSE: u32 = 0x0000_0400;
        const SYMLINK: u32 = 0xA000_000C;
        const MOUNT_POINT: u32 = 0xA000_0003;
        assert!(!reparse_is_refused(REPARSE, 0x9000_001A));
        assert!(!reparse_is_refused(REPARSE, 0x9000_101A));
        assert!(!reparse_is_refused(REPARSE, 0x9000_F01A));
        assert!(reparse_is_refused(REPARSE, 0x9000_001C));
        assert!(reparse_is_refused(REPARSE, 0x9001_001A));
        assert!(reparse_is_refused(REPARSE, SYMLINK));
        assert!(reparse_is_refused(REPARSE, MOUNT_POINT));
        assert!(reparse_is_refused(REPARSE, 0x8000_0015));
        assert!(!reparse_is_refused(0, SYMLINK));
    }

    #[test]
    fn drive_prefix_requires_a_letter() {
        assert!(is_drive_prefix(&utf16("C:")));
        assert!(is_drive_prefix(&utf16("c:")));
        assert!(!is_drive_prefix(&utf16("1:")));
        assert!(!is_drive_prefix(&utf16("C")));
        assert!(!is_drive_prefix(&utf16(":")));
    }

    #[cfg(unix)]
    #[test]
    fn model_parent_replaced_before_open_is_refused() {
        let _guard = ClearResolveHook;
        let parent = tempfile::tempdir().expect("parent");
        let jail = parent.path().join("jail");
        std::fs::create_dir(&jail).expect("jail");
        let model = jail.join("model.gltf");
        std::fs::write(&model, b"gltf").expect("write");
        let outside = parent.path().join("outside");
        std::fs::create_dir(&outside).expect("outside");
        std::fs::write(outside.join("model.gltf"), b"evil").expect("evil");
        let jail_hook = jail.clone();
        let outside_hook = outside.clone();
        let parked = parent.path().join("parked");
        AFTER_RESOLVE.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(move || {
                std::fs::rename(&jail_hook, &parked).expect("park");
                std::os::unix::fs::symlink(&outside_hook, &jail_hook).expect("swap");
            }));
        });
        let err = read_path_bytes(&model, 64, "").expect_err("replaced parent");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn resolved_component_reparse_is_refused_on_its_own() {
        struct Clear;
        impl Drop for Clear {
            fn drop(&mut self) {
                super::FORCE_DIRECTORY_REPARSE.with(|cell| cell.set(false));
            }
        }
        let _guard = Clear;
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::write(root.path().join("box.bin"), b"box!").expect("bin");
        super::FORCE_DIRECTORY_REPARSE.with(|cell| cell.set(true));
        let err = read_under(
            root.path(),
            Path::new("box.bin"),
            64,
            "",
            Path::new("box.bin"),
        )
        .expect_err("directory reparse");
        assert!(err.to_string().contains("not a regular file"), "{err}");
        let bytes = read_under(
            root.path(),
            Path::new("box.bin"),
            64,
            "",
            Path::new("box.bin"),
        )
        .expect("real directory");
        assert_eq!(bytes, b"box!");
    }

    #[cfg(windows)]
    #[test]
    fn resolved_component_final_path_is_refused_on_its_own() {
        struct Clear;
        impl Drop for Clear {
            fn drop(&mut self) {
                super::COMPONENT_FINAL_OVERRIDE.with(|cell| cell.take());
            }
        }
        let _guard = Clear;
        let root = tempfile::tempdir().expect("tempdir");
        std::fs::write(root.path().join("box.bin"), b"box!").expect("bin");
        super::COMPONENT_FINAL_OVERRIDE.with(|cell| {
            cell.set(Some(std::path::PathBuf::from(r"\\?\D:\outside\jail")));
        });
        let err = read_under(
            root.path(),
            Path::new("box.bin"),
            64,
            "",
            Path::new("box.bin"),
        )
        .expect_err("component final path");
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

    fn scratch_name(label: &str) -> String {
        format!(
            "aw-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        )
    }

    struct RemovePath(std::path::PathBuf);
    impl Drop for RemovePath {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn unresolved_bare_file_name_reads_from_the_current_directory() {
        let name = scratch_name("bare");
        let path = std::path::PathBuf::from(&name);
        let _guard = RemovePath(path.clone());
        std::fs::write(&path, b"bare").expect("write");
        let bytes = read_unresolved(Path::new(&name), 4, "").expect("bare");
        assert_eq!(bytes, b"bare");
    }

    #[test]
    fn unresolved_relative_subdirectory_is_not_canonicalized() {
        let name = scratch_name("rel");
        let dir = std::path::PathBuf::from("target").join(&name);
        let _guard = RemovePath(dir.clone());
        let nested = dir.join("sub");
        std::fs::create_dir_all(&nested).expect("dir");
        std::fs::write(nested.join("box.bin"), b"box!").expect("write");
        let relative = std::path::PathBuf::from("target")
            .join(&name)
            .join("sub")
            .join("box.bin");
        assert!(relative.is_relative());
        let bytes = read_unresolved(&relative, 4, "").expect("relative");
        assert_eq!(bytes, b"box!");
    }

    #[cfg(unix)]
    #[test]
    fn relative_directory_component_is_not_resolved() {
        let name = scratch_name("link");
        let root = std::path::PathBuf::from("target").join(&name);
        let _guard = RemovePath(root.clone());
        let outside = root.join("outside");
        let link_parent = root.join("rel");
        std::fs::create_dir_all(&outside).expect("outside");
        std::fs::create_dir_all(&link_parent).expect("rel");
        std::fs::write(outside.join("box.bin"), b"out!").expect("write");
        std::os::unix::fs::symlink(&outside, link_parent.join("sub")).expect("link");
        let relative = std::path::PathBuf::from("target")
            .join(&name)
            .join("rel")
            .join("sub")
            .join("box.bin");
        let err = read_unresolved(&relative, 4, "").expect_err("link");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(windows)]
    struct ClearLeaf;

    #[cfg(windows)]
    impl Drop for ClearLeaf {
        fn drop(&mut self) {
            super::FORCE_CLOUD_REOPEN.with(|cell| cell.set(false));
            super::AFTER_LEAF_OPEN.with(|cell| cell.borrow_mut().take());
            super::FORCE_FILE_REPARSE.with(|cell| cell.set(false));
            super::TAG_RESULT.with(|cell| cell.borrow_mut().take());
        }
    }

    #[cfg(windows)]
    #[test]
    fn cloud_reopen_refuses_a_different_file_id() {
        let _guard = ClearLeaf;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("model.bin");
        std::fs::write(&path, b"one!").expect("write");
        let other = dir.path().join("other.bin");
        std::fs::write(&other, b"two!").expect("write");
        let path_hook = path.clone();
        let other_hook = other.clone();
        super::FORCE_CLOUD_REOPEN.with(|cell| cell.set(true));
        super::AFTER_LEAF_OPEN.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(move || {
                std::fs::remove_file(&path_hook).expect("remove");
                std::fs::rename(&other_hook, &path_hook).expect("rename");
            }));
        });
        let err = read_under(dir.path(), Path::new("model.bin"), 64, "", &path).expect_err("id");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn cloud_reopen_of_the_same_file_is_kept() {
        let _guard = ClearLeaf;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("model.bin");
        std::fs::write(&path, b"one!").expect("write");
        super::FORCE_CLOUD_REOPEN.with(|cell| cell.set(true));
        let bytes = read_under(dir.path(), Path::new("model.bin"), 64, "", &path).expect("same");
        assert_eq!(bytes, b"one!");
    }

    #[cfg(windows)]
    #[test]
    fn file_reparse_tag_that_is_not_cloud_is_refused() {
        let _guard = ClearLeaf;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("model.bin");
        std::fs::write(&path, b"one!").expect("write");
        super::FORCE_FILE_REPARSE.with(|cell| cell.set(true));
        super::TAG_RESULT.with(|cell| *cell.borrow_mut() = Some(Ok(0x9000_001C)));
        let err = read_under(dir.path(), Path::new("model.bin"), 64, "", &path).expect_err("tag");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn file_reparse_tag_read_failure_is_refused() {
        let _guard = ClearLeaf;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("model.bin");
        std::fs::write(&path, b"one!").expect("write");
        super::FORCE_FILE_REPARSE.with(|cell| cell.set(true));
        super::TAG_RESULT.with(|cell| {
            *cell.borrow_mut() = Some(Err(std::io::Error::other("tag")));
        });
        let err = read_under(dir.path(), Path::new("model.bin"), 64, "", &path).expect_err("tag");
        assert!(err.to_string().contains("not a regular file"), "{err}");
    }
}
