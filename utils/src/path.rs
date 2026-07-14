use std::path::Path;
use crate::error::ValidationError;

/// Validates that a path exists, is a regular file, is not empty, and is readable.
pub fn validate_file_path(path: &Path) -> Result<(), ValidationError> {
    if !path.exists() {
        return Err(ValidationError::PathNotFound(path.to_string_lossy().into_owned()));
    }
    if !path.is_file() {
        return Err(ValidationError::NotAFile(path.to_string_lossy().into_owned()));
    }
    let metadata = std::fs::metadata(path)
        .map_err(|_| ValidationError::NotReadable(path.to_string_lossy().into_owned()))?;
    if metadata.len() == 0 {
        return Err(ValidationError::EmptyFile(path.to_string_lossy().into_owned()));
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

        // Valid file
        let valid_file_path = dir.path().join("valid.jpg");
        std::fs::write(&valid_file_path, b"test content").unwrap();
        assert!(validate_file_path(&valid_file_path).is_ok());
    }
}
