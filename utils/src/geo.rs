/// Converts Degrees, Minutes, Seconds (DMS) coordinates to standard Decimal degrees.
pub fn dms_to_decimal(degrees: f64, minutes: f64, seconds: f64, is_negative: bool) -> f64 {
    let dec = degrees + minutes / 60.0 + seconds / 3600.0;
    if is_negative { -dec } else { dec }
}

/// Converts pixels per unit (ppu) to dots per inch (DPI).
pub fn ppu_to_dpi(ppu: u32) -> f64 {
    (ppu as f64 * 0.0254).round()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dms_to_decimal() {
        let val = dms_to_decimal(43.0, 28.0, 2.118, false);
        assert!((val - 43.467255).abs() < 1e-5);

        let val_neg = dms_to_decimal(43.0, 28.0, 2.118, true);
        assert!((val_neg - -43.467255).abs() < 1e-5);
    }

    #[test]
    fn test_ppu_to_dpi() {
        assert_eq!(ppu_to_dpi(2835), 72.0);
    }
}
