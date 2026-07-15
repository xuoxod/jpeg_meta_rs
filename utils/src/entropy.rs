/// Computes the Shannon Entropy of a byte slice.
/// Returns a value between 0.0 (completely predictable) and 8.0 (completely random / encrypted / compressed).
pub fn calculate_shannon_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut counts = [0usize; 256];
    for &b in data {
        counts[b as usize] += 1;
    }

    let len = data.len() as f64;
    let mut entropy = 0.0;
    for count in counts {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_empty() {
        assert_eq!(calculate_shannon_entropy(&[]), 0.0);
    }

    #[test]
    fn test_entropy_predictable() {
        let data = vec![0u8; 100];
        assert_eq!(calculate_shannon_entropy(&data), 0.0);
    }

    #[test]
    fn test_entropy_high() {
        // High entropy: random spread of bytes
        let mut data = Vec::new();
        for i in 0..256 {
            data.push(i as u8);
        }
        let entropy = calculate_shannon_entropy(&data);
        // All 256 values occur exactly once, entropy should be exactly 8.0
        assert!((entropy - 8.0).abs() < 1e-9);
    }
}
