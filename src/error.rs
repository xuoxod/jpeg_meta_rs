use exif as kamadak_exif;

/// Standardized error type representing failures in JPEG or PNG metadata extraction.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    #[error("I/O error: {0}")]
    IoError(String),
    #[error("EXIF parsing error: {0}")]
    ExifError(String),
    #[error("Required metadata tag not found")]
    NotFound,
    #[error("Invalid file signature or format: {0}")]
    InvalidFormat(String),
}

impl From<kamadak_exif::Error> for ParseError {
    fn from(err: kamadak_exif::Error) -> Self {
        ParseError::ExifError(err.to_string())
    }
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::IoError(err.to_string())
    }
}
