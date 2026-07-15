use crate::error::ValidationError;

/// Creates a raw JPEG COM (Comment) segment.
fn create_jpeg_com_segment(comment: &str) -> Vec<u8> {
    let payload = comment.as_bytes();
    let len = (payload.len() + 2) as u16;
    let mut segment = Vec::new();
    segment.extend_from_slice(&[0xFF, 0xFE]);
    segment.extend_from_slice(&len.to_be_bytes());
    segment.extend_from_slice(payload);
    segment
}

/// Creates a raw PNG tEXt metadata chunk.
fn create_png_text_chunk(keyword: &str, value: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(keyword.as_bytes());
    payload.push(0); // null separator
    payload.extend_from_slice(value.as_bytes());

    let len = payload.len() as u32;
    let type_bytes = b"tEXt";

    let mut chunk = Vec::new();
    chunk.extend_from_slice(&len.to_be_bytes());
    chunk.extend_from_slice(type_bytes);
    chunk.extend_from_slice(&payload);

    // Calculate CRC32 over chunk type and payload
    let mut crc_input = Vec::new();
    crc_input.extend_from_slice(type_bytes);
    crc_input.extend_from_slice(&payload);
    let crc = crate::crc::crc32(&crc_input);

    chunk.extend_from_slice(&crc.to_be_bytes());
    chunk
}

/// Decoupled, reusable utility to set (insert/replace) or delete a JPEG COM comment.
/// If `new_comment` is `Some`, inserts/replaces the comment. If `None`, deletes it.
pub fn edit_jpeg_comment(bytes: &[u8], new_comment: Option<&str>) -> Result<Vec<u8>, ValidationError> {
    if bytes.len() < 2 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err(ValidationError::InvalidSignature {
            path: "in-memory-jpeg".to_string(),
            expected: "JPEG signature".to_string(),
        });
    }

    let mut out = Vec::new();
    out.extend_from_slice(&[0xFF, 0xD8]); // Write SOI

    // If setting a new comment, write it immediately after SOI
    if let Some(comment) = new_comment {
        out.extend_from_slice(&create_jpeg_com_segment(comment));
    }

    let mut pos = 2;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }

        let marker = bytes[pos + 1];
        if marker == 0xD9 {
            out.extend_from_slice(&[0xFF, 0xD9]);
            break;
        }

        if marker == 0xDA {
            out.extend_from_slice(&bytes[pos..]);
            break;
        }

        let len = u16::from_be_bytes([bytes[pos + 2], bytes[pos + 3]]) as usize;
        if pos + 2 + len > bytes.len() {
            return Err(ValidationError::EmptyFile("in-memory-jpeg".to_string()));
        }

        let payload_start = pos;
        let payload_len = 2 + len;

        // Skip any existing COM segments (0xFE)
        if marker != 0xFE {
            out.extend_from_slice(&bytes[payload_start..payload_start + payload_len]);
        }

        pos += payload_len;
    }

    Ok(out)
}

/// Decoupled, reusable utility to set (insert/replace) or delete a PNG tEXt/iTXt chunk by keyword.
/// If `new_value` is `Some`, inserts/replaces the keyword value. If `None`, deletes it.
pub fn edit_png_text(bytes: &[u8], keyword: &str, new_value: Option<&str>) -> Result<Vec<u8>, ValidationError> {
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

        let chunk_len = 12 + len;

        if chunk_type == "IEND" {
            // Write the new text chunk right before IEND if provided
            if let Some(val) = new_value {
                out.extend_from_slice(&create_png_text_chunk(keyword, val));
            }
            out.extend_from_slice(&bytes[pos..pos + chunk_len]);
            break;
        }

        // Check if this chunk is a matching text chunk to drop/replace
        let should_drop = if chunk_type == "tEXt" {
            let payload = &bytes[pos + 8..pos + 8 + len];
            if let Some(null_pos) = payload.iter().position(|&b| b == 0) {
                let kw_str = String::from_utf8_lossy(&payload[0..null_pos]);
                kw_str == keyword
            } else {
                false
            }
        } else if chunk_type == "iTXt" {
            let payload = &bytes[pos + 8..pos + 8 + len];
            if let Some(null_pos) = payload.iter().position(|&b| b == 0) {
                let kw_str = String::from_utf8_lossy(&payload[0..null_pos]);
                kw_str == keyword
            } else {
                false
            }
        } else {
            false
        };

        if !should_drop {
            out.extend_from_slice(&bytes[pos..pos + chunk_len]);
        }

        pos += chunk_len;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_jpeg_comment() {
        // Construct mock JPEG with APP0 and existing COM
        let mut jpeg = vec![0xFF, 0xD8]; // SOI
        
        // APP0 (length 2 + 3 = 5)
        jpeg.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x05, 0x01, 0x02, 0x03]);
        // COM (length 2 + 11 = 13)
        jpeg.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x0D]);
        jpeg.extend_from_slice(b"old_comment");
        // DQT (length 2 + 2 = 4)
        jpeg.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x04, 0xAA, 0xBB]);
        // EOI
        jpeg.extend_from_slice(&[0xFF, 0xD9]);

        // 1. Set a new comment
        let edited = edit_jpeg_comment(&jpeg, Some("new_comment_value")).unwrap();
        // Parse segments to verify
        assert!(edited.windows(2).any(|w| w == [0xFF, 0xFE]));
        
        // 2. Delete comment
        let deleted = edit_jpeg_comment(&edited, None).unwrap();
        assert!(!deleted.windows(2).any(|w| w == [0xFF, 0xFE]));
    }

    #[test]
    fn test_edit_png_text() {
        // Construct mock PNG
        let signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let mut png = signature.to_vec();
        
        // IHDR
        png.extend_from_slice(&0u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&[0, 0, 0, 0]); // CRC

        // tEXt chunk: Author\0Rick
        let t_chunk = create_png_text_chunk("Author", "Rick");
        png.extend_from_slice(&t_chunk);

        // IEND
        png.extend_from_slice(&0u32.to_be_bytes());
        png.extend_from_slice(b"IEND");
        png.extend_from_slice(&[0, 0, 0, 0]); // CRC

        // 1. Update Keyword
        let edited = edit_png_text(&png, "Author", Some("Walker")).unwrap();
        // Verify Walk exists
        assert!(edited.windows(4).any(|w| w == b"tEXt"));

        // 2. Delete Keyword
        let deleted = edit_png_text(&edited, "Author", None).unwrap();
        assert!(!deleted.windows(4).any(|w| w == b"tEXt"));
    }
}
