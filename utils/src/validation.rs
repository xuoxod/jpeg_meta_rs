use crate::error::ValidationError;

/// Parses and validates comma-separated filter keys, ensuring no empty or malformed strings.
pub fn validate_filter_keys(keys_str: &str) -> Result<Vec<String>, ValidationError> {
    let list: Vec<String> = keys_str
        .split(',')
        .map(|k| k.trim().to_lowercase())
        .filter(|k| !k.is_empty())
        .collect();

    if list.is_empty() {
        return Err(ValidationError::InvalidFilterKey(keys_str.to_string()));
    }
    Ok(list)
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
    fn test_validate_filter_keys() {
        let keys = validate_filter_keys("make, gps, iso").unwrap();
        assert_eq!(keys, vec!["make", "gps", "iso"]);
        
        let err = validate_filter_keys("  , , ").unwrap_err();
        assert!(matches!(err, ValidationError::InvalidFilterKey(_)));
    }

    #[test]
    fn test_matches_filter() {
        let filters = Some(vec!["gps".to_string(), "iso".to_string()]);
        assert!(matches_filter("GPS Latitude", &filters));
        assert!(matches_filter("iso", &filters));
        assert!(!matches_filter("Camera Make", &filters));
    }
}
