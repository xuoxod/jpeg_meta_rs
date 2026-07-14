use std::path::Path;
use crate::error::ValidationError;

/// Expected signature constants
pub const PNG_SIGNATURE: &[u8; 8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
pub const JPEG_SIGNATURE: &[u8; 2] = &[0xFF, 0xD8];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedType {
    Jpeg,
    Png,
}

/// Detects and validates the file type using magic bytes, falling back to file extension.
pub fn detect_file_type(bytes: &[u8], path: &Path) -> Result<DetectedType, ValidationError> {
    if bytes.len() >= 8 && &bytes[0..8] == PNG_SIGNATURE {
        return Ok(DetectedType::Png);
    }
    if bytes.len() >= 2 && &bytes[0..2] == JPEG_SIGNATURE {
        return Ok(DetectedType::Jpeg);
    }

    // Fallback to extension check
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    match ext.as_deref() {
        Some("png") => Ok(DetectedType::Png),
        Some("jpg") | Some("jpeg") => Ok(DetectedType::Jpeg),
        _ => Err(ValidationError::InvalidSignature {
            path: path.to_string_lossy().into_owned(),
            expected: "JPEG or PNG magic bytes/extension".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_file_type() {
        let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];
        assert_eq!(detect_file_type(&png_bytes, Path::new("test.png")).unwrap(), DetectedType::Png);

        let jpeg_bytes = [0xFF, 0xD8, 0, 0];
        assert_eq!(detect_file_type(&jpeg_bytes, Path::new("test.jpg")).unwrap(), DetectedType::Jpeg);

        let fallback_png = [0, 0, 0, 0];
        assert_eq!(detect_file_type(&fallback_png, Path::new("test.png")).unwrap(), DetectedType::Png);

        let err = detect_file_type(&fallback_png, Path::new("test.txt")).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidSignature { .. }));
    }
}
