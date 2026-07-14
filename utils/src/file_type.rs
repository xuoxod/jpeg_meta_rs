use std::path::Path;
use crate::error::ValidationError;

/// Expected signature constants
pub const PNG_SIGNATURE: &[u8; 8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
pub const JPEG_SIGNATURE: &[u8; 2] = &[0xFF, 0xD8];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedType {
    Jpeg,
    Png,
    Webp,
    Gif,
    Heic,
}

/// Detects and validates the file type using magic bytes, falling back to file extension.
pub fn detect_file_type(bytes: &[u8], path: &Path) -> Result<DetectedType, ValidationError> {
    if bytes.len() >= 8 && &bytes[0..8] == PNG_SIGNATURE {
        return Ok(DetectedType::Png);
    }
    if bytes.len() >= 2 && &bytes[0..2] == JPEG_SIGNATURE {
        return Ok(DetectedType::Jpeg);
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok(DetectedType::Webp);
    }
    if bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        return Ok(DetectedType::Gif);
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"heic" || brand == b"heix" || brand == b"hevc" || brand == b"mif1" || brand == b"msf1" {
            return Ok(DetectedType::Heic);
        }
    }

    // Fallback to extension check
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());

    match ext.as_deref() {
        Some("png") => Ok(DetectedType::Png),
        Some("jpg") | Some("jpeg") => Ok(DetectedType::Jpeg),
        Some("webp") => Ok(DetectedType::Webp),
        Some("gif") => Ok(DetectedType::Gif),
        Some("heic") | Some("heif") => Ok(DetectedType::Heic),
        _ => Err(ValidationError::InvalidSignature {
            path: path.to_string_lossy().into_owned(),
            expected: "JPEG, PNG, WebP, GIF, or HEIC magic bytes/extension".to_string(),
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

        let webp_bytes = b"RIFF\0\0\0\0WEBPvp8x";
        assert_eq!(detect_file_type(webp_bytes, Path::new("test.webp")).unwrap(), DetectedType::Webp);

        let gif_bytes = b"GIF89a\0\0\0";
        assert_eq!(detect_file_type(gif_bytes, Path::new("test.gif")).unwrap(), DetectedType::Gif);

        let heic_bytes = b"\0\0\0\x18ftypheic\0\0\0\0";
        assert_eq!(detect_file_type(heic_bytes, Path::new("test.heic")).unwrap(), DetectedType::Heic);

        let fallback_webp = [0, 0, 0, 0];
        assert_eq!(detect_file_type(&fallback_webp, Path::new("test.webp")).unwrap(), DetectedType::Webp);

        let err = detect_file_type(&fallback_webp, Path::new("test.txt")).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidSignature { .. }));
    }
}
