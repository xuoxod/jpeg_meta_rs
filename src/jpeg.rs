#![allow(clippy::collapsible_if)]
use crate::common::{ExifMetadata, GpsInfo};
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
            // APP1: extract EXIF payload if present
            0xE1 => {
                if payload.len() >= 6 && &payload[0..6] == b"Exif\0\0" {
                    raw_exif_payload = Some(payload[6..].to_vec());
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
            let get_ascii_string = |tag: kamadak_exif::Tag| -> Option<String> {
                exif_data
                    .get_field(tag, kamadak_exif::In::PRIMARY)
                    .and_then(|field| match &field.value {
                        kamadak_exif::Value::Ascii(vec) if !vec.is_empty() => {
                            std::str::from_utf8(&vec[0])
                                .ok()
                                .map(|s| s.trim().to_string())
                        }
                        _ => None,
                    })
                    .filter(|s| !s.is_empty())
            };

            metadata.camera_make = get_ascii_string(kamadak_exif::Tag::Make);
            metadata.camera_model = get_ascii_string(kamadak_exif::Tag::Model);
            metadata.lens_make = get_ascii_string(kamadak_exif::Tag::LensMake);
            metadata.lens_model = get_ascii_string(kamadak_exif::Tag::LensModel);
            metadata.date_time_original = get_ascii_string(kamadak_exif::Tag::DateTimeOriginal);
            metadata.software = get_ascii_string(kamadak_exif::Tag::Software);

            if let Some(field) = exif_data.get_field(kamadak_exif::Tag::FNumber, kamadak_exif::In::PRIMARY) {
                if let kamadak_exif::Value::Rational(rational_vec) = &field.value {
                    if !rational_vec.is_empty() {
                        metadata.f_number = Some(rational_vec[0].to_f32());
                    }
                }
            }

            if let Some(field) = exif_data.get_field(kamadak_exif::Tag::PhotographicSensitivity, kamadak_exif::In::PRIMARY) {
                match &field.value {
                    kamadak_exif::Value::Short(v) if !v.is_empty() => metadata.iso = Some(v[0]),
                    kamadak_exif::Value::Long(v) if !v.is_empty() => metadata.iso = v[0].try_into().ok(),
                    _ => {}
                }
            }

            if let Some(field) = exif_data.get_field(kamadak_exif::Tag::ExposureTime, kamadak_exif::In::PRIMARY) {
                metadata.exposure_time = match &field.value {
                    kamadak_exif::Value::Rational(v) if !v.is_empty() => {
                        let r = v[0];
                        if r.denom != 0 { Some(format!("{}/{}", r.num, r.denom)) } else { Some(format!("{}", r.to_f32())) }
                    }
                    kamadak_exif::Value::SRational(v) if !v.is_empty() => {
                        let r = v[0];
                        if r.denom != 0 { Some(format!("{}/{}", r.num, r.denom)) } else { Some(format!("{}", r.to_f32())) }
                    }
                    _ => None,
                };
            }

            if let Some(field) = exif_data.get_field(kamadak_exif::Tag::FocalLength, kamadak_exif::In::PRIMARY) {
                if let kamadak_exif::Value::Rational(v) = &field.value {
                    if let Some(r) = v.first() {
                        metadata.focal_length_mm = Some(r.to_f32());
                    }
                }
            }

            if let Some(field) = exif_data.get_field(kamadak_exif::Tag::FocalLengthIn35mmFilm, kamadak_exif::In::PRIMARY) {
                if let kamadak_exif::Value::Short(v) = &field.value {
                    metadata.focal_length_35mm = v.first().copied();
                }
            }

            if let Some(field) = exif_data.get_field(kamadak_exif::Tag::Orientation, kamadak_exif::In::PRIMARY) {
                if let kamadak_exif::Value::Short(v) = &field.value {
                    metadata.orientation = v.first().copied();
                }
            }

            let width_field = exif_data.get_field(kamadak_exif::Tag::PixelXDimension, kamadak_exif::In::PRIMARY)
                .or_else(|| exif_data.get_field(kamadak_exif::Tag::ImageWidth, kamadak_exif::In::PRIMARY));
            if let Some(field) = width_field {
                metadata.width = match &field.value {
                    kamadak_exif::Value::Long(v) if !v.is_empty() => Some(v[0]),
                    kamadak_exif::Value::Short(v) if !v.is_empty() => Some(v[0] as u32),
                    _ => None,
                };
            }

            let height_field = exif_data.get_field(kamadak_exif::Tag::PixelYDimension, kamadak_exif::In::PRIMARY)
                .or_else(|| exif_data.get_field(kamadak_exif::Tag::ImageLength, kamadak_exif::In::PRIMARY));
            if let Some(field) = height_field {
                metadata.height = match &field.value {
                    kamadak_exif::Value::Long(v) if !v.is_empty() => Some(v[0]),
                    kamadak_exif::Value::Short(v) if !v.is_empty() => Some(v[0] as u32),
                    _ => None,
                };
            }

            // GPS Parsing
            let lat_opt = exif_data.get_field(kamadak_exif::Tag::GPSLatitude, kamadak_exif::In::PRIMARY);
            let lat_ref_opt = exif_data.get_field(kamadak_exif::Tag::GPSLatitudeRef, kamadak_exif::In::PRIMARY);
            let lon_opt = exif_data.get_field(kamadak_exif::Tag::GPSLongitude, kamadak_exif::In::PRIMARY);
            let lon_ref_opt = exif_data.get_field(kamadak_exif::Tag::GPSLongitudeRef, kamadak_exif::In::PRIMARY);
            let alt_opt = exif_data.get_field(kamadak_exif::Tag::GPSAltitude, kamadak_exif::In::PRIMARY);
            let alt_ref_opt = exif_data.get_field(kamadak_exif::Tag::GPSAltitudeRef, kamadak_exif::In::PRIMARY);

            if let (Some(lat_field), Some(lat_ref_field), Some(lon_field), Some(lon_ref_field)) =
                (lat_opt, lat_ref_opt, lon_opt, lon_ref_opt)
            {
                if let (
                    kamadak_exif::Value::Rational(lat_val),
                    kamadak_exif::Value::Ascii(lat_ref_val),
                    kamadak_exif::Value::Rational(lon_val),
                    kamadak_exif::Value::Ascii(lon_ref_val),
                ) = (
                    &lat_field.value,
                    &lat_ref_field.value,
                    &lon_field.value,
                    &lon_ref_field.value,
                ) {
                    if lat_val.len() >= 3
                        && !lat_ref_val.is_empty()
                        && !lat_ref_val[0].is_empty()
                        && lon_val.len() >= 3
                        && !lon_ref_val.is_empty()
                        && !lon_ref_val[0].is_empty()
                    {
                        let mut gps = GpsInfo::default();
                        let lat_dec = lat_val[0].to_f64() + lat_val[1].to_f64() / 60.0 + lat_val[2].to_f64() / 3600.0;
                        let lon_dec = lon_val[0].to_f64() + lon_val[1].to_f64() / 60.0 + lon_val[2].to_f64() / 3600.0;

                        gps.latitude = Some(if lat_ref_val[0][0].eq_ignore_ascii_case(&b'S') { -lat_dec } else { lat_dec });
                        gps.longitude = Some(if lon_ref_val[0][0].eq_ignore_ascii_case(&b'W') { -lon_dec } else { lon_dec });

                        if let Some(alt_field) = alt_opt {
                            if let kamadak_exif::Value::Rational(alt_val) = &alt_field.value {
                                if !alt_val.is_empty() {
                                    let alt_value = alt_val[0].to_f32();
                                    let alt_sign = if let Some(alt_ref_field) = alt_ref_opt {
                                        if let kamadak_exif::Value::Byte(alt_ref_val) = &alt_ref_field.value {
                                            if !alt_ref_val.is_empty() && alt_ref_val[0] == 1 { -1.0 } else { 1.0 }
                                        } else { 1.0 }
                                    } else { 1.0 };
                                    gps.altitude = Some(alt_value * alt_sign);
                                }
                            }
                        }
                        metadata.gps = Some(gps);
                    }
                }
            }
        }
    }

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
}
