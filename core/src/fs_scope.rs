//! Filesystem validation and descriptor-relative mutations.
use crate::error::{ErrorCode, ManagerError};
use std::{
    fs,
    io::Write,
    path::{Component, Path},
};
pub(crate) fn io_error(error: std::io::Error) -> ManagerError {
    ManagerError::new(ErrorCode::IoError, error.to_string())
}
pub fn component(value: &str) -> Result<(), ManagerError> {
    let mut parts = Path::new(value).components();
    if value.is_empty()
        || !matches!(parts.next(), Some(Component::Normal(_)))
        || parts.next().is_some()
        || value.contains(['/', '\\', '\0'])
    {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Expected one safe path component",
        ));
    }
    Ok(())
}
pub fn relative(path: &Path) -> Result<(), ManagerError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
        || path.to_string_lossy().contains(['\\', '\0'])
    {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Expected a nonempty relative path without traversal",
        ));
    }
    Ok(())
}
pub fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}
pub fn ensure_no_symlink_parents(root: &Path, rel: &Path) -> Result<(), ManagerError> {
    relative(rel)?;
    let mut current = root.to_path_buf();
    if let Ok(meta) = fs::symlink_metadata(&current) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(ManagerError::new(
                ErrorCode::InvalidPath,
                "Root must be a directory",
            ));
        }
    }
    for part in rel.parent().unwrap_or(Path::new("")).components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                return Err(ManagerError::new(
                    ErrorCode::InvalidPath,
                    format!("Unsafe parent: {}", current.display()),
                ))
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(io_error(e)),
        }
    }
    Ok(())
}
#[cfg(unix)]
pub struct Directory {
    file: fs::File,
}
#[cfg(unix)]
impl Directory {
    pub(crate) fn mod_content_digest(&self) -> Result<String, ManagerError> {
        use sha2::{Digest, Sha256};
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::MetadataExt},
        };
        fn names(file: &fs::File) -> Result<Vec<std::ffi::OsString>, ManagerError> {
            let mut names = fs::read_dir(format!("/proc/self/fd/{}", file.as_raw_fd()))
                .map_err(io_error)?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(io_error)?;
            names.sort();
            Ok(names)
        }
        fn hash_dir(file: &fs::File) -> Result<[u8; 32], ManagerError> {
            let before = names(file)?;
            let mut digest = Sha256::new();
            digest.update(b"directory");
            for name in &before {
                let c_name = std::ffi::CString::new(name.as_bytes())
                    .map_err(|_| ManagerError::new(ErrorCode::InvalidPath, "NUL in file name"))?;
                let fd = unsafe {
                    libc::openat(
                        file.as_raw_fd(),
                        c_name.as_ptr(),
                        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                    )
                };
                if fd < 0 {
                    return Err(ManagerError::new(
                        ErrorCode::InvalidPath,
                        "Cannot safely read managed content",
                    ));
                }
                let mut child = unsafe { fs::File::from_raw_fd(fd) };
                let metadata = child.metadata().map_err(io_error)?;
                digest.update((name.as_bytes().len() as u64).to_le_bytes());
                digest.update(name.as_bytes());
                if metadata.is_dir() {
                    digest.update(hash_dir(&child)?);
                } else if metadata.is_file() {
                    use std::io::Read;
                    let mut content = Sha256::new();
                    content.update(b"file");
                    let mut buffer = [0; 65536];
                    loop {
                        let count = child.read(&mut buffer).map_err(io_error)?;
                        if count == 0 {
                            break;
                        }
                        content.update(&buffer[..count]);
                    }
                    let after = child.metadata().map_err(io_error)?;
                    if (
                        metadata.len(),
                        metadata.mtime(),
                        metadata.mtime_nsec(),
                        metadata.ctime(),
                        metadata.ctime_nsec(),
                    ) != (
                        after.len(),
                        after.mtime(),
                        after.mtime_nsec(),
                        after.ctime(),
                        after.ctime_nsec(),
                    ) {
                        return Err(ManagerError::new(
                            ErrorCode::InvalidPath,
                            "Managed content changed while reading",
                        ));
                    }
                    digest.update(content.finalize());
                } else {
                    return Err(ManagerError::new(
                        ErrorCode::InvalidPath,
                        "Special files are not managed release content",
                    ));
                }
                let current = fs::symlink_metadata(
                    Path::new(&format!("/proc/self/fd/{}", file.as_raw_fd())).join(name),
                )
                .map_err(io_error)?;
                if (metadata.dev(), metadata.ino()) != (current.dev(), current.ino()) {
                    return Err(ManagerError::new(
                        ErrorCode::InvalidPath,
                        "Managed content was replaced while reading",
                    ));
                }
            }
            if names(file)? != before {
                return Err(ManagerError::new(
                    ErrorCode::InvalidPath,
                    "Managed content changed while reading",
                ));
            }
            Ok(digest.finalize().into())
        }
        Ok(hash_dir(&self.file)?
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }

    pub fn open(path: &Path) -> Result<Self, ManagerError> {
        use std::os::unix::fs::OpenOptionsExt;
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().map_err(io_error)?.join(path)
        };
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/")
            .map_err(io_error)?;
        let root = Self { file };
        let rel = absolute
            .strip_prefix("/")
            .map_err(|_| ManagerError::new(ErrorCode::InvalidPath, "Invalid absolute path"))?;
        if rel.as_os_str().is_empty() {
            return Ok(root);
        }
        let (directory, _) = root.parent(&rel.join(".directory-sentinel"), false)?;
        Ok(directory)
    }
    pub fn parent(
        &self,
        rel: &Path,
        create: bool,
    ) -> Result<(Self, std::ffi::CString), ManagerError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        };
        relative(rel)?;
        let mut current = Self {
            file: self.file.try_clone().map_err(io_error)?,
        };
        for part in rel.parent().unwrap_or(Path::new("")).components() {
            let name = std::ffi::CString::new(part.as_os_str().as_bytes())
                .map_err(|_| ManagerError::new(ErrorCode::InvalidPath, "NUL in path"))?;
            let mut fd = unsafe {
                libc::openat(
                    current.file.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0
                && create
                && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound
            {
                if unsafe { libc::mkdirat(current.file.as_raw_fd(), name.as_ptr(), 0o700) } < 0 {
                    return Err(io_error(std::io::Error::last_os_error()));
                }
                fd = unsafe {
                    libc::openat(
                        current.file.as_raw_fd(),
                        name.as_ptr(),
                        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    )
                };
            }
            if fd < 0 {
                return Err(ManagerError::new(
                    ErrorCode::InvalidPath,
                    format!(
                        "Cannot open contained parent: {}",
                        std::io::Error::last_os_error()
                    ),
                ));
            }
            current = Self {
                file: unsafe { fs::File::from_raw_fd(fd) },
            };
        }
        let name = std::ffi::CString::new(rel.file_name().unwrap().as_bytes())
            .map_err(|_| ManagerError::new(ErrorCode::InvalidPath, "NUL in path"))?;
        Ok((current, name))
    }
    pub(crate) fn entry_names(
        &self,
        limit: usize,
    ) -> Result<Vec<std::ffi::OsString>, ManagerError> {
        use std::os::fd::AsRawFd;
        let mut names = Vec::new();
        for entry in fs::read_dir(format!("/proc/self/fd/{}", self.file.as_raw_fd()))
            .map_err(io_error)?
            .take(limit + 1)
        {
            names.push(entry.map_err(io_error)?.file_name());
        }
        if names.len() > limit {
            return Err(ManagerError::new(
                ErrorCode::InvalidPath,
                format!("Directory exceeds the {limit}-entry scan limit"),
            ));
        }
        names.sort();
        Ok(names)
    }
    pub(crate) fn open_regular(&self, relative: &Path) -> Result<fs::File, ManagerError> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let (parent, name) = self.parent(relative, false)?;
        let fd = unsafe {
            libc::openat(
                parent.file.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        let file = unsafe { fs::File::from_raw_fd(fd) };
        if !file.metadata().map_err(io_error)?.is_file() {
            return Err(ManagerError::new(
                ErrorCode::InvalidPath,
                "Download is not a regular file",
            ));
        }
        Ok(file)
    }

    pub fn create_file(&self, relative: &Path) -> Result<fs::File, ManagerError> {
        use std::os::fd::{AsRawFd, FromRawFd};
        let (parent, name) = self.parent(relative, true)?;
        let fd = unsafe {
            libc::openat(
                parent.file.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(unsafe { fs::File::from_raw_fd(fd) })
    }
    pub fn symlink(&self, rel: &Path, target: &Path) -> Result<(), ManagerError> {
        use std::os::{fd::AsRawFd, unix::ffi::OsStrExt};
        let (parent, name) = self.parent(rel, true)?;
        let target = std::ffi::CString::new(target.as_os_str().as_bytes())
            .map_err(|_| ManagerError::new(ErrorCode::InvalidPath, "NUL in target"))?;
        if unsafe { libc::symlinkat(target.as_ptr(), parent.file.as_raw_fd(), name.as_ptr()) } != 0
        {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        Ok(())
    }
    pub fn unlink_owned(&self, rel: &Path, target: &Path) -> Result<(), ManagerError> {
        use std::os::{fd::AsRawFd, unix::ffi::OsStrExt};
        let (parent, name) = self.parent(rel, false)?;
        let mut buffer = vec![0u8; 65536];
        let count = unsafe {
            libc::readlinkat(
                parent.file.as_raw_fd(),
                name.as_ptr(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
            )
        };
        if count < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        let actual = Path::new(std::ffi::OsStr::from_bytes(&buffer[..count as usize]));
        if actual != target && (!actual.is_absolute() || normalize(actual) != normalize(target)) {
            return Err(ManagerError::new(
                ErrorCode::ExternalLink,
                "Link target changed; preserving replacement",
            ));
        }
        // readlinkat and unlinkat share a held parent descriptor. A concurrent leaf replacement
        // still requires coordination with external writers; the core lock serializes app writers.
        if unsafe { libc::unlinkat(parent.file.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        parent.file.sync_all().map_err(io_error)
    }
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ManagerError> {
    let parent = path
        .parent()
        .ok_or_else(|| ManagerError::new(ErrorCode::InvalidPath, "Missing parent"))?;
    let temp_name = format!(".write-{}", uuid::Uuid::new_v4());
    #[cfg(unix)]
    {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        };
        let directory = Directory::open(parent)?;
        let source = std::ffi::CString::new(temp_name).unwrap();
        let dest = std::ffi::CString::new(path.file_name().unwrap().as_bytes())
            .map_err(|_| ManagerError::new(ErrorCode::InvalidPath, "NUL in path"))?;
        let fd = unsafe {
            libc::openat(
                directory.file.as_raw_fd(),
                source.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        let mut file = unsafe { fs::File::from_raw_fd(fd) };
        let result = (|| {
            file.write_all(bytes).map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
            if unsafe {
                libc::renameat(
                    directory.file.as_raw_fd(),
                    source.as_ptr(),
                    directory.file.as_raw_fd(),
                    dest.as_ptr(),
                )
            } < 0
            {
                return Err(io_error(std::io::Error::last_os_error()));
            }
            directory.file.sync_all().map_err(io_error)
        })();
        if result.is_err() {
            unsafe {
                libc::unlinkat(directory.file.as_raw_fd(), source.as_ptr(), 0);
            }
        }
        result
    }
    #[cfg(not(unix))]
    {
        let temp = parent.join(temp_name);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(io_error)?;
        file.write_all(bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        fs::rename(&temp, path).map_err(io_error)
    }
}
pub fn rename_no_replace(source: &Path, destination: &Path) -> Result<(), ManagerError> {
    #[cfg(target_os = "linux")]
    {
        use std::os::{fd::AsRawFd, unix::ffi::OsStrExt};
        let from =
            Directory::open(source.parent().ok_or_else(|| {
                ManagerError::new(ErrorCode::InvalidPath, "Missing source parent")
            })?)?;
        let to = Directory::open(destination.parent().ok_or_else(|| {
            ManagerError::new(ErrorCode::InvalidPath, "Missing destination parent")
        })?)?;
        let from_name = std::ffi::CString::new(source.file_name().unwrap().as_bytes())
            .map_err(|_| ManagerError::new(ErrorCode::InvalidPath, "NUL"))?;
        let to_name = std::ffi::CString::new(destination.file_name().unwrap().as_bytes())
            .map_err(|_| ManagerError::new(ErrorCode::InvalidPath, "NUL"))?;
        if unsafe {
            libc::renameat2(
                from.file.as_raw_fd(),
                from_name.as_ptr(),
                to.file.as_raw_fd(),
                to_name.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        } < 0
        {
            let error = std::io::Error::last_os_error();
            return Err(ManagerError::new(
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    ErrorCode::PathCollision
                } else {
                    ErrorCode::IoError
                },
                error.to_string(),
            ));
        }
        from.file.sync_all().map_err(io_error)?;
        to.file.sync_all().map_err(io_error)
    }
    #[cfg(not(target_os = "linux"))]
    {
        if exists(destination) {
            return Err(ManagerError::new(
                ErrorCode::PathCollision,
                "Destination exists",
            ));
        }
        fs::rename(source, destination).map_err(io_error)
    }
}

pub fn normalize(path: &Path) -> std::path::PathBuf {
    let mut result = std::path::PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => (),
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    result
}

pub fn create_file(root: &Path, relative: &Path) -> Result<fs::File, ManagerError> {
    Directory::open(root)?.create_file(relative)
}

pub fn create_dir_all(path: &Path) -> Result<(), ManagerError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_err(io_error)?.join(path)
    };
    let relative = absolute
        .strip_prefix("/")
        .map_err(|_| ManagerError::new(ErrorCode::InvalidPath, "Expected absolute directory"))?;
    if relative.as_os_str().is_empty() {
        return Ok(());
    }
    Directory::open(Path::new("/"))?.parent(&relative.join(".sentinel"), true)?;
    Ok(())
}

#[cfg(not(unix))]
pub struct Directory;
#[cfg(not(unix))]
impl Directory {
    pub fn open(_path: &Path) -> Result<Self, ManagerError> {
        Err(ManagerError::new(
            ErrorCode::InternalError,
            "Descriptor-relative filesystem mutations are unsupported on this platform",
        ))
    }
    pub fn parent(
        &self,
        _relative: &Path,
        _create: bool,
    ) -> Result<(Self, std::ffi::CString), ManagerError> {
        Self::open(Path::new(".")).map(|d| (d, std::ffi::CString::new("unsupported").unwrap()))
    }
    pub fn symlink(&self, _relative: &Path, _target: &Path) -> Result<(), ManagerError> {
        Self::open(Path::new(".")).map(|_| ())
    }
    pub fn unlink_owned(&self, _relative: &Path, _target: &Path) -> Result<(), ManagerError> {
        Self::open(Path::new(".")).map(|_| ())
    }
    pub fn create_file(&self, _relative: &Path) -> Result<fs::File, ManagerError> {
        Err(ManagerError::new(
            ErrorCode::InternalError,
            "Descriptor-relative file creation is unsupported on this platform",
        ))
    }
}

pub fn create_directory(path: &Path) -> Result<(), ManagerError> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        let directory = Directory::open(path.parent().ok_or_else(|| {
            ManagerError::new(ErrorCode::InvalidPath, "Missing directory parent")
        })?)?;
        let (parent, name) = directory.parent(
            Path::new(path.file_name().ok_or_else(|| {
                ManagerError::new(ErrorCode::InvalidPath, "Missing directory name")
            })?),
            false,
        )?;
        if unsafe { libc::mkdirat(parent.file.as_raw_fd(), name.as_ptr(), 0o700) } < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        parent.file.sync_all().map_err(io_error)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(ManagerError::new(
            ErrorCode::InternalError,
            "Descriptor-relative directory creation is unsupported on this platform",
        ))
    }
}
