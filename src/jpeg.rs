#![allow(clippy::collapsible_if)]
use crate::common::ExifMetadata;
use crate::error::ParseError;
use exif as kamadak_exif;
use serde::Serialize;

/// Details of a discovered JPEG segment marker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JpegSegment {
    pub marker: u8,
    pub name: String,
    pub offset: usize,
    pub length: usize,
}

/// Fully extracted structure and metadata of a JPEG file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JpegInfo {
    pub segments: Vec<JpegSegment>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub precision: Option<u8>,
    pub channels: Option<u8>,
    pub comment: Option<String>,
    pub metadata: ExifMetadata,
}

/// Parses raw JPEG bytes to extract segment lists, dimensions, comments, and EXIF tags.
pub fn parse_jpeg(bytes: &[u8]) -> Result<JpegInfo, ParseError> {
    if bytes.len() < 2 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err(ParseError::InvalidFormat("Missing JPEG SOI header".to_string()));
    }

    let mut segments = Vec::new();
    let mut width = None;
    let mut height = None;
    let mut precision = None;
    let mut channels = None;
    let mut comment = None;
    let mut raw_exif_payload: Option<Vec<u8>> = None;
    let mut raw_xmp_payload: Option<String> = None;

    let mut pos = 2;

    while pos + 2 <= bytes.len() {
        if bytes[pos] != 0xFF {
            break;
        }

        // Skip any padding 0xFF bytes
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }

        let marker = bytes[pos];
        pos += 1;

        let name = match marker {
            0xC0 => "SOF0 (Baseline DCT)".to_string(),
            0xC2 => "SOF2 (Progressive DCT)".to_string(),
            0xC4 => "DHT (Define Huffman Table)".to_string(),
            0xD9 => "EOI (End of Image)".to_string(),
            0xDA => "SOS (Start of Scan)".to_string(),
            0xDB => "DQT (Define Quantization Table)".to_string(),
            0xE0 => "APP0 (JFIF)".to_string(),
            0xE1 => "APP1 (EXIF/XMP)".to_string(),
            0xE2 => "APP2 (ICC Profile)".to_string(),
            0xED => "APP13 (Photoshop/IPTC)".to_string(),
            0xEE => "APP14 (Adobe)".to_string(),
            0xFE => "COM (Comment)".to_string(),
            m => format!("Marker 0xFF{:02X}", m),
        };

        if marker == 0xD9 {
            segments.push(JpegSegment { marker, name, offset: pos - 2, length: 2 });
            break;
        }
        if marker == 0xDA {
            segments.push(JpegSegment { marker, name, offset: pos - 2, length: 2 });
            // Scan stream starts here, stop scanning segment headers
            break;
        }

        if pos + 2 > bytes.len() {
            break;
        }
        let len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        let segment_start = pos - 2;
        let segment_length = len + 2;

        segments.push(JpegSegment {
            marker,
            name,
            offset: segment_start,
            length: segment_length,
        });

        if pos + len > bytes.len() {
            break;
        }

        let payload = &bytes[pos + 2 .. pos + len];

        match marker {
            // SOF0 & SOF2: parse dimensions
            0xC0 | 0xC2 => {
                if payload.len() >= 6 {
                    precision = Some(payload[0]);
                    height = Some(u16::from_be_bytes([payload[1], payload[2]]) as u32);
                    width = Some(u16::from_be_bytes([payload[3], payload[4]]) as u32);
                    channels = Some(payload[5]);
                }
            }
            // APP1: extract EXIF or XMP payload if present
            0xE1 => {
                if payload.len() >= 6 && &payload[0..6] == b"Exif\0\0" {
                    raw_exif_payload = Some(payload[6..].to_vec());
                } else if payload.len() >= 29 && &payload[0..29] == b"http://ns.adobe.com/xap/1.0/\0" {
                    raw_xmp_payload = Some(String::from_utf8_lossy(&payload[29..]).trim().to_string());
                }
            }
            // COM: extract comment
            0xFE => {
                if let Ok(s) = std::str::from_utf8(payload) {
                    comment = Some(s.trim().to_string());
                }
            }
            _ => {}
        }

        pos += len;
    }

    // Parse EXIF metadata using kamadak-exif on extracted raw bytes
    let mut metadata = ExifMetadata::default();
    if let Some(exif_bytes) = raw_exif_payload {
        let reader = kamadak_exif::Reader::new();
        if let Ok(exif_data) = reader.read_raw(exif_bytes) {
            metadata = crate::common::extract_exif_metadata(&exif_data);
        }
    }

    metadata.xmp = raw_xmp_payload;

    Ok(JpegInfo {
        segments,
        width,
        height,
        precision,
        channels,
        comment,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_test_jpeg(filename: &str) -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_images");
        path.push(filename);
        std::fs::read(path).expect("Failed to read test image")
    }

    #[test]
    fn test_parse_jpeg_invalid_format() {
        let bad_data = b"obviously not jpeg data";
        assert!(parse_jpeg(bad_data).is_err());
    }

    #[test]
    fn test_parse_jpeg_soi_only() {
        let soi_only = [0xFF, 0xD8];
        let info = parse_jpeg(&soi_only).unwrap();
        assert_eq!(info.segments.len(), 0);
        assert!(info.width.is_none());
        assert!(info.metadata.camera_make.is_none());
    }

    #[test]
    fn test_parse_jpeg_app_segments() {
        let bytes = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xE0, // APP0
            0x00, 0x07, // Length: 7
            0x01, 0x02, 0x03, 0x04, 0x05,
            0xFF, 0xD9, // EOI
        ];
        let info = parse_jpeg(&bytes).unwrap();
        assert_eq!(info.segments.len(), 2);
        assert_eq!(info.segments[0].name, "APP0 (JFIF)");
        assert_eq!(info.segments[0].length, 9); // len field (2) + marker (2) + payload (5)? Wait, u16 length field is 7, so segment length is len + 2 = 9. Correct!
        assert_eq!(info.segments[1].name, "EOI (End of Image)");
    }

    #[test]
    fn test_parse_jpeg_sof0() {
        let bytes = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xC0, // SOF0
            0x00, 0x0B, // Length: 11
            0x08,       // Precision: 8
            0x01, 0x2C, // Height: 300
            0x01, 0x90, // Width: 400
            0x03,       // Components: 3 (RGB)
            0x01, 0x11, 0x00, // Dummy component details
        ];
        let info = parse_jpeg(&bytes).unwrap();
        assert_eq!(info.width, Some(400));
        assert_eq!(info.height, Some(300));
        assert_eq!(info.precision, Some(8));
        assert_eq!(info.channels, Some(3));
    }

    #[test]
    fn test_parse_jpeg_exif() {
        let bytes = load_test_jpeg("img6-gps.jpg");
        let info = parse_jpeg(&bytes).unwrap();
        assert_eq!(info.metadata.camera_make, Some("NIKON".to_string()));
        assert_eq!(info.metadata.camera_model, Some("COOLPIX P6000".to_string()));
        let gps = info.metadata.gps.as_ref().unwrap();
        assert!(gps.latitude.is_some());
        assert!(gps.longitude.is_some());
    }

    #[test]
    fn test_parse_jpeg_truncated_marker_len() {
        let bytes = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xE0, // APP0
            0x00, 0xFF, // Length: 255 (points way beyond byte array size!)
            0x01, 0x02,
        ];
        let info = parse_jpeg(&bytes).unwrap();
        // Should parse the SOI and first marker offset safely without overflow/panic
        assert_eq!(info.segments.len(), 1);
        assert_eq!(info.segments[0].name, "APP0 (JFIF)");
    }

    #[test]
    fn test_parse_jpeg_non_utf8_comment() {
        let bytes = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xFE, // COM
            0x00, 0x05, // Length: 5
            0xFF, 0xFE, 0xFF, // Invalid UTF-8 bytes!
            0xFF, 0xD9, // EOI
        ];
        let info = parse_jpeg(&bytes).unwrap();
        // Should not panic, but comment should be None since it's invalid UTF-8
        assert!(info.comment.is_none());
        assert_eq!(info.segments.len(), 2);
    }

    #[test]
    fn test_parse_jpeg_empty_payload() {
        let bytes = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xE1, // APP1
            0x00, 0x02, // Length: 2 (zero payload)
            0xFF, 0xD9, // EOI
        ];
        let info = parse_jpeg(&bytes).unwrap();
        assert_eq!(info.segments.len(), 2);
        assert!(info.metadata.camera_make.is_none());
    }
}
