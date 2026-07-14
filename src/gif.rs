#![allow(clippy::field_reassign_with_default, clippy::collapsible_if)]
use crate::common::ExifMetadata;
use crate::error::ParseError;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GifBlock {
    pub block_type: String,
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GifInfo {
    pub blocks: Vec<GifBlock>,
    pub width: u16,
    pub height: u16,
    pub comment: Option<String>,
    pub metadata: ExifMetadata,
}

pub fn parse_gif(bytes: &[u8]) -> Result<GifInfo, ParseError> {
    if bytes.len() < 13 || (&bytes[0..6] != b"GIF87a" && &bytes[0..6] != b"GIF89a") {
        return Err(ParseError::InvalidFormat("Missing or invalid GIF signature".to_string()));
    }

    let mut blocks = Vec::new();
    let mut comments = Vec::new();
    let mut raw_xmp_payload = None;

    let width = u16::from_le_bytes([bytes[6], bytes[7]]);
    let height = u16::from_le_bytes([bytes[8], bytes[9]]);

    let packed = bytes[10];
    let has_gct = (packed & 0x80) != 0;
    let gct_size_indicator = packed & 0x07;
    let gct_len = if has_gct { 3 * (1 << (gct_size_indicator + 1)) } else { 0 };

    let mut pos = 13 + gct_len;

    while pos < bytes.len() {
        let block_start = pos;
        let sentinel = bytes[pos];
        pos += 1;

        match sentinel {
            0x2C => {
                // Image Descriptor
                if pos + 9 > bytes.len() {
                    break;
                }
                let packed_local = bytes[pos + 8];
                pos += 9;
                let has_lct = (packed_local & 0x80) != 0;
                let lct_size_indicator = packed_local & 0x07;
                let lct_len = if has_lct { 3 * (1 << (lct_size_indicator + 1)) } else { 0 };
                pos += lct_len;

                // LZW Minimum Code Size
                if pos >= bytes.len() {
                    break;
                }
                pos += 1;

                // Data Sub-blocks
                while pos < bytes.len() {
                    let sub_len = bytes[pos] as usize;
                    pos += 1;
                    if sub_len == 0 {
                        break;
                    }
                    pos += sub_len;
                }

                blocks.push(GifBlock {
                    block_type: "Image Descriptor".to_string(),
                    offset: block_start,
                    length: pos - block_start,
                });
            }
            0x21 => {
                // Extension
                if pos >= bytes.len() {
                    break;
                }
                let label = bytes[pos];
                pos += 1;

                let block_type_name = match label {
                    0xFE => "Comment Extension".to_string(),
                    0xFF => "Application Extension".to_string(),
                    0xF9 => "Graphics Control Extension".to_string(),
                    0x01 => "Plain Text Extension".to_string(),
                    other => format!("Extension (0x{other:02X})"),
                };

                let mut extension_data = Vec::new();
                let mut app_id = [0u8; 11];
                let mut is_xmp = false;

                if label == 0xFF {
                    if pos < bytes.len() {
                        let app_len = bytes[pos] as usize;
                        if app_len == 11 && pos + 12 <= bytes.len() {
                            app_id.copy_from_slice(&bytes[pos + 1..pos + 12]);
                            if &app_id[0..8] == b"XMP Data" {
                                is_xmp = true;
                            }
                        }
                    }
                }

                while pos < bytes.len() {
                    let sub_len = bytes[pos] as usize;
                    pos += 1;
                    if sub_len == 0 {
                        break;
                    }
                    if pos + sub_len > bytes.len() {
                        pos = bytes.len();
                        break;
                    }
                    let sub_payload = &bytes[pos..pos + sub_len];
                    extension_data.extend_from_slice(sub_payload);
                    pos += sub_len;
                }

                if label == 0xFE {
                    if let Ok(comment_str) = String::from_utf8(extension_data) {
                        comments.push(comment_str.trim().to_string());
                    }
                } else if is_xmp {
                    raw_xmp_payload = String::from_utf8(extension_data).ok();
                }

                blocks.push(GifBlock {
                    block_type: block_type_name,
                    offset: block_start,
                    length: pos - block_start,
                });
            }
            0x3B => {
                // Trailer
                blocks.push(GifBlock {
                    block_type: "Trailer".to_string(),
                    offset: block_start,
                    length: 1,
                });
                break;
            }
            _ => {
                break;
            }
        }
    }

    let comment = if comments.is_empty() { None } else { Some(comments.join("; ")) };
    let mut metadata = ExifMetadata::default();
    metadata.xmp = raw_xmp_payload;

    Ok(GifInfo {
        blocks,
        width,
        height,
        comment,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gif_invalid() {
        assert!(parse_gif(b"not gif data").is_err());
    }

    #[test]
    fn test_parse_gif_valid() {
        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&400u16.to_le_bytes());
        gif.extend_from_slice(&300u16.to_le_bytes());
        gif.push(0);
        gif.push(0);
        gif.push(0);

        gif.extend_from_slice(&[0x21, 0xFE, 11]);
        gif.extend_from_slice(b"Hello World");
        gif.push(0);

        gif.extend_from_slice(&[0x2C]);
        gif.extend_from_slice(&0u16.to_le_bytes());
        gif.extend_from_slice(&0u16.to_le_bytes());
        gif.extend_from_slice(&400u16.to_le_bytes());
        gif.extend_from_slice(&300u16.to_le_bytes());
        gif.push(0);
        gif.push(8);
        gif.push(0);

        gif.push(0x3B);

        let info = parse_gif(&gif).unwrap();
        assert_eq!(info.width, 400);
        assert_eq!(info.height, 300);
        assert_eq!(info.comment, Some("Hello World".to_string()));
        assert_eq!(info.blocks.len(), 3);
        assert_eq!(info.blocks[0].block_type, "Comment Extension");
        assert_eq!(info.blocks[1].block_type, "Image Descriptor");
        assert_eq!(info.blocks[2].block_type, "Trailer");
    }
}
