#![allow(clippy::collapsible_if)]
use crate::common::{ExifMetadata, GpsInfo};
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

/// Fully extracted structure and metadata of a PNG file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PngInfo {
    pub chunks: Vec<PngChunk>,
    pub header: Option<PngHeader>,
    pub text_metadata: Vec<(String, String)>,
    pub modification_time: Option<String>,
    pub resolution: Option<PngResolution>,
    pub metadata: ExifMetadata,
}

/// Helper function to compute IEEE CRC32 checksum.
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFFFFFFu32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            if c & 1 != 0 {
                c = (c >> 1) ^ 0xEDB88320;
            } else {
                c >>= 1;
            }
        }
    }
    !c
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
    let mut raw_exif_payload: Option<Vec<u8>> = None;

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
        let calculated_crc = crc32(&crc_check_data);
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
                    }
                }
            }
            "iTXt" => {
                if let Some(idx) = payload.iter().position(|&b| b == 0) {
                    if let Ok(key) = std::str::from_utf8(&payload[0..idx]) {
                        let remaining = &payload[idx + 1..];
                        if remaining.len() >= 2 {
                            let compression_flag = remaining[0];
                            // We ignore compression_method (remaining[1])
                            let mut inner_pos = 2;
                            // Find langTag string \0 terminated
                            if let Some(lang_len) = remaining[inner_pos..].iter().position(|&b| b == 0) {
                                inner_pos += lang_len + 1;
                                // Find translatedKeyword string \0 terminated
                                if let Some(trans_len) = remaining[inner_pos..].iter().position(|&b| b == 0) {
                                    inner_pos += trans_len + 1;
                                    // Text content (uncompressed UTF-8 supported)
                                    if compression_flag == 0 {
                                        if let Ok(val) = std::str::from_utf8(&remaining[inner_pos..]) {
                                            text_metadata.push((key.to_string(), val.to_string()));
                                        }
                                    }
                                }
                            }
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

    Ok(PngInfo {
        chunks,
        header,
        text_metadata,
        modification_time,
        resolution,
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
        let crc = crc32(&crc_check);
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
}
