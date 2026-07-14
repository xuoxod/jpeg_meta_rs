use serde::Serialize;

/// Geographic coordinates extracted from EXIF GPS tags.
#[derive(Debug, PartialEq, Default, Clone, Serialize)]
pub struct GpsInfo {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f32>,
}

/// Standardized EXIF and generic metadata tags extracted from JPEG or PNG containers.
#[derive(Debug, PartialEq, Default, Clone, Serialize)]
pub struct ExifMetadata {
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
}
