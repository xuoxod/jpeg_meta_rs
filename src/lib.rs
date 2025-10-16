// --- Dependencies ---
// Make sure you have `kamadak-exif = "0.5"` in your Cargo.toml
// The crate package is named "kamadak-exif" on crates.io but the Rust crate name is `exif`.
// We alias it as `kamadak_exif` to keep the code readable below.
use exif as kamadak_exif;
use std::io::Cursor; // Use the actual crate name with an alias 'kamadak_exif' for convenience

// --- Data Structures ---

// A simple struct to hold GPS coordinates (can be expanded later)
#[derive(Debug, PartialEq, Default, Clone, serde::Serialize)] // Added Serialize
pub struct GpsInfo {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f32>,
}

// The main struct to hold parsed metadata
#[derive(Debug, PartialEq, Default, Clone, serde::Serialize)] // Added Serialize
pub struct JpegMetadata {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub gps: Option<GpsInfo>,
    pub f_number: Option<f32>, // Representing F-stop (e.g., 2.8)
    pub iso: Option<u16>,
    pub exposure_time: Option<String>,  // e.g., "1/125"
    pub focal_length_mm: Option<f32>,   // e.g., 50.0
    pub focal_length_35mm: Option<u16>, // e.g., 75
    pub lens_make: Option<String>,
    pub lens_model: Option<String>,
    pub date_time_original: Option<String>,
    pub orientation: Option<u16>, // EXIF orientation value 1-8
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub software: Option<String>,
    // Add other fields/collections here as needed
}

// --- Error Handling ---
#[derive(Debug, PartialEq)]
pub enum ParseError {
    IoError(String),   // Wrap IO errors
    ExifError(String), // Wrap errors from the kamadak_exif crate
    NotFound,          // Required metadata not found (maybe specific tag)
    InvalidFormat,     // Data format is not as expected (e.g., not JPEG)
                       // Add more specific errors as needed
}

// Implement From trait for easier error conversion from kamadak_exif::Error
impl From<kamadak_exif::Error> for ParseError {
    fn from(err: kamadak_exif::Error) -> Self {
        ParseError::ExifError(err.to_string())
    }
}

// Implement From trait for easier error conversion from std::io::Error
impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::IoError(err.to_string())
    }
}

// --- Parsing Logic ---

/// Parses JPEG data from a byte slice to extract specific EXIF metadata.
///
/// # Arguments
///
/// * `jpeg_data` - A byte slice containing the JPEG file data.
///
/// # Returns
///
/// A `Result` containing the `JpegMetadata` struct on success,
/// or a `ParseError` on failure.
pub fn parse_metadata(jpeg_data: &[u8]) -> Result<JpegMetadata, ParseError> {
    // Use a cursor to allow the kamadak_exif reader to read from the in-memory byte slice
    let mut cursor = Cursor::new(jpeg_data);
    let exif_reader = kamadak_exif::Reader::new();

    // Attempt to read EXIF data from the JPEG container
    // The `?` operator will propagate any kamadak_exif::Error, converting it via our `From` trait
    let exif_data = match exif_reader.read_from_container(&mut cursor) {
        Ok(data) => data,
        Err(kamadak_exif::Error::NotFound(_)) => {
            // If no EXIF segment is found, it's not an error for parsing,
            // just return default metadata.
            return Ok(JpegMetadata::default());
        }
        Err(e) => {
            // For other errors (like invalid format), propagate them.
            return Err(ParseError::from(e));
        }
    };

    let mut metadata = JpegMetadata::default();

    // Helper closure to extract ASCII string fields cleanly
    let get_ascii_string = |tag: kamadak_exif::Tag| -> Option<String> {
        exif_data
            .get_field(tag, kamadak_exif::In::PRIMARY)
            .and_then(|field| match &field.value {
                kamadak_exif::Value::Ascii(vec) if !vec.is_empty() => {
                    // Attempt to convert the primary ASCII string, trim whitespace
                    std::str::from_utf8(&vec[0])
                        .ok()
                        .map(|s| s.trim().to_string())
                }
                _ => None, // Not ASCII or empty
            })
            .filter(|s| !s.is_empty()) // Ensure we don't return empty strings
    };

    // Extract Make and Model using the helper
    metadata.camera_make = get_ascii_string(kamadak_exif::Tag::Make);
    metadata.camera_model = get_ascii_string(kamadak_exif::Tag::Model);

    // Extract FNumber (often stored as Rational)
    if let Some(field) = exif_data.get_field(kamadak_exif::Tag::FNumber, kamadak_exif::In::PRIMARY)
    {
        if let kamadak_exif::Value::Rational(rational_vec) = &field.value {
            if !rational_vec.is_empty() {
                // F-number is typically the first rational value
                metadata.f_number = Some(rational_vec[0].to_f32());
            }
        }
    }

    // ExposureTime (as a string preserving fraction if available)
    if let Some(field) =
        exif_data.get_field(kamadak_exif::Tag::ExposureTime, kamadak_exif::In::PRIMARY)
    {
        metadata.exposure_time = match &field.value {
            kamadak_exif::Value::Rational(v) if !v.is_empty() => {
                let r = v[0];
                if r.denom != 0 {
                    Some(format!("{}/{}", r.num, r.denom))
                } else {
                    Some(format!("{}", r.to_f32()))
                }
            }
            kamadak_exif::Value::SRational(v) if !v.is_empty() => {
                let r = v[0];
                if r.denom != 0 {
                    Some(format!("{}/{}", r.num, r.denom))
                } else {
                    Some(format!("{}", r.to_f32()))
                }
            }
            _ => None,
        };
    }

    // FocalLength
    if let Some(field) =
        exif_data.get_field(kamadak_exif::Tag::FocalLength, kamadak_exif::In::PRIMARY)
    {
        if let kamadak_exif::Value::Rational(v) = &field.value {
            if let Some(r) = v.first() {
                metadata.focal_length_mm = Some(r.to_f32());
            }
        }
    }

    // FocalLengthIn35mmFilm
    if let Some(field) = exif_data.get_field(
        kamadak_exif::Tag::FocalLengthIn35mmFilm,
        kamadak_exif::In::PRIMARY,
    ) {
        if let kamadak_exif::Value::Short(v) = &field.value {
            if let Some(x) = v.first() {
                metadata.focal_length_35mm = Some(*x);
            }
        }
    }

    // Lens info
    metadata.lens_make = get_ascii_string(kamadak_exif::Tag::LensMake);
    metadata.lens_model = get_ascii_string(kamadak_exif::Tag::LensModel);

    // DateTimeOriginal
    metadata.date_time_original = get_ascii_string(kamadak_exif::Tag::DateTimeOriginal);

    // Orientation
    if let Some(field) =
        exif_data.get_field(kamadak_exif::Tag::Orientation, kamadak_exif::In::PRIMARY)
    {
        if let kamadak_exif::Value::Short(v) = &field.value {
            if let Some(x) = v.first() {
                metadata.orientation = Some(*x);
            }
        }
    }

    // Pixel dimensions: prefer PixelX/YDimension; fallback to ImageWidth/Length
    if let Some(field) = exif_data
        .get_field(
            kamadak_exif::Tag::PixelXDimension,
            kamadak_exif::In::PRIMARY,
        )
        .or_else(|| exif_data.get_field(kamadak_exif::Tag::ImageWidth, kamadak_exif::In::PRIMARY))
    {
        metadata.width = match &field.value {
            kamadak_exif::Value::Long(v) if !v.is_empty() => Some(v[0]),
            kamadak_exif::Value::Short(v) if !v.is_empty() => Some(v[0] as u32),
            _ => None,
        };
    }
    if let Some(field) = exif_data
        .get_field(
            kamadak_exif::Tag::PixelYDimension,
            kamadak_exif::In::PRIMARY,
        )
        .or_else(|| exif_data.get_field(kamadak_exif::Tag::ImageLength, kamadak_exif::In::PRIMARY))
    {
        metadata.height = match &field.value {
            kamadak_exif::Value::Long(v) if !v.is_empty() => Some(v[0]),
            kamadak_exif::Value::Short(v) if !v.is_empty() => Some(v[0] as u32),
            _ => None,
        };
    }

    // Software
    metadata.software = get_ascii_string(kamadak_exif::Tag::Software);

    // Extract ISO value using the PhotographicSensitivity tag (ISO equivalent)
    let iso_field = exif_data.get_field(
        kamadak_exif::Tag::PhotographicSensitivity,
        kamadak_exif::In::PRIMARY,
    );

    if let Some(field) = iso_field {
        if let kamadak_exif::Value::Short(short_vec) = &field.value {
            if !short_vec.is_empty() {
                // ISO is typically the first short value
                metadata.iso = Some(short_vec[0]);
            }
        }
        // Sometimes ISO might be stored as Long, handle that too if needed
        else if let kamadak_exif::Value::Long(long_vec) = &field.value {
            if !long_vec.is_empty() {
                // Convert u32 to u16, checking for overflow (though unlikely for ISO)
                metadata.iso = long_vec[0].try_into().ok();
            }
        }
    }

    // --- Extract GPS Information ---
    // GPS requires multiple tags, so we extract them individually first.
    let lat_opt = exif_data.get_field(kamadak_exif::Tag::GPSLatitude, kamadak_exif::In::PRIMARY);
    let lat_ref_opt =
        exif_data.get_field(kamadak_exif::Tag::GPSLatitudeRef, kamadak_exif::In::PRIMARY);
    let lon_opt = exif_data.get_field(kamadak_exif::Tag::GPSLongitude, kamadak_exif::In::PRIMARY);
    let lon_ref_opt = exif_data.get_field(
        kamadak_exif::Tag::GPSLongitudeRef,
        kamadak_exif::In::PRIMARY,
    );
    let alt_opt = exif_data.get_field(kamadak_exif::Tag::GPSAltitude, kamadak_exif::In::PRIMARY);
    let alt_ref_opt =
        exif_data.get_field(kamadak_exif::Tag::GPSAltitudeRef, kamadak_exif::In::PRIMARY);

    // Proceed only if we have the essential Latitude and Longitude fields and their references
    if let (Some(lat_field), Some(lat_ref_field), Some(lon_field), Some(lon_ref_field)) =
        (lat_opt, lat_ref_opt, lon_opt, lon_ref_opt)
    {
        // Check if the values are of the expected types (Rational for coords, Ascii for refs)
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
            // Ensure the vectors are not empty and contain the required components
            // GPS coords are often [Degrees, Minutes, Seconds]
            // GPS refs are often [[b'N'], [b'E']] etc.
            if lat_val.len() >= 3
                && !lat_ref_val.is_empty()
                && !lat_ref_val[0].is_empty()
                && lon_val.len() >= 3
                && !lon_ref_val.is_empty()
                && !lon_ref_val[0].is_empty()
            {
                let mut gps = GpsInfo::default();

                // Convert DMS (Degrees, Minutes, Seconds) rational to decimal degrees
                let lat_decimal =
                    lat_val[0].to_f64() + lat_val[1].to_f64() / 60.0 + lat_val[2].to_f64() / 3600.0;
                let lon_decimal =
                    lon_val[0].to_f64() + lon_val[1].to_f64() / 60.0 + lon_val[2].to_f64() / 3600.0;

                // Apply reference sign (N/S, E/W)
                // Compare the first byte of the reference ASCII value
                gps.latitude = Some(if lat_ref_val[0][0].to_ascii_uppercase() == b'S' {
                    -lat_decimal
                } else {
                    lat_decimal
                });
                gps.longitude = Some(if lon_ref_val[0][0].to_ascii_uppercase() == b'W' {
                    -lon_decimal
                } else {
                    lon_decimal
                });

                // Handle Altitude (Optional) - Check if altitude field exists
                if let Some(alt_field) = alt_opt {
                    if let kamadak_exif::Value::Rational(alt_val) = &alt_field.value {
                        if !alt_val.is_empty() {
                            let alt_value = alt_val[0].to_f32(); // Altitude is often a single rational

                            // Determine altitude reference (0 = above sea level, 1 = below)
                            let alt_sign = if let Some(alt_ref_field) = alt_ref_opt {
                                // Ref is often stored as a single Byte
                                if let kamadak_exif::Value::Byte(alt_ref_val) = &alt_ref_field.value
                                {
                                    // If ref byte is 1, sign is negative (below sea level)
                                    if !alt_ref_val.is_empty() && alt_ref_val[0] == 1 {
                                        -1.0
                                    } else {
                                        1.0
                                    }
                                } else {
                                    1.0
                                } // Default to above sea level if ref is wrong type
                            } else {
                                1.0
                            }; // Default to above sea level if ref tag is missing

                            gps.altitude = Some(alt_value * alt_sign);
                        }
                    }
                }
                // Assign the parsed GpsInfo to the main metadata struct
                metadata.gps = Some(gps);
            }
        }
    }

    // Return the populated (or partially populated) metadata struct
    Ok(metadata)
}

// --- Tests ---
#[cfg(test)]
mod tests {
    use super::*; // Import items from the parent module (structs, functions, etc.)
    use std::path::PathBuf;

    // Helper function to load test JPEG data from a `test_images` directory
    // relative to the project root (where Cargo.toml is).
    fn load_test_jpeg(filename: &str) -> Result<Vec<u8>, std::io::Error> {
        // Construct the path relative to the crate root
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_images"); // Assumes a directory named test_images
        path.push(filename);

        println!("Attempting to load test JPEG from: {:?}", path); // Debug print
        std::fs::read(path)
    }

    #[test]
    fn test_parse_basic_exif() {
        // TDD Step 1: Define the test case.

        // === ACTION REQUIRED BY YOU ===
        // 1. Create a directory named `test_images` in the root of your `jpeg_meta_rs` project
        //    (the same directory where `Cargo.toml` and `src` are).
        // 2. Place a sample JPEG file WITH KNOWN EXIF data inside `test_images`.
        //    Let's assume you name it `sample_with_exif.jpg`.
        // 3. Find out the actual Make, Model, FNumber, ISO, and GPS data for THAT file.
        //    You can use an online EXIF viewer or a desktop tool.
        // 4. Update the `expected_metadata` below with those ACTUAL values.
        // =============================

        // Use an existing image with EXIF + GPS in your repo
        let filename = "img6-gps.jpg";
        let jpeg_data_result = load_test_jpeg(filename);

        // Check if loading the test file failed
        if let Err(e) = jpeg_data_result {
            panic!(
                "Failed to load test file '{}': {}. \nPlease ensure the file exists in the 'test_images' directory relative to Cargo.toml.",
                filename, e
            );
        }
        let jpeg_data = jpeg_data_result.unwrap();

        // Call the function under test
        let result = parse_metadata(&jpeg_data);

        // Assert that parsing was successful (this might pass even if content is wrong initially)
        assert!(result.is_ok(), "Parsing failed: {:?}", result.err());

        let metadata = result.unwrap();

        // Expected values taken from observed CLI output for img6-gps.jpg
        let expected_metadata = JpegMetadata {
            camera_make: Some("NIKON".to_string()),
            camera_model: Some("COOLPIX P6000".to_string()),
            gps: Some(GpsInfo {
                latitude: Some(43.46725499999722),
                longitude: Some(11.879213333333334),
                altitude: None,
            }),
            f_number: Some(5.9),
            iso: Some(103),
            exposure_time: None,
            focal_length_mm: None,
            focal_length_35mm: None,
            lens_make: None,
            lens_model: None,
            date_time_original: None,
            orientation: None,
            width: None,
            height: None,
            software: None,
        };

        // Assert that the parsed data matches the expected data
        // These assertions WILL FAIL until parse_metadata is implemented correctly.
        assert_eq!(
            metadata.camera_make, expected_metadata.camera_make,
            "Camera make mismatch"
        );
        assert_eq!(
            metadata.camera_model, expected_metadata.camera_model,
            "Camera model mismatch"
        );
        assert_eq!(
            metadata.f_number, expected_metadata.f_number,
            "F-number mismatch"
        );
        assert_eq!(metadata.iso, expected_metadata.iso, "ISO mismatch");
        assert_eq!(metadata.gps, expected_metadata.gps, "GPS info mismatch");
    }

    #[test]
    fn test_parse_no_exif() {
        // TDD: Test case for a JPEG known to have NO EXIF data.

        // === ACTION REQUIRED BY YOU ===
        // 1. Place a sample JPEG file WITHOUT any EXIF data inside the `test_images` directory.
        //    Let's assume you name it `sample_no_exif.jpg`.
        // =============================

        // Use an existing image without EXIF in your repo
        let filename = "img5.jpg";
        let jpeg_data_result = load_test_jpeg(filename);

        if let Err(e) = jpeg_data_result {
            panic!(
                "Failed to load test file '{}': {}. \nPlease ensure the file exists in the 'test_images' directory relative to Cargo.toml.",
                filename, e
            );
        }
        let jpeg_data = jpeg_data_result.unwrap();

        let result = parse_metadata(&jpeg_data);

        // Should succeed
        assert!(
            result.is_ok(),
            "Parsing failed for no-EXIF JPEG: {:?}",
            result.err()
        );
        let metadata = result.unwrap();

        // For files without meaningful EXIF, the core EXIF fields should be None
        assert!(metadata.camera_make.is_none());
        assert!(metadata.camera_model.is_none());
        assert!(metadata.f_number.is_none());
        assert!(metadata.iso.is_none());
        assert!(metadata.gps.is_none());
    }

    #[test]
    fn test_cross_check_with_exif_tags() {
        use std::fs;
        use std::io::Cursor;

        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.push("test_images");

        let entries = fs::read_dir(&dir).expect("test_images directory should exist");
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());
            if !matches!(ext.as_deref(), Some("jpg" | "jpeg")) {
                continue;
            }
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };

            let parsed = parse_metadata(&bytes).expect("parse_metadata should succeed");

            // Load raw exif
            let mut cur = Cursor::new(&bytes);
            let reader = kamadak_exif::Reader::new();
            let exif_data = match reader.read_from_container(&mut cur) {
                Ok(d) => Some(d),
                Err(kamadak_exif::Error::NotFound(_)) => None,
                Err(_) => None,
            };

            if let Some(exif) = exif_data {
                // Helper for ascii
                let get_ascii = |tag: kamadak_exif::Tag| -> Option<String> {
                    exif.get_field(tag, kamadak_exif::In::PRIMARY)
                        .and_then(|f| match &f.value {
                            kamadak_exif::Value::Ascii(v) if !v.is_empty() => {
                                std::str::from_utf8(&v[0])
                                    .ok()
                                    .map(|s| s.trim().to_string())
                            }
                            _ => None,
                        })
                        .filter(|s| !s.is_empty())
                };

                // Make/Model
                assert_eq!(parsed.camera_make, get_ascii(kamadak_exif::Tag::Make));
                assert_eq!(parsed.camera_model, get_ascii(kamadak_exif::Tag::Model));

                // FNumber
                if let Some(field) =
                    exif.get_field(kamadak_exif::Tag::FNumber, kamadak_exif::In::PRIMARY)
                {
                    if let kamadak_exif::Value::Rational(v) = &field.value {
                        if let Some(r) = v.first() {
                            assert!((parsed.f_number.unwrap_or(-1.0) - r.to_f32()).abs() < 1e-3);
                        }
                    }
                } else {
                    assert!(parsed.f_number.is_none());
                }

                // ISO
                if let Some(field) = exif.get_field(
                    kamadak_exif::Tag::PhotographicSensitivity,
                    kamadak_exif::In::PRIMARY,
                ) {
                    match &field.value {
                        kamadak_exif::Value::Short(v) if !v.is_empty() => {
                            assert_eq!(parsed.iso, Some(v[0]))
                        }
                        kamadak_exif::Value::Long(v) if !v.is_empty() => {
                            assert_eq!(parsed.iso, v[0].try_into().ok())
                        }
                        _ => assert!(parsed.iso.is_none()),
                    }
                } else {
                    assert!(parsed.iso.is_none());
                }

                // ExposureTime
                if let Some(field) =
                    exif.get_field(kamadak_exif::Tag::ExposureTime, kamadak_exif::In::PRIMARY)
                {
                    let expect = match &field.value {
                        kamadak_exif::Value::Rational(v) if !v.is_empty() => {
                            let r = v[0];
                            if r.denom != 0 {
                                Some(format!("{}/{}", r.num, r.denom))
                            } else {
                                Some(format!("{}", r.to_f32()))
                            }
                        }
                        kamadak_exif::Value::SRational(v) if !v.is_empty() => {
                            let r = v[0];
                            if r.denom != 0 {
                                Some(format!("{}/{}", r.num, r.denom))
                            } else {
                                Some(format!("{}", r.to_f32()))
                            }
                        }
                        _ => None,
                    };
                    assert_eq!(parsed.exposure_time, expect);
                }

                // Focal Length
                if let Some(field) =
                    exif.get_field(kamadak_exif::Tag::FocalLength, kamadak_exif::In::PRIMARY)
                {
                    if let kamadak_exif::Value::Rational(v) = &field.value {
                        if let Some(r) = v.first() {
                            assert!(
                                (parsed.focal_length_mm.unwrap_or(-1.0) - r.to_f32()).abs() < 1e-3
                            );
                        }
                    }
                }

                if let Some(field) = exif.get_field(
                    kamadak_exif::Tag::FocalLengthIn35mmFilm,
                    kamadak_exif::In::PRIMARY,
                ) {
                    if let kamadak_exif::Value::Short(v) = &field.value {
                        if let Some(x) = v.first() {
                            assert_eq!(parsed.focal_length_35mm, Some(*x));
                        }
                    }
                }

                // Lens
                assert_eq!(parsed.lens_make, get_ascii(kamadak_exif::Tag::LensMake));
                assert_eq!(parsed.lens_model, get_ascii(kamadak_exif::Tag::LensModel));

                // DateTimeOriginal
                assert_eq!(
                    parsed.date_time_original,
                    get_ascii(kamadak_exif::Tag::DateTimeOriginal)
                );

                // Orientation
                if let Some(field) =
                    exif.get_field(kamadak_exif::Tag::Orientation, kamadak_exif::In::PRIMARY)
                {
                    if let kamadak_exif::Value::Short(v) = &field.value {
                        if let Some(x) = v.first() {
                            assert_eq!(parsed.orientation, Some(*x));
                        }
                    }
                }

                // Dimensions
                let width_field = exif
                    .get_field(
                        kamadak_exif::Tag::PixelXDimension,
                        kamadak_exif::In::PRIMARY,
                    )
                    .or_else(|| {
                        exif.get_field(kamadak_exif::Tag::ImageWidth, kamadak_exif::In::PRIMARY)
                    });
                if let Some(field) = width_field {
                    let expect = match &field.value {
                        kamadak_exif::Value::Long(v) if !v.is_empty() => Some(v[0]),
                        kamadak_exif::Value::Short(v) if !v.is_empty() => Some(v[0] as u32),
                        _ => None,
                    };
                    assert_eq!(parsed.width, expect);
                }
                let height_field = exif
                    .get_field(
                        kamadak_exif::Tag::PixelYDimension,
                        kamadak_exif::In::PRIMARY,
                    )
                    .or_else(|| {
                        exif.get_field(kamadak_exif::Tag::ImageLength, kamadak_exif::In::PRIMARY)
                    });
                if let Some(field) = height_field {
                    let expect = match &field.value {
                        kamadak_exif::Value::Long(v) if !v.is_empty() => Some(v[0]),
                        kamadak_exif::Value::Short(v) if !v.is_empty() => Some(v[0] as u32),
                        _ => None,
                    };
                    assert_eq!(parsed.height, expect);
                }

                // Software
                assert_eq!(parsed.software, get_ascii(kamadak_exif::Tag::Software));

                // GPS
                let lat_opt =
                    exif.get_field(kamadak_exif::Tag::GPSLatitude, kamadak_exif::In::PRIMARY);
                let lat_ref_opt =
                    exif.get_field(kamadak_exif::Tag::GPSLatitudeRef, kamadak_exif::In::PRIMARY);
                let lon_opt =
                    exif.get_field(kamadak_exif::Tag::GPSLongitude, kamadak_exif::In::PRIMARY);
                let lon_ref_opt = exif.get_field(
                    kamadak_exif::Tag::GPSLongitudeRef,
                    kamadak_exif::In::PRIMARY,
                );
                if let (
                    Some(lat_field),
                    Some(lat_ref_field),
                    Some(lon_field),
                    Some(lon_ref_field),
                ) = (lat_opt, lat_ref_opt, lon_opt, lon_ref_opt)
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
                            let lat_dec = lat_val[0].to_f64()
                                + lat_val[1].to_f64() / 60.0
                                + lat_val[2].to_f64() / 3600.0;
                            let lon_dec = lon_val[0].to_f64()
                                + lon_val[1].to_f64() / 60.0
                                + lon_val[2].to_f64() / 3600.0;
                            let lat_sign = if lat_ref_val[0][0].to_ascii_uppercase() == b'S' {
                                -1.0
                            } else {
                                1.0
                            };
                            let lon_sign = if lon_ref_val[0][0].to_ascii_uppercase() == b'W' {
                                -1.0
                            } else {
                                1.0
                            };
                            if let Some(gps) = parsed.gps.as_ref() {
                                assert!(
                                    (gps.latitude.unwrap_or(0.0) - lat_dec * lat_sign).abs() < 1e-6
                                );
                                assert!(
                                    (gps.longitude.unwrap_or(0.0) - lon_dec * lon_sign).abs()
                                        < 1e-6
                                );
                            } else {
                                panic!("Expected GPS info present from raw EXIF");
                            }
                        }
                    }
                }
            } else {
                // If no EXIF, we expect parse_metadata to return default metadata
                assert_eq!(parsed, JpegMetadata::default());
            }
        }
    }

    #[test]
    fn test_parse_invalid_data() {
        // TDD: Test case for data that isn't a valid JPEG/EXIF format
        let invalid_data = b"this is definitely not jpeg data";
        let result = parse_metadata(invalid_data);

        // We expect an error, likely ExifError or InvalidFormat depending on implementation
        assert!(result.is_err(), "Parsing should fail for invalid data");
        // Optionally, assert on the specific error type later
        // assert_eq!(result.err().unwrap(), ParseError::InvalidFormat); // Or ExifError(...)
        println!(
            "Received expected error for invalid data: {:?}",
            result.err()
        );
    }
}
