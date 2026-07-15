/// Decoupled, reusable utility to format raw byte counts into human-readable strings (e.g. KB, MB, GB).
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_boundaries() {
        // Bytes Boundary
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1), "1 B");
        assert_eq!(format_size(1023), "1023 B");

        // KB Boundary
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1025), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048575), "1024.00 KB");

        // MB Boundary
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1048577), "1.00 MB");
        assert_eq!(format_size(1572864), "1.50 MB");
        assert_eq!(format_size(1073741823), "1024.00 MB");

        // GB Boundary
        assert_eq!(format_size(1073741824), "1.00 GB");
        assert_eq!(format_size(1073741825), "1.00 GB");
        assert_eq!(format_size(536870912000), "500.00 GB");
        assert_eq!(format_size(1099511627776), "1024.00 GB");
    }

    #[test]
    fn test_format_size_rounding() {
        // 1.004 KB -> 1.00 KB
        assert_eq!(format_size(1028), "1.00 KB");
        // 1.005 KB -> 1.01 KB (1.005 * 1024 = 1029.12)
        assert_eq!(format_size(1030), "1.01 KB");
    }
}
