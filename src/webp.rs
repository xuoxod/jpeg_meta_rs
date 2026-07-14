#![allow(clippy::op_ref, clippy::manual_is_multiple_of)]
use crate::common::ExifMetadata;
use crate::error::ParseError;
use exif as kamadak_exif;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WebpChunk {
    pub tag: String,
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WebpInfo {
    pub chunks: Vec<WebpChunk>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub metadata: ExifMetadata,
    pub official_end_offset: usize,
}

pub fn parse_webp(bytes: &[u8]) -> Result<WebpInfo, ParseError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Err(ParseError::InvalidFormat("Missing or invalid WebP RIFF header".to_string()));
    }

    let mut chunks = Vec::new();
    let mut width = None;
    let mut height = None;
    let mut raw_exif_payload = None;
    let mut raw_xmp_payload = None;

    let riff_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let max_len = std::cmp::min(bytes.len(), riff_size + 8);
    let mut pos = 12;

    while pos + 8 <= max_len {
        let chunk_start = pos;
        let mut tag_bytes = [0u8; 4];
        tag_bytes.copy_from_slice(&bytes[pos..pos + 4]);
        let tag = String::from_utf8_lossy(&tag_bytes).to_string();
        pos += 4;

        let len = u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]) as usize;
        pos += 4;

        if pos + len > bytes.len() {
            break;
        }

        let payload = &bytes[pos..pos + len];

        chunks.push(WebpChunk {
            tag: tag.clone(),
            offset: chunk_start,
            length: len + 8,
        });

        match tag.as_str() {
            "VP8X" => {
                if len >= 10 {
                    let w = u32::from_le_bytes([payload[4], payload[5], payload[6], 0]) + 1;
                    let h = u32::from_le_bytes([payload[7], payload[8], payload[9], 0]) + 1;
                    width = Some(w);
                    height = Some(h);
                }
            }
            "VP8 " => {
                if len >= 10 && &payload[3..6] == &[0x9d, 0x01, 0x2a] {
                    let w = payload[6] as u32 | ((payload[7] as u32 & 0x3F) << 8);
                    let h = payload[8] as u32 | ((payload[9] as u32 & 0x3F) << 8);
                    width = Some(w);
                    height = Some(h);
                }
            }
            "VP8L" => {
                if len >= 5 && payload[0] == 0x2F {
                    let raw_bits = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
                    let w = 1 + (raw_bits & 0x3FFF);
                    let h = 1 + ((raw_bits >> 14) & 0x3FFF);
                    width = Some(w);
                    height = Some(h);
                }
            }
            "EXIF" => {
                raw_exif_payload = Some(payload.to_vec());
            }
            "XMP " => {
                raw_xmp_payload = String::from_utf8(payload.to_vec()).ok();
            }
            _ => {}
        }

        pos += len;
        if len % 2 != 0 {
            pos += 1;
        }
    }

    let mut metadata = ExifMetadata::default();
    if let Some(exif_bytes) = raw_exif_payload {
        let reader = kamadak_exif::Reader::new();
        if let Ok(exif_data) = reader.read_raw(exif_bytes) {
            metadata = crate::common::extract_exif_metadata(&exif_data);
        }
    }
    metadata.xmp = raw_xmp_payload;

    let official_end_offset = riff_size + 8;

    Ok(WebpInfo {
        chunks,
        width,
        height,
        metadata,
        official_end_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(tag);
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(payload);
        if payload.len() % 2 != 0 {
            data.push(0);
        }
        data
    }

    #[test]
    fn test_parse_webp_invalid() {
        assert!(parse_webp(b"not webp data").is_err());
    }

    #[test]
    fn test_parse_webp_vp8x() {
        let mut webp = b"RIFF\0\0\0\0WEBP".to_vec();
        
        let vp8x_payload = vec![
            0, 0, 0, 0,
            0x8F, 0x01, 0x00,
            0x2B, 0x01, 0x00,
        ];
        webp.extend_from_slice(&create_mock_chunk(b"VP8X", &vp8x_payload));
        
        let xmp_payload = b"<xmp>metadata</xmp>".to_vec();
        webp.extend_from_slice(&create_mock_chunk(b"XMP ", &xmp_payload));

        let size = (webp.len() - 8) as u32;
        webp[4..8].copy_from_slice(&size.to_le_bytes());

        let info = parse_webp(&webp).unwrap();
        assert_eq!(info.chunks.len(), 2);
        assert_eq!(info.chunks[0].tag, "VP8X");
        assert_eq!(info.width, Some(400));
        assert_eq!(info.height, Some(300));
        assert_eq!(info.metadata.xmp, Some("<xmp>metadata</xmp>".to_string()));
    }
}
