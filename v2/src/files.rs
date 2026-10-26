use crate::input::MAX_FILE;
#[cfg(unix)]
use std::fs::File;
#[cfg(not(unix))]
use std::fs::OpenOptions;
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

pub fn ordinary_path(path: &Path, directory: bool) -> Result<(), String> {
    if !path.is_absolute() || path.as_os_str().len() > 4096 {
        return Err("Choose an absolute path of at most 4096 bytes".into());
    }
    let mut walk = std::path::PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::ParentDir | Component::CurDir) {
            return Err("Non-normal path".into());
        }
        walk.push(component);
        let metadata = fs::symlink_metadata(&walk).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err(
                "Symbolic links are not supported; select the ordinary file directly".into(),
            );
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err("Reparse paths are not supported".into());
            }
        }
    }
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if (directory && !m.is_dir()) || (!directory && !m.is_file()) {
        return Err("Path must be an ordinary file/directory".into());
    }
    Ok(())
}
fn same(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if a.dev() != b.dev()
            || a.ino() != b.ino()
            || a.ctime() != b.ctime()
            || a.ctime_nsec() != b.ctime_nsec()
        {
            return false;
        }
    }
    a.is_file() == b.is_file() && a.len() == b.len() && a.modified().ok() == b.modified().ok()
}
#[cfg(unix)]
fn open_confined(path: &Path, directory: bool) -> Result<File, String> {
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    };
    if !path.is_absolute() {
        return Err("Expected absolute path".into());
    }
    let mut current = File::open("/").map_err(|e| e.to_string())?;
    let parts: Vec<_> = path
        .components()
        .filter(|c| !matches!(c, Component::RootDir))
        .collect();
    for (n, part) in parts.iter().enumerate() {
        let Component::Normal(name) = part else {
            return Err("Non-normal path".into());
        };
        let name = std::ffi::CString::new(name.as_bytes()).map_err(|e| e.to_string())?;
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if directory || n + 1 < parts.len() {
                libc::O_DIRECTORY
            } else {
                0
            };
        // Each component is opened relative to its already validated directory.
        let fd = unsafe { libc::openat(current.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        current = unsafe { File::from_raw_fd(fd) };
    }
    Ok(current)
}
fn read_inner(path: &Path) -> Result<Vec<u8>, String> {
    ordinary_path(path, false)?;
    let before = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if before.len() > MAX_FILE as u64 {
        return Err("Selected file exceeds 8,000,000 bytes".into());
    }
    #[cfg(unix)]
    let mut file = open_confined(path, false)?;
    #[cfg(not(unix))]
    let mut file = {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x00200000);
        }
        options.open(path).map_err(|e| e.to_string())?
    };
    let opened = file.metadata().map_err(|e| e.to_string())?;
    if !opened.is_file() || !same(&before, &opened) {
        return Err("File changed while opening".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_FILE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    ordinary_path(path, false)?;
    if bytes.len() > MAX_FILE
        || bytes.len() as u64 != before.len()
        || !same(&before, &file.metadata().map_err(|e| e.to_string())?)
        || !same(
            &before,
            &fs::symlink_metadata(path).map_err(|e| e.to_string())?,
        )
    {
        return Err("File changed during conversion".into());
    }
    Ok(bytes)
}

pub fn read_selected(path: &Path) -> Result<Vec<u8>, String> {
    read_inner(path).map_err(|_| "Select an unchanged ordinary absolute file, without symlinks, of at most 8,000,000 bytes.".into())
}
