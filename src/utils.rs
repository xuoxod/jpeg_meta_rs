/// Helper function to compute the standard IEEE CRC32 checksum.
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

/// Converts Degrees, Minutes, Seconds (DMS) coordinates to standard Decimal degrees.
pub fn dms_to_decimal(degrees: f64, minutes: f64, seconds: f64, is_negative: bool) -> f64 {
    let dec = degrees + minutes / 60.0 + seconds / 3600.0;
    if is_negative { -dec } else { dec }
}

/// Converts pixels per unit (ppu) to dots per inch (DPI).
pub fn ppu_to_dpi(ppu: u32) -> f64 {
    (ppu as f64 * 0.0254).round()
}

/// Matches a field name against a case-insensitive list of filter patterns.
pub fn matches_filter(field_name: &str, filter: &Option<Vec<String>>) -> bool {
    if let Some(list) = filter {
        let name_lower = field_name.to_lowercase().replace(' ', "_").replace("-", "_");
        list.iter().any(|k| name_lower.contains(k))
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn test_dms_to_decimal() {
        // Test standard latitude: 43 deg, 28 min, 2.118 sec North
        let val = dms_to_decimal(43.0, 28.0, 2.118, false);
        assert!((val - 43.467255).abs() < 1e-5);

        // Test South latitude (negative)
        let val_neg = dms_to_decimal(43.0, 28.0, 2.118, true);
        assert!((val_neg - -43.467255).abs() < 1e-5);
    }

    #[test]
    fn test_ppu_to_dpi() {
        // 2835 pixels per meter is roughly 72 DPI
        assert_eq!(ppu_to_dpi(2835), 72.0);
    }

    #[test]
    fn test_matches_filter() {
        let filters = Some(vec!["gps".to_string(), "iso".to_string()]);
        assert!(matches_filter("GPS Latitude", &filters));
        assert!(matches_filter("iso", &filters));
        assert!(!matches_filter("Camera Make", &filters));
    }
}
