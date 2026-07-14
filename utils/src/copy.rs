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

/// Resolves the target destination path.
/// If `destination` is a directory, it joins it with the file name of `source`.
/// If the resolved path already exists (collision), it appends `_copy_1`, `_copy_2`, etc.
/// until it finds a unique, non-colliding file name.
pub fn resolve_destination_path(source: &Path, destination: &Path) -> PathBuf {
    let mut target = if destination.is_dir() {
        let file_name = source.file_name().unwrap_or_else(|| std::ffi::OsStr::new("image"));
        destination.join(file_name)
    } else {
        destination.to_path_buf()
    };

    if target.exists() {
        let parent = target.parent().unwrap_or_else(|| Path::new(""));
        let file_stem = target.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
        let extension = target.extension().and_then(|s| s.to_str()).unwrap_or("");

        let mut counter = 1;
        loop {
            let new_name = if extension.is_empty() {
                format!("{}_copy_{}", file_stem, counter)
            } else {
                format!("{}_copy_{}.{}", file_stem, counter, extension)
            };
            let candidate = parent.join(new_name);
            if !candidate.exists() {
                target = candidate;
                break;
            }
            counter += 1;
        }
    }

    target
}

/// Copies a file from source to destination. Creates parent directories if missing.
/// Resolves filename collisions automatically.
pub fn copy_file(source: &Path, destination: &Path) -> Result<PathBuf, ValidationError> {
    let resolved_dest = resolve_destination_path(source, destination);
    if let Some(parent) = resolved_dest.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|_| ValidationError::PathNotFound(
            parent.to_string_lossy().into_owned()
        ))?;
    }
    fs::copy(source, &resolved_dest).map_err(|_| ValidationError::PathNotFound(
        source.to_string_lossy().into_owned()
    ))?;
    Ok(resolved_dest)
}

/// Copies a file from source to destination, truncating any trailing data beyond the logical container size.
/// Resolves filename collisions automatically.
pub fn copy_scrubbed_file(source: &Path, destination: &Path, official_end_offset: usize) -> Result<PathBuf, ValidationError> {
    let resolved_dest = resolve_destination_path(source, destination);
    let bytes = fs::read(source).map_err(|_| ValidationError::PathNotFound(
        source.to_string_lossy().into_owned()
    ))?;

    let scrubbed_bytes = if official_end_offset < bytes.len() {
        &bytes[0..official_end_offset]
    } else {
        &bytes[..]
    };

    if let Some(parent) = resolved_dest.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|_| ValidationError::PathNotFound(
            parent.to_string_lossy().into_owned()
        ))?;
    }

    fs::write(&resolved_dest, scrubbed_bytes).map_err(|_| ValidationError::PathNotFound(
        resolved_dest.to_string_lossy().into_owned()
    ))?;
    Ok(resolved_dest)
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
    fn test_resolve_destination_path_collision() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("flower.png");
        let dest = dir.path().join("flower.png");

        fs::write(&src, b"abc").unwrap();
        fs::write(&dest, b"xyz").unwrap();

        // Target already exists, should resolve to flower_copy_1.png
        let resolved = resolve_destination_path(&src, &dest);
        assert_eq!(resolved.file_name().unwrap(), "flower_copy_1.png");

        // Write flower_copy_1.png so that it exists
        fs::write(&resolved, b"123").unwrap();

        // Target flower.png and flower_copy_1.png exist, should resolve to flower_copy_2.png
        let resolved_2 = resolve_destination_path(&src, &dest);
        assert_eq!(resolved_2.file_name().unwrap(), "flower_copy_2.png");
    }

    #[test]
    fn test_copy_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dest = dir.path().join("subdir/dest.txt");

        fs::write(&src, b"hello world").unwrap();
        let final_dest = copy_file(&src, &dest).unwrap();
        assert_eq!(final_dest, dest);
        assert_eq!(fs::read_to_string(&dest).unwrap(), "hello world");
    }

    #[test]
    fn test_copy_scrubbed_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        fs::write(&src, b"original_payload_trailing_garbage").unwrap();
        let final_dest = copy_scrubbed_file(&src, &dest, 16).unwrap();
        assert_eq!(final_dest, dest);
        assert_eq!(fs::read_to_string(&dest).unwrap(), "original_payload");
    }
}
