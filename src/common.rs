use serde::Serialize;

/// Geographic coordinates and tracking metadata extracted from EXIF GPS tags.
#[derive(Debug, PartialEq, Default, Clone, Serialize)]
pub struct GpsInfo {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f32>,
    pub speed: Option<f64>,
    pub speed_ref: Option<String>,
    pub track: Option<f64>,
    pub track_ref: Option<String>,
    pub img_direction: Option<f64>,
    pub img_direction_ref: Option<String>,
    pub date_stamp: Option<String>,
    pub time_stamp: Option<String>,
}

/// Standardized EXIF and generic metadata tags extracted from JPEG or PNG containers.
#[derive(Debug, PartialEq, Default, Clone, Serialize)]
pub struct ExifMetadata {
    // Basic Camera Tags
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub gps: Option<GpsInfo>,
    pub f_number: Option<f32>,
    pub iso: Option<u16>,
    pub exposure_time: Option<String>,
    pub focal_length_mm: Option<f32>,
    pub focal_length_35mm: Option<u16>,
    pub lens_make: Option<String>,
    pub lens_model: Option<String>,
    pub date_time_original: Option<String>,
    pub orientation: Option<u16>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub software: Option<String>,

    // Advanced & Forensic Tags
    pub body_serial_number: Option<String>,
    pub lens_serial_number: Option<String>,
    pub flash: Option<u16>,
    pub exposure_program: Option<u16>,
    pub metering_mode: Option<u16>,
    pub white_balance: Option<u16>,
    pub light_source: Option<u16>,
    pub user_comment: Option<String>,

    // Extensible XML Metadata Block
    pub xmp: Option<String>,
}
