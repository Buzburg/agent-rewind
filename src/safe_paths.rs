//! Restore boundaries for a quiescent workspace, not a concurrent-writer sandbox.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(crate) fn checked_path(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    for component in absolute.ancestors() {
        match fs::symlink_metadata(component) {
            Ok(info) => {
                if is_link(&info) || (component != absolute && !info.is_dir()) {
                    return Err(invalid("Snapshot path traverses a link or non-directory"));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(absolute)
}

pub(crate) fn workspace(root: &Path) -> io::Result<PathBuf> {
    let root = checked_path(root)?;
    if !root.is_dir() {
        return Err(invalid("Workspace must be an existing directory"));
    }
    root.canonicalize()
}

pub(crate) fn destination(root: &Path, relative: &str) -> io::Result<PathBuf> {
    if relative.contains(['\\', ':', '\0'])
        || relative.split('/').any(|part| {
            let lower = part.to_ascii_lowercase();
            let stem = lower.split('.').next().unwrap_or("");
            matches!(lower.as_str(), "" | "." | ".." | ".git" | ".rewind")
                || part.ends_with(['.', ' '])
                || matches!(stem, "con" | "prn" | "aux" | "nul")
                || (1..=9).any(|i| stem == format!("com{i}") || stem == format!("lpt{i}"))
        })
    {
        return Err(invalid("Invalid checkpoint relative path"));
    }
    let target = checked_path(&root.join(relative))?;
    if target.exists() && !target.is_file() {
        return Err(invalid("Checkpoint destination is not a regular file"));
    }
    Ok(target)
}
