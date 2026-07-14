#![allow(clippy::collapsible_if)]
use crate::common::ExifMetadata;
use crate::error::ParseError;
use exif as kamadak_exif;
use serde::Serialize;

/// Details of a discovered PNG chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PngChunk {
    pub chunk_type: [u8; 4],
    pub type_name: String,
    pub offset: usize,
    pub length: usize,
    pub crc: u32,
    pub crc_valid: bool,
}

/// Structural details parsed from the IHDR chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PngHeader {
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub color_type: u8,
    pub compression_method: u8,
    pub filter_method: u8,
    pub interlace_method: u8,
}

/// Resolution info parsed from the pHYs chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PngResolution {
    pub ppu_x: u32,
    pub ppu_y: u32,
    pub unit_specifier: u8, // 1 = meter, 0 = unknown
}

/// Significant bits from sBIT chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PngSignificantBits {
    pub bits: Vec<u8>,
}

/// Background color from bKGD chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PngBackgroundColor {
    pub gray: Option<u16>,
    pub rgb: Option<(u16, u16, u16)>,
    pub palette_index: Option<u8>,
}

/// Image offset from oFFs chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PngOffset {
    pub offset_x: i32,
    pub offset_y: i32,
    pub unit_specifier: u8, // 0 = pixel, 1 = micrometer
}

/// Physical scale from sCAL chunk.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PngPhysicalScale {
    pub unit_specifier: u8, // 1 = meter, 2 = radian
    pub scale_x: f64,
    pub scale_y: f64,
}

/// Fully extracted structure and metadata of a PNG file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PngInfo {
    pub chunks: Vec<PngChunk>,
    pub header: Option<PngHeader>,
    pub text_metadata: Vec<(String, String)>,
    pub modification_time: Option<String>,
    pub resolution: Option<PngResolution>,
    pub significant_bits: Option<PngSignificantBits>,
    pub background_color: Option<PngBackgroundColor>,
    pub offset: Option<PngOffset>,
    pub physical_scale: Option<PngPhysicalScale>,
    pub metadata: ExifMetadata,
}

/// Parses raw PNG bytes to extract chunk lists, IHDR properties, text tags, timestamps, and EXIF metadata.
pub fn parse_png(bytes: &[u8]) -> Result<PngInfo, ParseError> {
    let signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 8 || bytes[0..8] != signature {
        return Err(ParseError::InvalidFormat("Missing or invalid PNG signature".to_string()));
    }

    let mut chunks = Vec::new();
    let mut header = None;
    let mut text_metadata = Vec::new();
    let mut modification_time = None;
    let mut resolution = None;
    let mut significant_bits = None;
    let mut background_color = None;
    let mut offset = None;
    let mut physical_scale = None;
    let mut raw_exif_payload: Option<Vec<u8>> = None;
    let mut raw_xmp_payload: Option<String> = None;

    let mut pos = 8;

    while pos + 8 <= bytes.len() {
        let chunk_start = pos;
        let length = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]) as usize;
        pos += 4;

        let mut type_bytes = [0u8; 4];
        type_bytes.copy_from_slice(&bytes[pos..pos + 4]);
        let type_name = String::from_utf8_lossy(&type_bytes).to_string();
        pos += 4;

        if pos + length + 4 > bytes.len() {
            break;
        }

        let payload = &bytes[pos..pos + length];
        pos += length;

        let crc = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
        pos += 4;

        // Verify CRC (covers type + data)
        let mut crc_check_data = Vec::with_capacity(4 + length);
        crc_check_data.extend_from_slice(&type_bytes);
        crc_check_data.extend_from_slice(payload);
        let calculated_crc = crate::utils::crc32(&crc_check_data);
        let crc_valid = calculated_crc == crc;

        chunks.push(PngChunk {
            chunk_type: type_bytes,
            type_name: type_name.clone(),
            offset: chunk_start,
            length: length + 12,
            crc,
            crc_valid,
        });

        match type_name.as_str() {
            "IHDR" => {
                if payload.len() >= 13 {
                    header = Some(PngHeader {
                        width: u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]),
                        height: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
                        bit_depth: payload[8],
                        color_type: payload[9],
                        compression_method: payload[10],
                        filter_method: payload[11],
                        interlace_method: payload[12],
                    });
                }
            }
            "tEXt" => {
                if let Some(idx) = payload.iter().position(|&b| b == 0) {
                    if let (Ok(key), Ok(val)) = (
                        std::str::from_utf8(&payload[0..idx]),
                        std::str::from_utf8(&payload[idx + 1..]),
                    ) {
                        text_metadata.push((key.to_string(), val.to_string()));
                        if key == "XML:com.adobe.xmp" {
                            raw_xmp_payload = Some(val.trim().to_string());
                        }
                    }
                }
            }
            "iTXt" => {
                if let Some(idx) = payload.iter().position(|&b| b == 0) {
                    if let Ok(key) = std::str::from_utf8(&payload[0..idx]) {
                        let remaining = &payload[idx + 1..];
                        if remaining.len() >= 2 {
                            let compression_flag = remaining[0];
                            let mut inner_pos = 2;
                            if let Some(lang_len) = remaining[inner_pos..].iter().position(|&b| b == 0) {
                                inner_pos += lang_len + 1;
                                if let Some(trans_len) = remaining[inner_pos..].iter().position(|&b| b == 0) {
                                    inner_pos += trans_len + 1;
                                    if compression_flag == 0 {
                                        if let Ok(val) = std::str::from_utf8(&remaining[inner_pos..]) {
                                            text_metadata.push((key.to_string(), val.to_string()));
                                            if key == "XML:com.adobe.xmp" {
                                                raw_xmp_payload = Some(val.trim().to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "sBIT" => {
                significant_bits = Some(PngSignificantBits {
                    bits: payload.to_vec(),
                });
            }
            "bKGD" => {
                if payload.len() == 1 {
                    background_color = Some(PngBackgroundColor {
                        gray: None,
                        rgb: None,
                        palette_index: Some(payload[0]),
                    });
                } else if payload.len() == 2 {
                    let gray = u16::from_be_bytes([payload[0], payload[1]]);
                    background_color = Some(PngBackgroundColor {
                        gray: Some(gray),
                        rgb: None,
                        palette_index: None,
                    });
                } else if payload.len() == 6 {
                    let r = u16::from_be_bytes([payload[0], payload[1]]);
                    let g = u16::from_be_bytes([payload[2], payload[3]]);
                    let b = u16::from_be_bytes([payload[4], payload[5]]);
                    background_color = Some(PngBackgroundColor {
                        gray: None,
                        rgb: Some((r, g, b)),
                        palette_index: None,
                    });
                }
            }
            "oFFs" => {
                if payload.len() >= 9 {
                    let offset_x = i32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                    let offset_y = i32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                    let unit_specifier = payload[8];
                    offset = Some(PngOffset {
                        offset_x,
                        offset_y,
                        unit_specifier,
                    });
                }
            }
            "sCAL" => {
                if payload.len() >= 3 {
                    let unit_specifier = payload[0];
                    let rest = &payload[1..];
                    if let Some(idx) = rest.iter().position(|&b| b == 0) {
                        if let (Ok(x_str), Ok(y_str)) = (
                            std::str::from_utf8(&rest[0..idx]),
                            std::str::from_utf8(&rest[idx + 1..]),
                        ) {
                            let scale_x = x_str.parse::<f64>().unwrap_or(0.0);
                            let scale_y = y_str.parse::<f64>().unwrap_or(0.0);
                            physical_scale = Some(PngPhysicalScale {
                                unit_specifier,
                                scale_x,
                                scale_y,
                            });
                        }
                    }
                }
            }
            "tIME" => {
                if payload.len() >= 7 {
                    let year = u16::from_be_bytes([payload[0], payload[1]]);
                    let month = payload[2];
                    let day = payload[3];
                    let hour = payload[4];
                    let minute = payload[5];
                    let second = payload[6];
                    modification_time = Some(format!(
                        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                        year, month, day, hour, minute, second
                    ));
                }
            }
            "pHYs" => {
                if payload.len() >= 9 {
                    resolution = Some(PngResolution {
                        ppu_x: u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]),
                        ppu_y: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
                        unit_specifier: payload[8],
                    });
                }
            }
            "eXIf" => {
                raw_exif_payload = Some(payload.to_vec());
            }
            "IEND" => {
                break;
            }
            _ => {}
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

    Ok(PngInfo {
        chunks,
        header,
        text_metadata,
        modification_time,
        resolution,
        significant_bits,
        background_color,
        offset,
        physical_scale,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let len = data.len() as u32;
        let mut out = Vec::new();
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);

        let mut crc_check = Vec::new();
        crc_check.extend_from_slice(name);
        crc_check.extend_from_slice(data);
        let crc = crate::utils::crc32(&crc_check);
        out.extend_from_slice(&crc.to_be_bytes());
        out
    }

    #[test]
    fn test_parse_png_invalid_format() {
        let bad_data = b"obviously not png data";
        assert!(parse_png(bad_data).is_err());
    }

    #[test]
    fn test_parse_png_ihdr() {
        let mut mock_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // signature
        
        let ihdr_data = vec![
            0x00, 0x00, 0x01, 0x90, // Width: 400
            0x00, 0x00, 0x01, 0x2C, // Height: 300
            0x08,                   // Bit depth: 8
            0x06,                   // Color type: RGBA (6)
            0x00,                   // Compression
            0x00,                   // Filter
            0x00,                   // Interlace
        ];
        mock_png.extend_from_slice(&create_mock_chunk(b"IHDR", &ihdr_data));
        mock_png.extend_from_slice(&create_mock_chunk(b"IEND", &[]));

        let info = parse_png(&mock_png).unwrap();
        assert_eq!(info.chunks.len(), 2);
        assert_eq!(info.chunks[0].type_name, "IHDR");
        assert!(info.chunks[0].crc_valid);
        
        let ihdr = info.header.as_ref().unwrap();
        assert_eq!(ihdr.width, 400);
        assert_eq!(ihdr.height, 300);
        assert_eq!(ihdr.bit_depth, 8);
        assert_eq!(ihdr.color_type, 6);
    }

    #[test]
    fn test_parse_png_text_metadata() {
        let mut mock_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        
        let mut text_data = Vec::new();
        text_data.extend_from_slice(b"Author\0Rick Walker");
        mock_png.extend_from_slice(&create_mock_chunk(b"tEXt", &text_data));
        mock_png.extend_from_slice(&create_mock_chunk(b"IEND", &[]));

        let info = parse_png(&mock_png).unwrap();
        assert_eq!(info.text_metadata.len(), 1);
        assert_eq!(info.text_metadata[0], ("Author".to_string(), "Rick Walker".to_string()));
    }

    #[test]
    fn test_parse_png_time_and_phys() {
        let mut mock_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        
        // tIME payload: 2026, 7, 13, 20, 30, 45
        let mut time_data = Vec::new();
        time_data.extend_from_slice(&2026u16.to_be_bytes());
        time_data.extend_from_slice(&[7, 13, 20, 30, 45]);
        mock_png.extend_from_slice(&create_mock_chunk(b"tIME", &time_data));

        // pHYs payload: x=2835, y=2835, unit=1 (meter)
        let mut phys_data = Vec::new();
        phys_data.extend_from_slice(&2835u32.to_be_bytes());
        phys_data.extend_from_slice(&2835u32.to_be_bytes());
        phys_data.push(1);
        mock_png.extend_from_slice(&create_mock_chunk(b"pHYs", &phys_data));

        mock_png.extend_from_slice(&create_mock_chunk(b"IEND", &[]));

        let info = parse_png(&mock_png).unwrap();
        assert_eq!(info.modification_time, Some("2026-07-13T20:30:45Z".to_string()));
        let res = info.resolution.as_ref().unwrap();
        assert_eq!(res.ppu_x, 2835);
        assert_eq!(res.ppu_y, 2835);
        assert_eq!(res.unit_specifier, 1);
    }

    #[test]
    fn test_parse_png_truncated_chunk() {
        let mut mock_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // signature
        
        // Chunk length u32 = 0x0000FFFF (65535 bytes)
        mock_png.extend_from_slice(&[0x00, 0x00, 0xFF, 0xFF]);
        mock_png.extend_from_slice(b"IHDR");
        // No payload following, points out of bounds!
        
        let info = parse_png(&mock_png).unwrap();
        // Should parse signature safely, but stop when length points out of bounds
        assert_eq!(info.chunks.len(), 0);
    }

    #[test]
    fn test_parse_png_corrupt_crc() {
        let mut mock_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        
        // Let's create an IEND chunk, but overwrite its CRC to be invalid
        let mut end_chunk = create_mock_chunk(b"IEND", &[]);
        let last_idx = end_chunk.len() - 1;
        end_chunk[last_idx] ^= 0x55; // Corrupt a byte in CRC
        
        mock_png.extend_from_slice(&end_chunk);
        
        let info = parse_png(&mock_png).unwrap();
        assert_eq!(info.chunks.len(), 1);
        assert!(!info.chunks[0].crc_valid);
    }

    #[test]
    fn test_parse_png_text_missing_null() {
        let mut mock_png = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        
        // tEXt chunk with key but missing null terminator and value
        let text_data = b"KeywordOnlyNoNullTerminator";
        mock_png.extend_from_slice(&create_mock_chunk(b"tEXt", text_data));
        mock_png.extend_from_slice(&create_mock_chunk(b"IEND", &[]));
        
        let info = parse_png(&mock_png).unwrap();
        // Should handle missing null separator gracefully without panicking
        assert_eq!(info.text_metadata.len(), 0);
    }
}
