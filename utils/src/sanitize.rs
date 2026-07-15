use crate::error::ValidationError;

/// Sanitizes a JPEG by removing all COM (comment) and APP1..APP15 (EXIF, XMP, ICC) segments.
/// Keeps SOI, APP0 (JFIF), DQT, DHT, SOF, SOS, and EOI.
pub fn sanitize_jpeg_bytes(bytes: &[u8]) -> Result<Vec<u8>, ValidationError> {
    if bytes.len() < 2 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err(ValidationError::InvalidSignature {
            path: "in-memory-jpeg".to_string(),
            expected: "JPEG signature".to_string(),
        });
    }

    let mut out = Vec::new();
    out.extend_from_slice(&[0xFF, 0xD8]); // Write SOI

    let mut pos = 2;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            // Scan forward for next marker
            pos += 1;
            continue;
        }

        let marker = bytes[pos + 1];
        if marker == 0xD9 {
            // EOI
            out.extend_from_slice(&[0xFF, 0xD9]);
            break;
        }

        if marker == 0xDA {
            // SOS: Image data follows. We copy the rest of the stream up to EOI.
            out.extend_from_slice(&bytes[pos..]);
            break;
        }

        let len = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
        if pos + 2 + len > bytes.len() {
            return Err(ValidationError::EmptyFile("in-memory-jpeg".to_string()));
        }

        let payload_start = pos;
        let payload_len = 2 + len; // Marker prefix (2) + length field and payload

        // Drop COM (0xFE) and APP1..APP15 (0xE1..0xEF)
        // Keep APP0 (0xE0) for decoder compatibility
        let should_drop = marker == 0xFE || (0xE1..=0xEF).contains(&marker);

        if !should_drop {
            out.extend_from_slice(&bytes[payload_start..payload_start + payload_len]);
        }

        pos += payload_len;
    }

    Ok(out)
}

/// Sanitizes a PNG by keeping only the critical chunks (IHDR, PLTE, IDAT, IEND)
/// and stripping all ancillary/private chunks (tEXt, iTXt, zTXt, tIME, pHYs, iCCP, eXIf).
pub fn sanitize_png_bytes(bytes: &[u8]) -> Result<Vec<u8>, ValidationError> {
    let signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 8 || bytes[0..8] != signature {
        return Err(ValidationError::InvalidSignature {
            path: "in-memory-png".to_string(),
            expected: "PNG signature".to_string(),
        });
    }

    let mut out = Vec::new();
    out.extend_from_slice(&signature);

    let mut pos = 8;
    while pos + 12 <= bytes.len() {
        let len = u32::from_be_bytes([
            bytes[pos],
            bytes[pos + 1],
            bytes[pos + 2],
            bytes[pos + 3],
        ]) as usize;

        let mut type_bytes = [0u8; 4];
        type_bytes.copy_from_slice(&bytes[pos + 4..pos + 8]);
        let chunk_type = String::from_utf8_lossy(&type_bytes).to_string();

        if pos + 12 + len > bytes.len() {
            return Err(ValidationError::EmptyFile("in-memory-png".to_string()));
        }

        // Only keep critical chunks
        let is_critical = chunk_type == "IHDR"
            || chunk_type == "PLTE"
            || chunk_type == "IDAT"
            || chunk_type == "IEND";

        if is_critical {
            let chunk_len = 12 + len;
            out.extend_from_slice(&bytes[pos..pos + chunk_len]);
        }

        if chunk_type == "IEND" {
            break;
        }

        pos += 12 + len;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_jpeg() {
        // Construct mock JPEG with APP1 (EXIF) and COM (Comment) segments
        let mut jpeg = vec![0xFF, 0xD8]; // SOI
        
        // APP0 (Keep)
        jpeg.extend_from_slice(&[0xFF, 0xE0, 0, 5, 1, 2, 3]);
        // APP1 (Drop)
        jpeg.extend_from_slice(&[0xFF, 0xE1, 0, 5, 9, 9, 9]);
        // COM (Drop)
        jpeg.extend_from_slice(&[0xFF, 0xFE, 0, 5, 8, 8, 8]);
        // SOS
        jpeg.extend_from_slice(&[0xFF, 0xDA, 0, 4, 1, 2, 0xAA, 0xBB]);
        // EOI
        jpeg.extend_from_slice(&[0xFF, 0xD9]);

        let sanitized = sanitize_jpeg_bytes(&jpeg).unwrap();
        // Check APP1 and COM are gone, SOI, APP0, SOS, and EOI are kept
        assert!(sanitized.windows(2).any(|w| w == [0xFF, 0xE0]));
        assert!(!sanitized.windows(2).any(|w| w == [0xFF, 0xE1]));
        assert!(!sanitized.windows(2).any(|w| w == [0xFF, 0xFE]));
        assert!(sanitized.windows(2).any(|w| w == [0xFF, 0xD9]));
    }

    #[test]
    fn test_sanitize_png() {
        // Construct mock PNG with IHDR, tEXt (Drop), and IEND
        let mut png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        
        // IHDR
        png.extend_from_slice(&0u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&0u32.to_be_bytes()); // CRC
        
        // tEXt
        png.extend_from_slice(&5u32.to_be_bytes());
        png.extend_from_slice(b"tEXt");
        png.extend_from_slice(b"hello");
        png.extend_from_slice(&0u32.to_be_bytes()); // CRC

        // IEND
        png.extend_from_slice(&0u32.to_be_bytes());
        png.extend_from_slice(b"IEND");
        png.extend_from_slice(&0u32.to_be_bytes()); // CRC

        let sanitized = sanitize_png_bytes(&png).unwrap();
        // tEXt should be gone
        let content_str = String::from_utf8_lossy(&sanitized);
        assert!(content_str.contains("IHDR"));
        assert!(content_str.contains("IEND"));
        assert!(!content_str.contains("tEXt"));
    }
}
