// Minimal JPEG segment scanner (SOI, APPn markers)
// This can be expanded to detect other payloads (JFIF/EXIF/XMP) generically.
pub fn list_jpeg_segments(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    if bytes.len() < 2 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        out.push("Not a JPEG (missing SOI)".to_string());
        return out;
    }
    out.push("SOI".to_string());
    // Skip SOI
    let mut pos = 2;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            break;
        }
        // Skip fill bytes 0xFF
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let marker = bytes[pos];
        pos += 1;
        if marker == 0xD9 {
            // EOI
            out.push("EOI".to_string());
            break;
        }
        if marker == 0xDA {
            // SOS, scan data follows until next 0xFF
            out.push(format!("SOS"));
            // We won't parse compressed data fully here
            break;
        }
        if pos + 2 > bytes.len() {
            break;
        }
        let len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        pos += 2;
        out.push(format!("APP/Marker 0xFF{:02X} len {}", marker, len));
        if pos + len - 2 > bytes.len() {
            break;
        }
        pos += len - 2;
    }
    out
}
