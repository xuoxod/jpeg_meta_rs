use std::fs;
use std::path::{Path, PathBuf};
use crate::error::ValidationError;

/// Returns the user's default Pictures directory based on platform conventions.
pub fn get_default_pictures_dir() -> Option<PathBuf> {
    let home = if cfg!(target_os = "windows") {
        std::env::var_os("USERPROFILE")
            .or_else(|| {
                let drive = std::env::var_os("HOMEDRIVE")?;
                let path = std::env::var_os("HOMEPATH")?;
                let mut base = std::ffi::OsString::new();
                base.push(drive);
                base.push(path);
                Some(base)
            })
    } else {
        std::env::var_os("HOME")
    };
    home.map(|h| PathBuf::from(h).join("Pictures"))
}

/// Copies a file from source to destination. Creates parent directories if missing.
pub fn copy_file(source: &Path, destination: &Path) -> Result<u64, ValidationError> {
    if let Some(parent) = destination.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|_| ValidationError::PathNotFound(
            parent.to_string_lossy().into_owned()
        ))?;
    }
    fs::copy(source, destination).map_err(|_| ValidationError::PathNotFound(
        source.to_string_lossy().into_owned()
    ))
}

/// Copies a file from source to destination, truncating any trailing data beyond the logical container size.
pub fn copy_scrubbed_file(source: &Path, destination: &Path, official_end_offset: usize) -> Result<(), ValidationError> {
    let bytes = fs::read(source).map_err(|_| ValidationError::PathNotFound(
        source.to_string_lossy().into_owned()
    ))?;

    let scrubbed_bytes = if official_end_offset < bytes.len() {
        &bytes[0..official_end_offset]
    } else {
        &bytes[..]
    };

    if let Some(parent) = destination.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|_| ValidationError::PathNotFound(
            parent.to_string_lossy().into_owned()
        ))?;
    }

    fs::write(destination, scrubbed_bytes).map_err(|_| ValidationError::PathNotFound(
        destination.to_string_lossy().into_owned()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_get_default_pictures_dir() {
        let dir = get_default_pictures_dir();
        if let Some(path) = dir {
            assert!(path.to_string_lossy().contains("Pictures"));
        }
    }

    #[test]
    fn test_copy_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dest = dir.path().join("subdir/dest.txt");

        fs::write(&src, b"hello world").unwrap();
        let copied = copy_file(&src, &dest).unwrap();
        assert_eq!(copied, 11);
        assert_eq!(fs::read_to_string(&dest).unwrap(), "hello world");
    }

    #[test]
    fn test_copy_scrubbed_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&src, b"original_payload_trailing_garbage").unwrap();
        copy_scrubbed_file(&src, &dest, 16).unwrap();
        assert_eq!(fs::read_to_string(&dest).unwrap(), "original_payload");
    }
}
