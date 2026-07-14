#![allow(clippy::too_many_arguments, clippy::collapsible_match)]
use crate::common::ExifMetadata;
use crate::error::ParseError;
use exif as kamadak_exif;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HeicBox {
    pub box_type: String,
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HeicInfo {
    pub boxes: Vec<HeicBox>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub metadata: ExifMetadata,
}

pub fn parse_heic(bytes: &[u8]) -> Result<HeicInfo, ParseError> {
    if bytes.len() < 12 || &bytes[4..8] != b"ftyp" {
        return Err(ParseError::InvalidFormat("Missing or invalid HEIC ftyp header".to_string()));
    }

    let mut boxes = Vec::new();
    let mut width = None;
    let mut height = None;
    let mut exif_item_id = None;
    let mut exif_extents = Vec::new();

    // Box scanning helper
    fn scan_boxes(
        bytes: &[u8],
        start: usize,
        end: usize,
        boxes: &mut Vec<HeicBox>,
        width: &mut Option<u32>,
        height: &mut Option<u32>,
        exif_item_id: &mut Option<u32>,
        exif_extents: &mut Vec<(u64, u64)>,
    ) {
        let mut pos = start;
        while pos + 8 <= end {
            let box_start = pos;
            let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]) as usize;
            let mut type_bytes = [0u8; 4];
            type_bytes.copy_from_slice(&bytes[pos + 4..pos + 8]);
            let box_type = String::from_utf8_lossy(&type_bytes).to_string();

            if len == 0 {
                // Extends to end of file
                boxes.push(HeicBox {
                    box_type,
                    offset: box_start,
                    length: end - box_start,
                });
                break;
            }

            if pos + len > end {
                break;
            }

            boxes.push(HeicBox {
                box_type: box_type.clone(),
                offset: box_start,
                length: len,
            });

            let payload_start = pos + 8;
            let payload_end = pos + len;

            match box_type.as_str() {
                "meta" => {
                    // meta box has a 4-byte FullBox header (version/flags)
                    if payload_start + 4 <= payload_end {
                        scan_boxes(
                            bytes,
                            payload_start + 4,
                            payload_end,
                            boxes,
                            width,
                            height,
                            exif_item_id,
                            exif_extents,
                        );
                    }
                }
                "iprp" | "ipco" => {
                    scan_boxes(
                        bytes,
                        payload_start,
                        payload_end,
                        boxes,
                        width,
                        height,
                        exif_item_id,
                        exif_extents,
                    );
                }
                "ispe" => {
                    // ispe is a FullBox (4 bytes header) followed by 4-byte width and 4-byte height
                    if len >= 16 {
                        let w = u32::from_be_bytes([
                            bytes[payload_start + 4],
                            bytes[payload_start + 5],
                            bytes[payload_start + 6],
                            bytes[payload_start + 7],
                        ]);
                        let h = u32::from_be_bytes([
                            bytes[payload_start + 8],
                            bytes[payload_start + 9],
                            bytes[payload_start + 10],
                            bytes[payload_start + 11],
                        ]);
                        *width = Some(w);
                        *height = Some(h);
                    }
                }
                "iinf" => {
                    // iinf is a FullBox. Version 0/1 has 2-byte entry count. Version 2+ has 4-byte entry count.
                    if len >= 14 {
                        let version = bytes[payload_start];
                        let header_size = if version == 0 { 6 } else { 8 };
                        if payload_start + header_size <= payload_end {
                            scan_boxes(
                                bytes,
                                payload_start + header_size,
                                payload_end,
                                boxes,
                                width,
                                height,
                                exif_item_id,
                                exif_extents,
                            );
                        }
                    }
                }
                "infe" => {
                    // infe is a FullBox
                    if len >= 12 {
                        let version = bytes[payload_start];
                        if version >= 2 {
                            let mut infe_pos = payload_start + 4; // Skip FullBox header
                            let item_id = if version == 2 {
                                let id = u16::from_be_bytes([bytes[infe_pos], bytes[infe_pos + 1]]) as u32;
                                infe_pos += 4; // Skip item_id (2) + item_protection_index (2)
                                id
                            } else {
                                let id = u32::from_be_bytes([
                                    bytes[infe_pos],
                                    bytes[infe_pos + 1],
                                    bytes[infe_pos + 2],
                                    bytes[infe_pos + 3],
                                ]);
                                infe_pos += 6; // Skip item_id (4) + item_protection_index (2)
                                id
                            };

                            if infe_pos + 4 <= payload_end {
                                let item_type = &bytes[infe_pos..infe_pos + 4];
                                if item_type == b"Exif" {
                                    *exif_item_id = Some(item_id);
                                }
                            }
                        }
                    }
                }
                "iloc" => {
                    // iloc is a FullBox
                    if len >= 12 {
                        let version = bytes[payload_start];
                        let flags_byte = bytes[payload_start + 4];
                        let offset_size = ((flags_byte >> 4) & 0x0F) as usize;
                        let length_size = (flags_byte & 0x0F) as usize;

                        let base_offset_flags = bytes[payload_start + 5];
                        let base_offset_size = ((base_offset_flags >> 4) & 0x0F) as usize;

                        let mut iloc_pos = payload_start + 6; // Skip version/flags/size fields
                        
                        let item_count = if version < 2 {
                            let count = u16::from_be_bytes([bytes[iloc_pos], bytes[iloc_pos + 1]]) as u32;
                            iloc_pos += 2;
                            count
                        } else {
                            let count = u32::from_be_bytes([
                                bytes[iloc_pos],
                                bytes[iloc_pos + 1],
                                bytes[iloc_pos + 2],
                                bytes[iloc_pos + 3],
                            ]);
                            iloc_pos += 4;
                            count
                        };

                        for _ in 0..item_count {
                            if iloc_pos + 2 > payload_end {
                                break;
                            }
                            let item_id = if version < 2 {
                                let id = u16::from_be_bytes([bytes[iloc_pos], bytes[iloc_pos + 1]]) as u32;
                                iloc_pos += 2;
                                id
                            } else {
                                let id = u32::from_be_bytes([
                                    bytes[iloc_pos],
                                    bytes[iloc_pos + 1],
                                    bytes[iloc_pos + 2],
                                    bytes[iloc_pos + 3],
                                ]);
                                iloc_pos += 4;
                                id
                            };

                            iloc_pos += 2; // Skip data_reference_index

                            let mut base_offset = 0u64;
                            if base_offset_size > 0 && iloc_pos + base_offset_size <= payload_end {
                                let mut temp = [0u8; 8];
                                temp[8 - base_offset_size..].copy_from_slice(&bytes[iloc_pos..iloc_pos + base_offset_size]);
                                base_offset = u64::from_be_bytes(temp);
                                iloc_pos += base_offset_size;
                            }

                            if iloc_pos + 2 > payload_end {
                                break;
                            }
                            let extent_count = u16::from_be_bytes([bytes[iloc_pos], bytes[iloc_pos + 1]]);
                            iloc_pos += 2;

                            let mut extents = Vec::new();
                            for _ in 0..extent_count {
                                let mut ext_offset = 0u64;
                                if offset_size > 0 && iloc_pos + offset_size <= payload_end {
                                    let mut temp = [0u8; 8];
                                    temp[8 - offset_size..].copy_from_slice(&bytes[iloc_pos..iloc_pos + offset_size]);
                                    ext_offset = u64::from_be_bytes(temp);
                                    iloc_pos += offset_size;
                                }

                                let mut ext_length = 0u64;
                                if length_size > 0 && iloc_pos + length_size <= payload_end {
                                    let mut temp = [0u8; 8];
                                    temp[8 - length_size..].copy_from_slice(&bytes[iloc_pos..iloc_pos + length_size]);
                                    ext_length = u64::from_be_bytes(temp);
                                    iloc_pos += length_size;
                                }

                                extents.push((base_offset + ext_offset, ext_length));
                            }

                            if Some(item_id) == *exif_item_id {
                                *exif_extents = extents;
                            }
                        }
                    }
                }
                _ => {}
            }

            pos += len;
        }
    }

    scan_boxes(
        bytes,
        0,
        bytes.len(),
        &mut boxes,
        &mut width,
        &mut height,
        &mut exif_item_id,
        &mut exif_extents,
    );

    let mut metadata = ExifMetadata::default();

    // Extract EXIF payload from extents
    if !exif_extents.is_empty() {
        let (offset, length) = exif_extents[0];
        let offset = offset as usize;
        let length = length as usize;

        if offset + length <= bytes.len() && length > 4 {
            let payload = &bytes[offset..offset + length];
            // The first 4 bytes of Exif item in HEIC is an offset/prefix to the TIFF header
            let exif_offset = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
            if exif_offset + 4 < length {
                let exif_bytes = &payload[4 + exif_offset..];
                let reader = kamadak_exif::Reader::new();
                if let Ok(exif_data) = reader.read_raw(exif_bytes.to_vec()) {
                    metadata = crate::common::extract_exif_metadata(&exif_data);
                }
            }
        }
    }

    Ok(HeicInfo {
        boxes,
        width,
        height,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_box(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        let total_len = (payload.len() + 8) as u32;
        data.extend_from_slice(&total_len.to_be_bytes());
        data.extend_from_slice(tag);
        data.extend_from_slice(payload);
        data
    }

    #[test]
    fn test_parse_heic_invalid() {
        assert!(parse_heic(b"not heic data").is_err());
    }

    #[test]
    fn test_parse_heic_valid() {
        let mut heic = Vec::new();
        
        let ftyp_payload = b"heic\0\0\0\0heicheixhevc";
        heic.extend_from_slice(&create_mock_box(b"ftyp", ftyp_payload));

        let mut meta_payload = vec![0, 0, 0, 0];
        
        let mut ispe_payload = vec![0, 0, 0, 0];
        ispe_payload.extend_from_slice(&400u32.to_be_bytes());
        ispe_payload.extend_from_slice(&300u32.to_be_bytes());
        meta_payload.extend_from_slice(&create_mock_box(b"ispe", &ispe_payload));

        heic.extend_from_slice(&create_mock_box(b"meta", &meta_payload));

        let info = parse_heic(&heic).unwrap();
        assert_eq!(info.width, Some(400));
        assert_eq!(info.height, Some(300));
        assert_eq!(info.boxes.len(), 3);
        assert_eq!(info.boxes[0].box_type, "ftyp");
        assert_eq!(info.boxes[1].box_type, "meta");
        assert_eq!(info.boxes[2].box_type, "ispe");
    }
}
