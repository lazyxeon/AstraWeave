//! Size-bounded read from a directory handle.
//!
//! On the editor, cook, and runtime paths, one handle covers the container
//! and every URI. A base-taking loader opens the base once for its URIs, and
//! the caller supplies the container bytes. On Unix the handle is a directory
//! file descriptor. Each URI is opened from that descriptor, one component at
//! a time, with `O_NOFOLLOW`, and those descriptors are not kept. An
//! intermediate component uses directory flags. On Linux those flags include
//! `O_PATH` and `O_DIRECTORY`, so an execute-only directory still loads and a
//! non-directory is refused before a read-only open. Other Unix opens an
//! intermediate read-only with `O_DIRECTORY`, so an execute-only directory
//! does not load there. On Windows the root directory handle is its own
//! field. Intermediate directory handles are a separate set, and every URI
//! starts from the root handle. Those directory handles do not share delete,
//! so a directory the load holds cannot be renamed or removed while they are
//! open. Each distinct intermediate directory stays on `OpenedRoot` until
//! that root drops. A repeated open of the same directory does not add
//! another handle. The held set fails closed past 64 directory handles. That
//! budget counts the resolved-root chain, so a deep root leaves fewer URI
//! directory slots. Each URI component is opened from the held parent
//! handle, and that directory's final path must stay under the root. A
//! cloud-file reparse tag is allowed. Delete sharing is on the first open of
//! each URI leaf file, so that name can be replaced while the handle is held.
//! Only a cloud placeholder is opened a second time, without delete sharing,
//! and that second open must be the same file. A symlink,
//! a junction, and a mount point are refused. The file's final path must stay
//! under the root's. Those paths compare as exact UTF-16 units. A name that
//! is not one path component is refused before the create call. An empty
//! relative path is refused. On Unix that read reports the root descriptor is
//! not a regular file. On Windows the message is that the relative path is
//! empty. Directory identity is the 128-bit file id plus the volume serial.
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
/// On the editor, cook, and runtime paths, this handle covers the container
/// and every URI. A base-taking loader opens the base once for its URIs, and
/// the caller supplies the container bytes. On Unix each URI is opened from
/// this descriptor with `O_NOFOLLOW`, and those descriptors are not kept. On
/// Windows the root directory handle is its own field. Intermediate handles
/// are a separate set, and every URI starts from the root handle. Those
/// handles do not share delete, so those directories cannot be renamed or
/// removed while the load holds them. Each distinct intermediate directory
/// stays here until this root drops. A repeated open of the same directory
/// does not add another handle. The held set fails closed past 64 directory
/// handles. That budget counts the resolved-root chain, so a deep root leaves
/// fewer URI directory slots.
#[derive(Debug)]
pub(crate) struct OpenedRoot {
    #[cfg(unix)]
    dir: rustix::fd::OwnedFd,
    #[cfg(windows)]
    root_final: PathBuf,
    /// Resolved root directory. Never stored in `held.files`.
    #[cfg(windows)]
    root_dir: File,
    #[cfg(windows)]
    held: std::sync::Mutex<HeldDirectories>,
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
    // `join` of an empty path leaves the directory unchanged, and
    // `normalize_lexical` drops a current-directory component. Both `""` and
    // `"."` therefore resolve to the current directory without a separate arm.
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
    // The leaf handle is dropped before the hook. The next URI has not opened.
    drop(file);
    #[cfg(all(windows, test))]
    fire_uri_directory_hook(relative);
    Ok(bytes)
}

fn subject(noun: &str, path: &Path) -> String {
    if noun.is_empty() {
        path.display().to_string()
    } else {
        format!("{noun} {}", path.display())
    }
}

pub(crate) fn not_regular_open(err: &std::io::Error) -> bool {
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
    static AFTER_URI_DIRECTORY_OPEN: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
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
    AFTER_URI_DIRECTORY_OPEN.with(|cell| *cell.borrow_mut() = None);
}

#[cfg(test)]
pub(crate) fn root_open_count() -> u32 {
    ROOT_OPENS.with(|cell| cell.get())
}

#[cfg(test)]
pub(crate) fn after_root_open(hook: Box<dyn FnOnce()>) {
    AFTER_ROOT_OPEN.with(|cell| *cell.borrow_mut() = Some(hook));
}

#[cfg(test)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn after_uri_directory_open(hook: Box<dyn FnOnce()>) {
    AFTER_URI_DIRECTORY_OPEN.with(|cell| *cell.borrow_mut() = Some(hook));
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
    Ok(names)
}

#[cfg(unix)]
fn directory_flags() -> rustix::fs::OFlags {
    use rustix::fs::OFlags;
    // Linux opens each directory component with O_PATH so a load still works
    // when an ancestor is not readable. Other Unix keeps a read-only directory
    // open. `open_flag_masks_match_the_target_bits` pins the Linux mask on
    // x86, x86_64, aarch64, and arm, and the other-Unix mask on macOS and iOS.
    // On any other target that test only checks that `O_RDONLY` is zero.
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
fn canonical_root_flags() -> rustix::fs::OFlags {
    use rustix::fs::OFlags;
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC
}

#[cfg(unix)]
fn file_flags() -> rustix::fs::OFlags {
    use rustix::fs::OFlags;
    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK
}

#[cfg(unix)]
const _: () = assert!(rustix::fs::OFlags::RDONLY.bits() == 0);

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
    use rustix::fs::{openat, Mode};

    let directory = directory_flags();
    let file_flags = file_flags();
    let names = relative_names(relative)?;
    // Each URI starts again at the root descriptor. Component descriptors are
    // not kept after this function returns.
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
    use rustix::fs::{openat, Mode, CWD};

    let mut components = absolute.components();
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    let mut dir = opened(openat(CWD, "/", canonical_root_flags(), Mode::empty()))?;
    for component in components {
        match component {
            Component::Normal(name) => {
                dir = opened(openat(&dir, name, flags, Mode::empty()))?;
            }
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
type BeforeComponentHook = Box<dyn FnMut(usize)>;

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
    static AFTER_SECOND_CLOUD_OPEN: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
    static TAG_RESULT: std::cell::RefCell<Option<std::io::Result<u32>>> =
        const { std::cell::RefCell::new(None) };
    static INTERMEDIATE_FINAL_OVERRIDE: std::cell::Cell<Option<PathBuf>> =
        const { std::cell::Cell::new(None) };
    static FILE_ID_ERROR: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
    static FILE_ID_CALLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static FILE_ID_BYTES: std::cell::RefCell<Vec<[u8; 16]>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static NT_CREATE_CALLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static BEFORE_COMPONENT: std::cell::RefCell<Option<BeforeComponentHook>> =
        std::cell::RefCell::new(None);
}

#[cfg(windows)]
fn open_path(path: &Path, reparse: bool) -> std::io::Result<File> {
    // Directory handles do not share delete. The second open of a cloud file
    // also goes through this path and does not share delete. A directory this
    // load holds cannot be renamed or removed while that handle is open.
    open_with_share(path, reparse, false)
}

#[cfg(windows)]
fn open_with_share(path: &Path, reparse: bool, share_delete: bool) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;

    let mut share = FILE_SHARE_READ | FILE_SHARE_WRITE;
    if share_delete {
        share |= FILE_SHARE_DELETE;
    }
    let mut flags = FILE_FLAG_BACKUP_SEMANTICS;
    if reparse {
        flags |= FILE_FLAG_OPEN_REPARSE_POINT;
    }
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(share)
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
        let (root_final, root_dir, held) = open_resolved_directory(absolute)?;
        let root = Self {
            root_final,
            root_dir,
            held: std::sync::Mutex::new(held),
        };
        record_root_open();
        Ok(root)
    }
}

/// Root chain, including the root handle stored outside `files`, plus distinct
/// URI directories. One past this fails the open. A deep root leaves fewer
/// URI slots.
#[cfg(windows)]
const MAX_HELD_DIRECTORIES: usize = 64;
#[cfg(windows)]
const _: () = assert!(MAX_HELD_DIRECTORIES == 64);

#[cfg(windows)]
#[derive(Debug)]
struct HeldDirectories {
    files: Vec<File>,
    ids: std::collections::HashSet<WindowsFileId>,
}

#[cfg(windows)]
impl HeldDirectories {
    fn push(&mut self, file: File) -> std::io::Result<()> {
        let id = windows_file_id(&file)?;
        if self.ids.contains(&id) {
            return Ok(());
        }
        // `files` does not hold the root directory. That handle counts as one.
        if self.files.len() + 1 >= MAX_HELD_DIRECTORIES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "too many open directories",
            ));
        }
        self.ids.insert(id);
        self.files.push(file);
        Ok(())
    }
}

#[cfg(windows)]
fn hold_directory(root: &OpenedRoot, file: File) -> std::io::Result<()> {
    let mut held = root
        .held
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    held.push(file)
}

#[cfg(windows)]
fn open_resolved_directory(resolved: &Path) -> std::io::Result<(PathBuf, File, HeldDirectories)> {
    let mut cursor = PathBuf::new();
    let mut parent_final: Option<PathBuf> = None;
    let mut opened = Vec::new();
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
        opened.push(file);
    }
    let Some(root_final) = parent_final else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    };
    let Some(root_file) = opened.pop() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    };
    let mut held = HeldDirectories {
        files: Vec::new(),
        ids: std::collections::HashSet::new(),
    };
    for file in opened {
        held.push(file)?;
    }
    Ok((root_final, root_file, held))
}

#[cfg(windows)]
fn root_directory_handle(root: &OpenedRoot) -> std::io::Result<File> {
    root.root_dir.try_clone()
}

#[cfg(all(windows, test))]
fn fire_before_component(index: usize) {
    BEFORE_COMPONENT.with(|cell| {
        if let Some(hook) = cell.borrow_mut().as_mut() {
            hook(index);
        }
    });
}

#[cfg(windows)]
fn open_relative(root: &OpenedRoot, relative: &Path) -> std::io::Result<File> {
    let names = relative_names(relative)?;
    let mut parent = root_directory_handle(root)?;
    for (index, name) in names.iter().enumerate() {
        #[cfg(test)]
        fire_before_component(index);
        let last = index + 1 == names.len();
        if !last {
            let file = open_at(&parent, name, RelativeOpen::Directory)?;
            ensure_directory(&file)?;
            let component_final = final_path(&file)?;
            #[cfg(test)]
            let component_final = INTERMEDIATE_FINAL_OVERRIDE
                .with(|cell| cell.take())
                .unwrap_or(component_final);
            if !path_is_under(&root.root_final, &component_final) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "not a regular file",
                ));
            }
            let kept = file.try_clone()?;
            hold_directory(root, file)?;
            parent = kept;
            continue;
        }
        let file = open_leaf_at(&parent, name)?;
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
        return Ok(file);
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "empty relative path",
    ))
}

#[cfg(windows)]
enum RelativeOpen {
    Directory,
    Leaf {
        share_delete: bool,
        follow_reparse: bool,
    },
}

#[cfg(windows)]
#[allow(unsafe_code, reason = "NtCreateFile has no safe wrapper in this crate")]
fn open_at(parent: &File, name: &std::ffi::OsStr, kind: RelativeOpen) -> std::io::Result<File> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *mut u16,
    }

    #[repr(C)]
    struct ObjectAttributes {
        length: u32,
        root_directory: *mut std::ffi::c_void,
        object_name: *mut UnicodeString,
        attributes: u32,
        security_descriptor: *mut std::ffi::c_void,
        security_quality_of_service: *mut std::ffi::c_void,
    }

    #[repr(C)]
    struct IoStatusBlock {
        status: isize,
        information: usize,
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtCreateFile(
            file_handle: *mut *mut std::ffi::c_void,
            desired_access: u32,
            object_attributes: *mut ObjectAttributes,
            io_status_block: *mut IoStatusBlock,
            allocation_size: *mut i64,
            file_attributes: u32,
            share_access: u32,
            create_disposition: u32,
            create_options: u32,
            ea_buffer: *mut std::ffi::c_void,
            ea_length: u32,
        ) -> i32;
        fn RtlNtStatusToDosError(status: i32) -> u32;
    }

    const OBJ_CASE_INSENSITIVE: u32 = 0x0000_0040;
    const FILE_OPEN: u32 = 1;
    const FILE_LIST_DIRECTORY: u32 = 0x0000_0001;
    const FILE_READ_DATA: u32 = 0x0000_0001;
    const FILE_READ_EA: u32 = 0x0000_0008;
    const FILE_TRAVERSE: u32 = 0x0000_0020;
    const FILE_READ_ATTRIBUTES: u32 = 0x0000_0080;
    const READ_CONTROL: u32 = 0x0002_0000;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    // Leaf read access. `FILE_READ_DATA` is the file form of the directory-list bit.
    const FILE_READ_ACCESS: u32 =
        READ_CONTROL | FILE_READ_DATA | FILE_READ_ATTRIBUTES | FILE_READ_EA | SYNCHRONIZE;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    const FILE_DIRECTORY_FILE: u32 = 0x0000_0001;
    const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x0000_0020;
    const FILE_NON_DIRECTORY_FILE: u32 = 0x0000_0040;
    const FILE_OPEN_FOR_BACKUP_INTENT: u32 = 0x0000_4000;
    const FILE_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const STATUS_NOT_A_DIRECTORY: i32 = 0xC000_0103u32 as i32;
    const STATUS_FILE_IS_A_DIRECTORY: i32 = 0xC000_00BAu32 as i32;
    const STATUS_STOPPED_ON_SYMLINK: i32 = 0x8000_002Du32 as i32;

    let mut units: Vec<u16> = name.encode_wide().collect();
    // One path segment. A colon, NUL, separator, or a reserved `.` / `..`
    // name is not a component name.
    let reserved = units == [u16::from(b'.')] || units == [u16::from(b'.'), u16::from(b'.')];
    if units.is_empty()
        || reserved
        || units.len() > (u16::MAX as usize) / 2
        || units.iter().any(|unit| {
            *unit == 0
                || *unit == u16::from(b':')
                || *unit == u16::from(b'\\')
                || *unit == u16::from(b'/')
        })
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    let bytes = (units.len() * 2) as u16;
    let mut unicode = UnicodeString {
        length: bytes,
        maximum_length: bytes,
        buffer: units.as_mut_ptr(),
    };
    let mut attributes = ObjectAttributes {
        length: std::mem::size_of::<ObjectAttributes>() as u32,
        root_directory: parent.as_raw_handle(),
        object_name: &mut unicode,
        attributes: OBJ_CASE_INSENSITIVE,
        security_descriptor: std::ptr::null_mut(),
        security_quality_of_service: std::ptr::null_mut(),
    };
    let (access, share, options) = match kind {
        RelativeOpen::Directory => (
            FILE_LIST_DIRECTORY | FILE_TRAVERSE | FILE_READ_ATTRIBUTES | READ_CONTROL | SYNCHRONIZE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_DIRECTORY_FILE
                | FILE_SYNCHRONOUS_IO_NONALERT
                | FILE_OPEN_FOR_BACKUP_INTENT
                | FILE_OPEN_REPARSE_POINT,
        ),
        RelativeOpen::Leaf {
            share_delete,
            follow_reparse,
        } => {
            let mut options = FILE_NON_DIRECTORY_FILE
                | FILE_SYNCHRONOUS_IO_NONALERT
                | FILE_OPEN_FOR_BACKUP_INTENT;
            if !follow_reparse {
                options |= FILE_OPEN_REPARSE_POINT;
            }
            let mut share = FILE_SHARE_READ | FILE_SHARE_WRITE;
            if share_delete {
                share |= FILE_SHARE_DELETE;
            }
            (FILE_READ_ACCESS, share, options)
        }
    };
    let mut io = IoStatusBlock {
        status: 0,
        information: 0,
    };
    let mut handle = std::ptr::null_mut();
    #[cfg(test)]
    NT_CREATE_CALLS.with(|cell| cell.set(cell.get().saturating_add(1)));
    // SAFETY: `parent` is a live directory handle. `unicode` and `attributes`
    // stay alive for the call, and `units` backs the name. `handle` and `io`
    // are writable outputs. A success status writes one owned handle.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            access,
            &mut attributes,
            &mut io,
            std::ptr::null_mut(),
            0,
            share,
            FILE_OPEN,
            options,
            std::ptr::null_mut(),
            0,
        )
    };
    if status < 0 {
        if status == STATUS_NOT_A_DIRECTORY
            || status == STATUS_FILE_IS_A_DIRECTORY
            || status == STATUS_STOPPED_ON_SYMLINK
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        // SAFETY: `RtlNtStatusToDosError` maps one NTSTATUS and has no buffer.
        let code = unsafe { RtlNtStatusToDosError(status) };
        return Err(std::io::Error::from_raw_os_error(code as i32));
    }
    if handle.is_null() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        ));
    }
    // SAFETY: `NtCreateFile` returned success and `handle` is the new object.
    Ok(File::from(unsafe { OwnedHandle::from_raw_handle(handle) }))
}

#[cfg(all(windows, test))]
fn fire_uri_directory_hook(relative: &Path) {
    let Ok(names) = relative_names(relative) else {
        return;
    };
    if names.len() < 2 {
        return;
    }
    if let Some(hook) = AFTER_URI_DIRECTORY_OPEN.with(|cell| cell.borrow_mut().take()) {
        hook();
    }
}

#[cfg(windows)]
fn open_leaf_at(parent: &File, name: &std::ffi::OsStr) -> std::io::Result<File> {
    // Delete sharing is on this first open of each URI leaf file, so the name
    // can be replaced while the handle is held. A cloud placeholder is opened a
    // second time without delete sharing; the file-id compare then refuses a
    // different file. Both opens are relative to the held parent.
    let first = open_at(
        parent,
        name,
        RelativeOpen::Leaf {
            share_delete: true,
            follow_reparse: false,
        },
    )?;
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
    let second = open_at(
        parent,
        name,
        RelativeOpen::Leaf {
            share_delete: false,
            follow_reparse: true,
        },
    )?;
    // `second` does not share delete. A hook here runs while that handle is held.
    #[cfg(test)]
    if let Some(hook) = AFTER_SECOND_CLOUD_OPEN.with(|cell| cell.borrow_mut().take()) {
        hook();
    }
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
    #[allow(
        dead_code,
        reason = "the unread fields are the Win32 BY_HANDLE_FILE_INFORMATION layout"
    )]
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
    })
}

#[cfg(windows)]
fn windows_link_count(file: &File) -> std::io::Result<u64> {
    Ok(windows_by_handle(file)?.links)
}

/// Compare and hash all 16 id bytes. A shorter prefix treats a high-half
/// change as the same file.
#[cfg(any(windows, test))]
fn file_id_bytes_eq(left: &[u8; 16], right: &[u8; 16]) -> bool {
    left.iter().zip(right.iter()).all(|(a, b)| a == b)
}

#[cfg(any(windows, test))]
fn hash_file_id_bytes<H: std::hash::Hasher>(bytes: &[u8; 16], state: &mut H) {
    for byte in bytes.iter() {
        std::hash::Hash::hash(byte, state);
    }
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug)]
struct WindowsFileId {
    volume: u64,
    file_id: [u8; 16],
}

#[cfg(windows)]
impl PartialEq for WindowsFileId {
    fn eq(&self, other: &Self) -> bool {
        self.volume == other.volume && file_id_bytes_eq(&self.file_id, &other.file_id)
    }
}

#[cfg(windows)]
impl Eq for WindowsFileId {}

#[cfg(windows)]
impl std::hash::Hash for WindowsFileId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.volume, state);
        hash_file_id_bytes(&self.file_id, state);
    }
}

#[cfg(windows)]
#[allow(
    unsafe_code,
    reason = "GetFileInformationByHandleEx has no safe wrapper in this crate"
)]
fn windows_file_id(file: &File) -> std::io::Result<WindowsFileId> {
    #[cfg(test)]
    {
        FILE_ID_CALLS.with(|cell| cell.set(cell.get().saturating_add(1)));
        if let Some(message) = FILE_ID_ERROR.with(|cell| cell.take()) {
            return Err(std::io::Error::other(message));
        }
    }
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;

    #[repr(C)]
    struct FileIdInfo {
        volume_serial_number: u64,
        file_id: [u8; 16],
    }
    const _: () = assert!(std::mem::size_of::<FileIdInfo>() == 24);
    const _: () = assert!(std::mem::offset_of!(FileIdInfo, file_id) == 8);

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandleEx(
            file: *mut c_void,
            class: i32,
            info: *mut c_void,
            size: u32,
        ) -> i32;
    }

    // `FileIdInfo` in `FILE_INFO_BY_HANDLE_CLASS`.
    const FILE_ID_INFO: i32 = 18;
    const _: () = assert!(FILE_ID_INFO == 18);
    let mut info = FileIdInfo {
        volume_serial_number: 0,
        file_id: [0; 16],
    };
    // SAFETY: `file` is the live handle. `info` is writable for one
    // `FileIdInfo`, and that size is the size passed in.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FILE_ID_INFO,
            &mut info as *mut FileIdInfo as *mut c_void,
            std::mem::size_of::<FileIdInfo>() as u32,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    #[cfg(test)]
    if let Some(bytes) = FILE_ID_BYTES.with(|cell| {
        let mut queue = cell.borrow_mut();
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0))
        }
    }) {
        info.file_id = bytes;
    }
    Ok(WindowsFileId {
        volume: info.volume_serial_number,
        file_id: info.file_id,
    })
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

/// A FIFO `open` for reading waits until a writer connects. `op` runs on
/// another thread so this returns when that wait does not finish. Joining the
/// worker would sit on the same open.
#[cfg(all(unix, test))]
pub(crate) fn assert_not_regular_without_blocking(
    op: impl FnOnce() -> Result<()> + Send + 'static,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let message = match op() {
            Ok(()) => String::new(),
            Err(err) => format!("{err:#}"),
        };
        let _ = tx.send(message);
    });
    match rx.recv_timeout(std::time::Duration::from_secs(2)) {
        Ok(message) => {
            worker.join().expect("reader");
            assert!(message.contains("not a regular file"), "{message}");
        }
        Err(_) => {
            drop(worker);
            panic!("fifo open blocked");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_path_bytes(path: &Path, max_bytes: u64, noun: &str) -> Result<Vec<u8>> {
        read_path(path, max_bytes, noun).map(|(bytes, _root)| bytes)
    }

    #[test]
    fn file_id_bytes_that_differ_in_the_high_half_stay_distinct() {
        use std::hash::Hasher;
        let mut low = [0u8; 16];
        low[0] = 1;
        let mut at_eight = low;
        at_eight[8] = 2;
        let mut at_last = low;
        at_last[15] = 3;
        assert!(file_id_bytes_eq(&low, &low));
        assert!(!file_id_bytes_eq(&low, &at_eight));
        assert!(!file_id_bytes_eq(&low, &at_last));
        let hash = |bytes: &[u8; 16]| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            hash_file_id_bytes(bytes, &mut hasher);
            hasher.finish()
        };
        assert_ne!(hash(&low), hash(&at_eight));
        assert_ne!(hash(&low), hash(&at_last));
    }

    #[test]
    fn leading_dot_component_loads_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tex = dir.path().join("tex");
        std::fs::create_dir(&tex).expect("tex");
        std::fs::write(tex.join("a.png"), b"png-bytes").expect("write");
        let root = OpenedRoot::open(dir.path()).expect("root");
        let bytes = root
            .read(Path::new("./tex/a.png"), 64, "", Path::new("./tex/a.png"))
            .expect("dot uri");
        assert_eq!(bytes, b"png-bytes");
    }

    #[test]
    fn empty_relative_path_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.bin"), b"abc").expect("write");
        let root = OpenedRoot::open(dir.path()).expect("root");
        let err = root
            .read(Path::new(""), 64, "", Path::new(""))
            .expect_err("empty");
        let msg = format!("{err:#}");
        #[cfg(unix)]
        {
            assert!(msg.contains("not a regular file"), "{msg}");
            assert!(!msg.contains("empty relative path"), "{msg}");
        }
        #[cfg(windows)]
        {
            assert!(msg.contains("empty relative path"), "{msg}");
            assert!(!msg.contains("not a regular file"), "{msg}");
        }
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
    fn descriptors_named(path: &Path) -> usize {
        let want = std::fs::canonicalize(path).expect("canonical");
        let dir = if Path::new("/proc/self/fd").is_dir() {
            Path::new("/proc/self/fd")
        } else {
            Path::new("/dev/fd")
        };
        std::fs::read_dir(dir)
            .expect("fds")
            .flatten()
            .filter(|entry| {
                std::fs::read_link(entry.path()).ok().as_deref() == Some(want.as_path())
            })
            .count()
    }

    /// Repeated reads of one nested URI start again at the root descriptor.
    #[cfg(unix)]
    #[test]
    fn repeated_nested_uri_does_not_keep_the_intermediate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mid = dir.path().join("mid");
        std::fs::create_dir(&mid).expect("mid");
        let leaf = mid.join("leaf.bin");
        std::fs::write(&leaf, b"leaf").expect("leaf");
        let root = OpenedRoot::open(dir.path()).expect("root");
        for _ in 0..8 {
            let bytes = root
                .read(Path::new("mid/leaf.bin"), 64, "", Path::new("mid/leaf.bin"))
                .expect("read");
            assert_eq!(bytes, b"leaf");
        }
        assert_eq!(descriptors_named(&mid), 0);
    }

    /// An intermediate directory that is execute-only still opens. Directory
    /// flags include `O_PATH` on Linux; a read-only open of that directory does not.
    #[cfg(target_os = "linux")]
    #[test]
    fn execute_only_intermediate_directory_loads() {
        use std::os::unix::fs::PermissionsExt;
        struct Restore(std::path::PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
            }
        }
        let dir = tempfile::tempdir().expect("tempdir");
        let mid = dir.path().join("mid");
        std::fs::create_dir(&mid).expect("mid");
        std::fs::write(mid.join("leaf.bin"), b"leaf").expect("leaf");
        let _restore = Restore(mid.clone());
        std::fs::set_permissions(&mid, std::fs::Permissions::from_mode(0o111)).expect("mode");
        let root = OpenedRoot::open(dir.path()).expect("root");
        let bytes = root
            .read(Path::new("mid/leaf.bin"), 64, "", Path::new("mid/leaf.bin"))
            .expect("execute-only intermediate");
        assert_eq!(bytes, b"leaf");
    }

    /// A regular file used as an intermediate is refused before it is opened
    /// read-only. `O_DIRECTORY` fails in the open check, so inotify sees no `IN_OPEN`.
    #[cfg(target_os = "linux")]
    #[test]
    fn non_directory_intermediate_is_refused_before_a_read_only_open() {
        use std::mem::MaybeUninit;
        let dir = tempfile::tempdir().expect("tempdir");
        let mid = dir.path().join("mid");
        std::fs::write(&mid, b"not-a-dir").expect("file");
        let root = OpenedRoot::open(dir.path()).expect("root");
        let watch = rustix::fs::inotify::init(
            rustix::fs::inotify::CreateFlags::NONBLOCK | rustix::fs::inotify::CreateFlags::CLOEXEC,
        )
        .expect("inotify");
        rustix::fs::inotify::add_watch(&watch, &mid, rustix::fs::inotify::WatchFlags::OPEN)
            .expect("watch");
        let err = root
            .read(Path::new("mid/leaf.bin"), 64, "", Path::new("mid/leaf.bin"))
            .expect_err("file intermediate");
        let msg = format!("{err:#}");
        assert!(msg.contains("not a regular file"), "{msg}");
        let mut buf = [MaybeUninit::uninit(); 256];
        let mut reader = rustix::fs::inotify::Reader::new(&watch, &mut buf);
        match reader.next() {
            Err(rustix::io::Errno::AGAIN) => {}
            Ok(event) => panic!("intermediate was opened: {event:?}"),
            Err(err) => panic!("inotify read: {err}"),
        }
    }

    /// The leaf of a URI is a FIFO. The open must fail before a writer connects.
    #[cfg(unix)]
    #[test]
    fn fifo_uri_is_rejected_without_blocking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sub = dir.path().join("textures");
        std::fs::create_dir(&sub).expect("textures");
        let pipe = sub.join("pipe.bin");
        let status = std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .expect("mkfifo");
        assert!(status.success(), "mkfifo");
        let root = dir.path().to_path_buf();
        let display = std::path::PathBuf::from("textures/pipe.bin");
        let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = ran.clone();
        assert_not_regular_without_blocking(move || {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
            read_under(&root, Path::new("textures/pipe.bin"), 64, "", &display).map(|_| ())
        });
        assert!(
            ran.load(std::sync::atomic::Ordering::Relaxed),
            "the non-blocking check did not run the open"
        );
    }

    /// A FIFO used as an intermediate must fail before a writer connects.
    /// Other Unix opens that component read-only, so a missing directory flag
    /// would block here.
    #[cfg(unix)]
    #[test]
    fn fifo_intermediate_is_rejected_without_blocking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pipe = dir.path().join("mid");
        let status = std::process::Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .expect("mkfifo");
        assert!(status.success(), "mkfifo");
        let root = dir.path().to_path_buf();
        let display = std::path::PathBuf::from("mid/leaf.bin");
        let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = ran.clone();
        assert_not_regular_without_blocking(move || {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
            read_under(&root, Path::new("mid/leaf.bin"), 64, "", &display).map(|_| ())
        });
        assert!(
            ran.load(std::sync::atomic::Ordering::Relaxed),
            "the non-blocking check did not run the open"
        );
    }

    #[cfg(unix)]
    #[test]
    fn open_flag_masks_match_the_target_bits() {
        use rustix::fs::OFlags;
        assert_eq!(OFlags::RDONLY.bits(), 0);
        #[cfg(all(
            any(target_os = "linux", target_os = "android"),
            any(target_arch = "x86_64", target_arch = "x86")
        ))]
        {
            assert_eq!(super::directory_flags().bits(), 0x002B_0000);
            assert_eq!(super::canonical_root_flags().bits(), 0x0009_0000);
            assert_eq!(super::file_flags().bits(), 0x000A_0800);
        }
        #[cfg(all(
            any(target_os = "linux", target_os = "android"),
            any(target_arch = "aarch64", target_arch = "arm")
        ))]
        {
            assert_eq!(super::directory_flags().bits(), 0x0028_C000);
            assert_eq!(super::canonical_root_flags().bits(), 0x0008_4000);
            assert_eq!(super::file_flags().bits(), 0x0008_8800);
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            assert_eq!(super::directory_flags().bits(), 0x0110_0100);
            assert_eq!(super::canonical_root_flags().bits(), 0x0110_0000);
            assert_eq!(super::file_flags().bits(), 0x0100_0104);
        }
    }

    #[cfg(unix)]
    #[test]
    fn opened_descriptors_are_close_on_exec() {
        use rustix::fs::{openat, Mode, CWD};
        use rustix::io::{fcntl_getfd, FdFlags};
        let root_fd = openat(CWD, "/", canonical_root_flags(), Mode::empty()).expect("slash");
        let root_flags = fcntl_getfd(&root_fd).expect("slash flags");
        assert!(root_flags.contains(FdFlags::CLOEXEC));
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.bin"), b"abc").expect("write");
        let opened_root = OpenedRoot::open(dir.path()).expect("root");
        let dir_flags = fcntl_getfd(&opened_root.dir).expect("dir flags");
        assert!(dir_flags.contains(FdFlags::CLOEXEC));
        let file = open_relative(&opened_root, Path::new("a.bin")).expect("file");
        let file_flags = fcntl_getfd(&file).expect("file flags");
        assert!(file_flags.contains(FdFlags::CLOEXEC));
    }

    #[cfg(unix)]
    #[test]
    fn slash_root_descriptor_is_close_on_exec() {
        use rustix::io::{fcntl_getfd, FdFlags};
        let root = OpenedRoot::open(Path::new("/")).expect("slash root");
        let flags = fcntl_getfd(&root.dir).expect("slash flags");
        assert!(flags.contains(FdFlags::CLOEXEC));
    }

    #[cfg(unix)]
    #[test]
    fn opening_a_regular_file_as_the_root_fails_at_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("plain.bin");
        std::fs::write(&file, b"bytes").expect("write");
        OpenedRoot::open(&file).expect_err("a regular file is not a directory");
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
            super::AFTER_SECOND_CLOUD_OPEN.with(|cell| cell.borrow_mut().take());
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
    fn second_cloud_open_refuses_a_rename() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let _guard = ClearLeaf;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("model.bin");
        std::fs::write(&path, b"one!").expect("write");
        let hold = dir.path().join("held.bin");
        let fired = Arc::new(AtomicBool::new(false));
        let renamed = Arc::new(AtomicBool::new(false));
        let fired_hook = Arc::clone(&fired);
        let renamed_hook = Arc::clone(&renamed);
        let path_hook = path.clone();
        super::FORCE_CLOUD_REOPEN.with(|cell| cell.set(true));
        super::AFTER_SECOND_CLOUD_OPEN.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(move || {
                fired_hook.store(true, Ordering::SeqCst);
                let result = std::fs::rename(&path_hook, &hold);
                if result.is_ok() {
                    std::fs::rename(&hold, &path_hook).expect("restore");
                    renamed_hook.store(true, Ordering::SeqCst);
                }
            }));
        });
        let bytes = read_under(dir.path(), Path::new("model.bin"), 64, "", &path).expect("read");
        assert!(
            fired.load(Ordering::SeqCst),
            "second cloud open hook did not fire"
        );
        assert!(
            !renamed.load(Ordering::SeqCst),
            "second cloud open allowed a rename"
        );
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

    #[cfg(windows)]
    fn held_ids(root: &OpenedRoot) -> Vec<WindowsFileId> {
        let held = root
            .held
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let mut ids: Vec<_> = held
            .files
            .iter()
            .map(|file| windows_file_id(file).expect("file id"))
            .collect();
        ids.push(windows_file_id(&root.root_dir).expect("root id"));
        ids
    }

    /// Eight reads of one nested URI add that intermediate directory once.
    #[cfg(windows)]
    #[test]
    fn repeated_nested_uri_keeps_one_intermediate_handle() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mid = dir.path().join("mid");
        std::fs::create_dir(&mid).expect("mid");
        let leaf = mid.join("leaf.bin");
        std::fs::write(&leaf, b"leaf").expect("leaf");
        let root = OpenedRoot::open(dir.path()).expect("root");
        let before = held_ids(&root);
        let bytes = root
            .read(Path::new("mid/leaf.bin"), 64, "", Path::new("mid/leaf.bin"))
            .expect("read");
        assert_eq!(bytes, b"leaf");
        let once = held_ids(&root);
        assert_eq!(once.len(), before.len() + 1);
        let mid_file = open_path(&mid, true).expect("mid");
        let mid_id = windows_file_id(&mid_file).expect("mid id");
        let leaf_file = std::fs::File::open(&leaf).expect("leaf");
        let leaf_id = windows_file_id(&leaf_file).expect("leaf id");
        assert!(once.contains(&mid_id));
        assert!(!once.contains(&leaf_id));
        for _ in 0..7 {
            let again = root
                .read(Path::new("mid/leaf.bin"), 64, "", Path::new("mid/leaf.bin"))
                .expect("again");
            assert_eq!(again, b"leaf");
        }
        assert_eq!(held_ids(&root), once);
    }

    #[cfg(windows)]
    struct ClearComponent;

    #[cfg(windows)]
    impl Drop for ClearComponent {
        fn drop(&mut self) {
            super::INTERMEDIATE_FINAL_OVERRIDE.with(|cell| cell.take());
            super::FILE_ID_ERROR.with(|cell| cell.take());
            super::FILE_ID_CALLS.with(|cell| cell.set(0));
            super::FILE_ID_BYTES.with(|cell| cell.borrow_mut().clear());
            super::NT_CREATE_CALLS.with(|cell| cell.set(0));
            super::BEFORE_COMPONENT.with(|cell| cell.borrow_mut().take());
        }
    }

    #[cfg(windows)]
    #[test]
    fn file_id_error_refuses_the_uri() {
        let _guard = ClearComponent;
        let dir = tempfile::tempdir().expect("tempdir");
        let mid = dir.path().join("mid");
        std::fs::create_dir(&mid).expect("mid");
        std::fs::write(mid.join("leaf.bin"), b"leaf").expect("leaf");
        let root = OpenedRoot::open(dir.path()).expect("root");
        super::FILE_ID_ERROR.with(|cell| cell.set(Some("file id unavailable")));
        let err = root
            .read(Path::new("mid/leaf.bin"), 64, "", Path::new("mid/leaf.bin"))
            .expect_err("file id");
        let msg = format!("{err:#}");
        assert!(msg.contains("file id unavailable"), "{msg}");
        let bytes = root
            .read(Path::new("mid/leaf.bin"), 64, "", Path::new("mid/leaf.bin"))
            .expect("read after the injected error");
        assert_eq!(bytes, b"leaf");
    }

    #[cfg(windows)]
    #[test]
    fn intermediate_final_path_outside_the_root_is_refused() {
        let _guard = ClearComponent;
        let dir = tempfile::tempdir().expect("tempdir");
        let mid = dir.path().join("mid");
        std::fs::create_dir(&mid).expect("mid");
        std::fs::write(mid.join("leaf.bin"), b"leaf").expect("leaf");
        super::INTERMEDIATE_FINAL_OVERRIDE.with(|cell| {
            cell.set(Some(std::path::PathBuf::from(r"\\?\D:\outside\mid")));
        });
        let err = read_under(
            dir.path(),
            Path::new("mid/leaf.bin"),
            64,
            "",
            Path::new("mid/leaf.bin"),
        )
        .expect_err("outside intermediate");
        assert!(err.to_string().contains("not a regular file"), "{err}");
        let bytes = read_under(
            dir.path(),
            Path::new("mid/leaf.bin"),
            64,
            "",
            Path::new("mid/leaf.bin"),
        )
        .expect("real intermediate");
        assert_eq!(bytes, b"leaf");
    }

    #[cfg(windows)]
    #[test]
    fn earlier_component_junction_is_not_followed() {
        let base = tempfile::tempdir().expect("tempdir");
        let jail = base.path().join("jail");
        std::fs::create_dir(&jail).expect("jail");
        let outside = base.path().join("outside");
        let outside_late = outside.join("late");
        std::fs::create_dir_all(&outside_late).expect("outside");
        std::fs::write(outside_late.join("leaf.bin"), b"pwned").expect("pwned");
        let early = jail.join("early");
        junction(&early, &outside);
        let root = OpenedRoot::open(&jail).expect("root");
        let before = held_ids(&root);
        let err = root
            .read(
                Path::new("early/late/leaf.bin"),
                64,
                "",
                Path::new("early/late/leaf.bin"),
            )
            .expect_err("junction");
        let msg = format!("{err:#}");
        assert!(msg.contains("not a regular file"), "{msg}");
        assert!(!msg.contains("pwned"), "{msg}");
        assert_eq!(held_ids(&root), before);
    }

    #[cfg(windows)]
    #[test]
    fn earlier_component_swapped_to_a_junction_stays_on_the_held_directory() {
        let _guard = ClearComponent;
        let base = tempfile::tempdir().expect("tempdir");
        let jail = base.path().join("jail");
        let early = jail.join("early");
        let late = early.join("late");
        std::fs::create_dir_all(&late).expect("late");
        std::fs::write(late.join("leaf.bin"), b"safe").expect("safe");
        let outside = base.path().join("outside");
        let outside_late = outside.join("late");
        std::fs::create_dir_all(&outside_late).expect("outside");
        std::fs::write(outside_late.join("leaf.bin"), b"pwned").expect("pwned");
        std::fs::write(outside.join("leaf.bin"), b"pwned").expect("outside leaf");
        let root = OpenedRoot::open(&jail).expect("root");
        let early_hook = early.clone();
        let late_hook = late.clone();
        let outside_hook = outside.clone();
        let parked = base.path().join("parked");
        let early_renamed = std::rc::Rc::new(std::cell::Cell::new(false));
        let late_renamed = std::rc::Rc::new(std::cell::Cell::new(false));
        let early_flag = early_renamed.clone();
        let late_flag = late_renamed.clone();
        super::BEFORE_COMPONENT.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(move |index| {
                if index != 1 {
                    return;
                }
                early_flag.set(std::fs::rename(&early_hook, &parked).is_ok());
                if early_flag.get() {
                    junction(&early_hook, &outside_hook);
                    return;
                }
                let held_late = early_hook.join("late_held");
                late_flag.set(std::fs::rename(&late_hook, &held_late).is_ok());
                if late_flag.get() {
                    junction(&late_hook, &outside_hook);
                }
            }));
        });
        let held = root.read(
            Path::new("early/late/leaf.bin"),
            64,
            "",
            Path::new("early/late/leaf.bin"),
        );
        assert!(
            !early_renamed.get(),
            "the held directory was renamed while its handle was open"
        );
        assert!(
            late_renamed.get(),
            "the directory that was not held was not replaced"
        );
        let err = held.expect_err("held handle followed the junction");
        let msg = format!("{err:#}");
        assert!(msg.contains("not a regular file"), "{msg}");
        assert!(!msg.contains("pwned"), "{msg}");
        let path_bytes = std::fs::read(early.join("late").join("leaf.bin")).expect("path read");
        assert_eq!(path_bytes, b"pwned");
    }

    #[cfg(windows)]
    fn nested_leaf(dir: &Path, depth: usize) -> std::path::PathBuf {
        let mut rel = std::path::PathBuf::new();
        let mut cursor = dir.to_path_buf();
        for index in 0..depth {
            let name = format!("d{index}");
            cursor.push(&name);
            std::fs::create_dir(&cursor).expect("dir");
            rel.push(&name);
        }
        cursor.push("leaf.bin");
        rel.push("leaf.bin");
        std::fs::write(&cursor, b"leaf").expect("leaf");
        rel
    }

    #[cfg(windows)]
    #[test]
    fn directory_past_the_handle_budget_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = OpenedRoot::open(dir.path()).expect("root");
        assert_eq!(super::MAX_HELD_DIRECTORIES, 64);
        let used = held_ids(&root).len();
        assert!(used < 64, "root chain uses the budget");
        assert!(used >= 1, "the root handle counts");
        let room = 64 - used;
        let within = nested_leaf(dir.path(), room);
        let bytes = root.read(&within, 64, "", &within).expect("at the budget");
        assert_eq!(bytes, b"leaf");
        assert_eq!(held_ids(&root).len(), 64);

        let over_dir = tempfile::tempdir().expect("over");
        let over = OpenedRoot::open(over_dir.path()).expect("over root");
        let over_used = held_ids(&over).len();
        let too_many = nested_leaf(over_dir.path(), 65 - over_used);
        let err = over
            .read(&too_many, 64, "", &too_many)
            .expect_err("past the budget");
        let msg = format!("{err:#}");
        assert!(msg.contains("too many open directories"), "{msg}");
    }

    #[cfg(windows)]
    #[test]
    fn repeated_distinct_directories_do_not_rescan_every_held_id() {
        let _guard = ClearComponent;
        let dir = tempfile::tempdir().expect("tempdir");
        let root = OpenedRoot::open(dir.path()).expect("root");
        let before = held_ids(&root).len();
        let count = 24usize;
        let mut names = Vec::new();
        for index in 0..count {
            let name = format!("s{index}");
            let mid = dir.path().join(&name);
            std::fs::create_dir(&mid).expect("mid");
            std::fs::write(mid.join("leaf.bin"), b"leaf").expect("leaf");
            names.push(format!("{name}/leaf.bin"));
        }
        super::FILE_ID_CALLS.with(|cell| cell.set(0));
        for name in &names {
            let bytes = root
                .read(Path::new(name), 64, "", Path::new(name))
                .expect("read");
            assert_eq!(bytes, b"leaf");
        }
        for name in &names {
            let bytes = root
                .read(Path::new(name), 64, "", Path::new(name))
                .expect("again");
            assert_eq!(bytes, b"leaf");
        }
        let calls = super::FILE_ID_CALLS.with(|cell| cell.get());
        assert!(
            calls <= (count as u32) * 3,
            "file-id lookups {calls} for {count} directories"
        );
        assert_eq!(held_ids(&root).len(), before + count);
    }

    /// A nested URI must not move the directory later top-level reads start from.
    #[cfg(windows)]
    #[test]
    fn top_level_file_after_a_nested_uri_is_the_root_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tex = dir.path().join("tex");
        std::fs::create_dir(&tex).expect("tex");
        std::fs::write(tex.join("a.png"), b"nested-a").expect("a");
        std::fs::write(tex.join("b.png"), b"nested-b").expect("nested b");
        std::fs::write(dir.path().join("b.png"), b"root-b").expect("root b");
        let root = OpenedRoot::open(dir.path()).expect("root");
        let nested = root
            .read(Path::new("tex/a.png"), 64, "", Path::new("tex/a.png"))
            .expect("nested");
        assert_eq!(nested, b"nested-a");
        let top = root
            .read(Path::new("b.png"), 64, "", Path::new("b.png"))
            .expect("top");
        assert_eq!(top, b"root-b");
        let again = root
            .read(Path::new("tex/a.png"), 64, "", Path::new("tex/a.png"))
            .expect("nested again");
        assert_eq!(again, b"nested-a");
    }

    #[cfg(windows)]
    #[test]
    fn inside_root_junction_is_not_followed() {
        let base = tempfile::tempdir().expect("tempdir");
        let jail = base.path().join("jail");
        let real = jail.join("real");
        std::fs::create_dir_all(&real).expect("real");
        std::fs::write(real.join("leaf.bin"), b"real-bytes").expect("leaf");
        let via = jail.join("via");
        junction(&via, &real);
        let root = OpenedRoot::open(&jail).expect("root");
        let err = root
            .read(Path::new("via/leaf.bin"), 64, "", Path::new("via/leaf.bin"))
            .expect_err("junction");
        let msg = format!("{err:#}");
        assert!(msg.contains("not a regular file"), "{msg}");
        assert!(!msg.contains("real-bytes"), "{msg}");
    }

    #[cfg(windows)]
    #[test]
    fn inside_root_symlink_leaf_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real = dir.path().join("real.bin");
        std::fs::write(&real, b"target-bytes").expect("real");
        let link = dir.path().join("link.bin");
        std::os::windows::fs::symlink_file(&real, &link).expect("symlink");
        let err = read_under(
            dir.path(),
            Path::new("link.bin"),
            64,
            "",
            Path::new("link.bin"),
        )
        .expect_err("symlink leaf");
        let msg = format!("{err:#}");
        assert!(msg.contains("not a regular file"), "{msg}");
        assert!(!msg.contains("target-bytes"), "{msg}");
    }

    #[cfg(windows)]
    #[test]
    fn windows_file_id_high_half_is_part_of_eq_and_hash() {
        let mut low = [0u8; 16];
        low[0] = 1;
        let mut high = low;
        high[8] = 4;
        let left = WindowsFileId {
            volume: 3,
            file_id: low,
        };
        let right = WindowsFileId {
            volume: 3,
            file_id: high,
        };
        assert_ne!(left, right);
        let mut set = std::collections::HashSet::new();
        set.insert(left);
        assert!(
            set.insert(right),
            "an id that differs in the high half was treated as the same file"
        );
        assert_eq!(set.len(), 2);
    }

    #[cfg(windows)]
    #[test]
    fn file_ids_differing_in_the_high_half_are_distinct() {
        let _guard = ClearComponent;
        let dir = tempfile::tempdir().expect("tempdir");
        for name in ["a", "b"] {
            let mid = dir.path().join(name);
            std::fs::create_dir(&mid).expect("mid");
            std::fs::write(mid.join("leaf.bin"), b"leaf").expect("leaf");
        }
        let root = OpenedRoot::open(dir.path()).expect("root");
        let before = stored_ids(&root).len();
        let mut low = [0u8; 16];
        low[0] = 1;
        let mut high = low;
        high[15] = 2;
        super::FILE_ID_BYTES.with(|cell| {
            cell.borrow_mut().clear();
            cell.borrow_mut().extend([low, high]);
        });
        for name in ["a/leaf.bin", "b/leaf.bin"] {
            let bytes = root
                .read(Path::new(name), 64, "", Path::new(name))
                .expect("read");
            assert_eq!(bytes, b"leaf");
        }
        let ids = stored_ids(&root);
        assert!(ids.iter().any(|id| id.file_id == low), "{ids:?}");
        assert!(ids.iter().any(|id| id.file_id == high), "{ids:?}");
        assert_eq!(ids.len(), before + 2);
    }

    #[cfg(windows)]
    fn stored_ids(root: &OpenedRoot) -> Vec<WindowsFileId> {
        let held = root
            .held
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        held.ids.iter().copied().collect()
    }

    #[cfg(windows)]
    #[test]
    fn windows_file_id_from_two_real_files_is_distinct() {
        let dir = tempfile::tempdir().expect("tempdir");
        let left_path = dir.path().join("a.bin");
        let right_path = dir.path().join("b.bin");
        std::fs::write(&left_path, b"left").expect("left");
        std::fs::write(&right_path, b"right").expect("right");
        super::FILE_ID_BYTES.with(|cell| cell.borrow_mut().clear());
        super::FILE_ID_ERROR.with(|cell| cell.take());
        let left = File::open(&left_path).expect("open left");
        let right = File::open(&right_path).expect("open right");
        let left_id = windows_file_id(&left).expect("left id");
        let right_id = windows_file_id(&right).expect("right id");
        assert_eq!(left_id.file_id.len(), 16);
        assert_eq!(right_id.file_id.len(), 16);
        assert_ne!(left_id.file_id, [0; 16], "{left_id:?}");
        assert_ne!(right_id.file_id, [0; 16], "{right_id:?}");
        assert_ne!(left_id, right_id, "{left_id:?} {right_id:?}");
    }

    #[cfg(windows)]
    #[test]
    fn open_at_refuses_a_name_with_a_separator_or_a_colon() {
        use std::os::windows::ffi::OsStringExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("sub");
        std::fs::create_dir(&nested).expect("sub");
        std::fs::write(nested.join("ok.bin"), b"nested").expect("nested");
        std::fs::write(dir.path().join("ok.bin"), b"ok").expect("ok");
        let root = OpenedRoot::open(dir.path()).expect("root");
        let parent = root.root_dir.try_clone().expect("clone");
        super::NT_CREATE_CALLS.with(|cell| cell.set(0));
        let rejected = [
            std::ffi::OsString::from("ok.bin:$DATA"),
            std::ffi::OsString::from("sub\\ok.bin"),
            std::ffi::OsString::from("sub/ok.bin"),
            std::ffi::OsString::from_wide(&[b'o' as u16, b'k' as u16, 0, b'x' as u16]),
            std::ffi::OsString::from("."),
            std::ffi::OsString::from(".."),
        ];
        for name in &rejected {
            let err = open_at(
                &parent,
                name,
                RelativeOpen::Leaf {
                    share_delete: false,
                    follow_reparse: false,
                },
            )
            .expect_err("rejected name");
            assert!(
                err.to_string().contains("not a regular file"),
                "{name:?} {err}"
            );
        }
        let calls = super::NT_CREATE_CALLS.with(|cell| cell.get());
        assert_eq!(calls, 0, "rejected names reached the create call");
        let mut file = open_at(
            &parent,
            std::ffi::OsStr::new("ok.bin"),
            RelativeOpen::Leaf {
                share_delete: false,
                follow_reparse: false,
            },
        )
        .expect("leaf");
        let calls = super::NT_CREATE_CALLS.with(|cell| cell.get());
        assert_eq!(calls, 1);
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut bytes).expect("read");
        assert_eq!(bytes, b"ok");
    }

    fn error_kind(err: &anyhow::Error) -> Option<std::io::ErrorKind> {
        err.chain()
            .find_map(|cause| cause.downcast_ref::<std::io::Error>().map(|io| io.kind()))
    }

    fn assert_keeps_not_found(err: &anyhow::Error) {
        let msg = format!("{err:#}");
        assert!(!msg.contains("not a regular file"), "{msg}");
        assert_eq!(error_kind(err), Some(std::io::ErrorKind::NotFound), "{msg}");
    }

    #[test]
    fn not_regular_open_needs_both_the_kind_and_the_message() {
        let both = std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a regular file");
        let kind_only = std::io::Error::new(std::io::ErrorKind::InvalidInput, "other");
        let message_only = std::io::Error::new(std::io::ErrorKind::NotFound, "not a regular file");
        assert!(not_regular_open(&both));
        assert!(!not_regular_open(&kind_only));
        assert!(!not_regular_open(&message_only));
    }

    #[test]
    fn directory_of_a_bare_name_is_the_current_directory() {
        assert_eq!(directory_of(Path::new("file.bin")), Path::new("."));
        assert_eq!(directory_of(Path::new("dir/file.bin")), Path::new("dir"));
    }

    #[test]
    fn empty_and_dot_paths_are_the_current_directory() {
        let cwd = std::env::current_dir().expect("cwd");
        assert_eq!(lexical_absolute(Path::new("")).expect("empty"), cwd);
        assert_eq!(lexical_absolute(Path::new(".")).expect("dot"), cwd);
        assert_eq!(
            lexical_absolute(Path::new("a/../b")).expect("nested"),
            cwd.join("b")
        );
    }

    #[test]
    fn missing_parent_after_resolve_keeps_its_error_class() {
        let _guard = ClearResolveHook;
        let parent = tempfile::tempdir().expect("parent");
        let jail = parent.path().join("jail");
        std::fs::create_dir(&jail).expect("jail");
        let model = jail.join("model.gltf");
        std::fs::write(&model, b"gltf").expect("write");
        let jail_hook = jail.clone();
        let parked = parent.path().join("parked");
        AFTER_RESOLVE.with(|cell| {
            *cell.borrow_mut() = Some(Box::new(move || {
                std::fs::rename(&jail_hook, &parked).expect("park");
            }));
        });
        let err = read_path(&model, 64, "").expect_err("missing parent");
        assert_keeps_not_found(&err);
    }

    #[test]
    fn unresolved_missing_parent_keeps_its_error_class() {
        let missing = std::env::temp_dir()
            .join("astraweave-missing-parent-guard")
            .join("file.bin");
        let err = read_unresolved(&missing, 64, "").expect_err("missing");
        assert_keeps_not_found(&err);
    }

    #[test]
    fn missing_root_keeps_its_error_class() {
        let missing = std::env::temp_dir().join("astraweave-missing-root-guard");
        let err = read_under(&missing, Path::new("a.bin"), 64, "", Path::new("a.bin"))
            .expect_err("missing");
        assert_keeps_not_found(&err);
    }

    #[test]
    fn missing_leaf_keeps_its_error_class() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = read_under(
            dir.path(),
            Path::new("missing.bin"),
            64,
            "",
            Path::new("missing.bin"),
        )
        .expect_err("leaf");
        assert_keeps_not_found(&err);
    }

    fn component_text(path: &str) -> Vec<String> {
        windows_final_components(&utf16(path))
            .unwrap_or_else(|| panic!("components {path}"))
            .into_iter()
            .map(|part| String::from_utf16(&part).unwrap_or_else(|_| panic!("utf16 {path}")))
            .collect()
    }

    #[test]
    fn final_path_components_keep_the_prefix_shape() {
        assert!(windows_final_components(&[]).is_none());
        assert_eq!(
            component_text(r"\\?\C:\jail\a.bin"),
            vec![r"\\?\C:", "jail", "a.bin"]
        );
        assert_eq!(
            component_text(r"C:\jail\a.bin"),
            vec!["C:", "jail", "a.bin"]
        );
        assert_eq!(
            component_text(r"\\server\share\jail\a.bin"),
            vec![r"\\server", "share", "jail", "a.bin"]
        );
        assert_eq!(
            component_text(r"\\?\UNC\server\share\jail\a.bin"),
            vec![r"\\?\UNC", "server", "share", "jail", "a.bin"]
        );
        assert_eq!(
            component_text(r"\\?\UNC/server/share/a.bin"),
            vec![r"\\?\UNC", "server", "share", "a.bin"]
        );
        assert!(windows_final_path_is_under(
            r"\\server\share\jail",
            r"\\server\share\jail\a.bin"
        ));
        assert!(windows_final_components(&utf16(r"C:\jail\..\a.bin")).is_none());
        assert!(windows_final_components(&utf16(r"C:\jail\.\a.bin")).is_none());
    }
}
