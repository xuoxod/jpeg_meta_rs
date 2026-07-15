use std::path::Path;
use crate::error::ValidationError;

/// Default maximum allowed file size threshold (200 MB) to prevent Out-Of-Memory (OOM) situations.
pub const DEFAULT_MAX_FILE_SIZE: u64 = 200 * 1024 * 1024;

/// Validates that a path exists, is a regular file, is not empty, is readable, and does not exceed the default maximum allowed size.
pub fn validate_file_path(path: &Path) -> Result<(), ValidationError> {
    validate_file_path_with_limit(path, DEFAULT_MAX_FILE_SIZE)
}

/// Validates that a path exists, is a regular file, is not empty, is readable, and does not exceed a specified maximum size.
pub fn validate_file_path_with_limit(path: &Path, max_size: u64) -> Result<(), ValidationError> {
    if !path.exists() {
        return Err(ValidationError::PathNotFound(path.to_string_lossy().into_owned()));
    }
    if !path.is_file() {
        return Err(ValidationError::NotAFile(path.to_string_lossy().into_owned()));
    }
    let metadata = std::fs::metadata(path)
        .map_err(|_| ValidationError::NotReadable(path.to_string_lossy().into_owned()))?;
    
    let size = metadata.len();
    if size == 0 {
        return Err(ValidationError::EmptyFile(path.to_string_lossy().into_owned()));
    }
    if size > max_size {
        return Err(ValidationError::FileTooLarge {
            path: path.to_string_lossy().into_owned(),
            max_size,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_validate_file_path() {
        let dir = tempdir().unwrap();
        
        // Non-existent path
        let non_existent = dir.path().join("does_not_exist.jpg");
        assert!(matches!(validate_file_path(&non_existent).unwrap_err(), ValidationError::PathNotFound(_)));

        // Is directory, not file
        assert!(matches!(validate_file_path(dir.path()).unwrap_err(), ValidationError::NotAFile(_)));

        // Empty file
        let empty_file_path = dir.path().join("empty.jpg");
        File::create(&empty_file_path).unwrap();
        assert!(matches!(validate_file_path(&empty_file_path).unwrap_err(), ValidationError::EmptyFile(_)));

        // File too large
        let large_file_path = dir.path().join("large.jpg");
        std::fs::write(&large_file_path, b"some content here").unwrap();
        assert!(matches!(
            validate_file_path_with_limit(&large_file_path, 5).unwrap_err(),
            ValidationError::FileTooLarge { max_size: 5, .. }
        ));

        // Valid file
        let valid_file_path = dir.path().join("valid.jpg");
        std::fs::write(&valid_file_path, b"test content").unwrap();
        assert!(validate_file_path(&valid_file_path).is_ok());
    }
}
