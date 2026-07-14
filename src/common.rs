use serde::Serialize;
use exif as kamadak_exif;

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

/// Extracts full EXIF data properties from a kamadak-exif payload structure.
#[allow(clippy::collapsible_if)]
pub fn extract_exif_metadata(exif_data: &kamadak_exif::Exif) -> ExifMetadata {
    let mut metadata = ExifMetadata::default();
    
    let get_ascii_string = |tag: kamadak_exif::Tag| -> Option<String> {
        exif_data
            .get_field(tag, kamadak_exif::In::PRIMARY)
            .and_then(|field| match &field.value {
                kamadak_exif::Value::Ascii(vec) if !vec.is_empty() => {
                    std::str::from_utf8(&vec[0])
                        .ok()
                        .map(|s| s.trim().to_string())
                }
                _ => None,
            })
            .filter(|s| !s.is_empty())
    };

    metadata.camera_make = get_ascii_string(kamadak_exif::Tag::Make);
    metadata.camera_model = get_ascii_string(kamadak_exif::Tag::Model);
    metadata.lens_make = get_ascii_string(kamadak_exif::Tag::LensMake);
    metadata.lens_model = get_ascii_string(kamadak_exif::Tag::LensModel);
    metadata.date_time_original = get_ascii_string(kamadak_exif::Tag::DateTimeOriginal);
    metadata.software = get_ascii_string(kamadak_exif::Tag::Software);

    if let Some(field) = exif_data.get_field(kamadak_exif::Tag::FNumber, kamadak_exif::In::PRIMARY) {
        if let kamadak_exif::Value::Rational(rational_vec) = &field.value {
            if !rational_vec.is_empty() {
                metadata.f_number = Some(rational_vec[0].to_f32());
            }
        }
    }

    if let Some(field) = exif_data.get_field(kamadak_exif::Tag::PhotographicSensitivity, kamadak_exif::In::PRIMARY) {
        match &field.value {
            kamadak_exif::Value::Short(v) if !v.is_empty() => metadata.iso = Some(v[0]),
            kamadak_exif::Value::Long(v) if !v.is_empty() => metadata.iso = v[0].try_into().ok(),
            _ => {}
        }
    }

    if let Some(field) = exif_data.get_field(kamadak_exif::Tag::ExposureTime, kamadak_exif::In::PRIMARY) {
        metadata.exposure_time = match &field.value {
            kamadak_exif::Value::Rational(v) if !v.is_empty() => {
                let r = v[0];
                if r.denom != 0 { Some(format!("{}/{}", r.num, r.denom)) } else { Some(format!("{}", r.to_f32())) }
            }
            kamadak_exif::Value::SRational(v) if !v.is_empty() => {
                let r = v[0];
                if r.denom != 0 { Some(format!("{}/{}", r.num, r.denom)) } else { Some(format!("{}", r.to_f32())) }
            }
            _ => None,
        };
    }

    if let Some(field) = exif_data.get_field(kamadak_exif::Tag::FocalLength, kamadak_exif::In::PRIMARY) {
        if let kamadak_exif::Value::Rational(v) = &field.value {
            if let Some(r) = v.first() {
                metadata.focal_length_mm = Some(r.to_f32());
            }
        }
    }

    if let Some(field) = exif_data.get_field(kamadak_exif::Tag::FocalLengthIn35mmFilm, kamadak_exif::In::PRIMARY) {
        if let kamadak_exif::Value::Short(v) = &field.value {
            metadata.focal_length_35mm = v.first().copied();
        }
    }

    if let Some(field) = exif_data.get_field(kamadak_exif::Tag::Orientation, kamadak_exif::In::PRIMARY) {
        if let kamadak_exif::Value::Short(v) = &field.value {
            metadata.orientation = v.first().copied();
        }
    }

    let width_field = exif_data.get_field(kamadak_exif::Tag::PixelXDimension, kamadak_exif::In::PRIMARY)
        .or_else(|| exif_data.get_field(kamadak_exif::Tag::ImageWidth, kamadak_exif::In::PRIMARY));
    if let Some(field) = width_field {
        metadata.width = match &field.value {
            kamadak_exif::Value::Long(v) if !v.is_empty() => Some(v[0]),
            kamadak_exif::Value::Short(v) if !v.is_empty() => Some(v[0] as u32),
            _ => None,
        };
    }

    let height_field = exif_data.get_field(kamadak_exif::Tag::PixelYDimension, kamadak_exif::In::PRIMARY)
        .or_else(|| exif_data.get_field(kamadak_exif::Tag::ImageLength, kamadak_exif::In::PRIMARY));
    if let Some(field) = height_field {
        metadata.height = match &field.value {
            kamadak_exif::Value::Long(v) if !v.is_empty() => Some(v[0]),
            kamadak_exif::Value::Short(v) if !v.is_empty() => Some(v[0] as u32),
            _ => None,
        };
    }

    // Advanced & Forensic Tags
    let get_short = |tag: kamadak_exif::Tag| -> Option<u16> {
        exif_data.get_field(tag, kamadak_exif::In::PRIMARY)
            .and_then(|field| match &field.value {
                kamadak_exif::Value::Short(v) if !v.is_empty() => Some(v[0]),
                _ => None,
            })
    };

    metadata.body_serial_number = get_ascii_string(kamadak_exif::Tag::BodySerialNumber);
    metadata.lens_serial_number = get_ascii_string(kamadak_exif::Tag::LensSerialNumber);
    metadata.flash = get_short(kamadak_exif::Tag::Flash);
    metadata.exposure_program = get_short(kamadak_exif::Tag::ExposureProgram);
    metadata.metering_mode = get_short(kamadak_exif::Tag::MeteringMode);
    metadata.white_balance = get_short(kamadak_exif::Tag::WhiteBalance);
    metadata.light_source = get_short(kamadak_exif::Tag::LightSource);
    metadata.user_comment = exif_data.get_field(kamadak_exif::Tag::UserComment, kamadak_exif::In::PRIMARY)
        .map(|field| field.display_value().to_string());

    // GPS Parsing
    let lat_opt = exif_data.get_field(kamadak_exif::Tag::GPSLatitude, kamadak_exif::In::PRIMARY);
    let lat_ref_opt = exif_data.get_field(kamadak_exif::Tag::GPSLatitudeRef, kamadak_exif::In::PRIMARY);
    let lon_opt = exif_data.get_field(kamadak_exif::Tag::GPSLongitude, kamadak_exif::In::PRIMARY);
    let lon_ref_opt = exif_data.get_field(kamadak_exif::Tag::GPSLongitudeRef, kamadak_exif::In::PRIMARY);
    let alt_opt = exif_data.get_field(kamadak_exif::Tag::GPSAltitude, kamadak_exif::In::PRIMARY);
    let alt_ref_opt = exif_data.get_field(kamadak_exif::Tag::GPSAltitudeRef, kamadak_exif::In::PRIMARY);

    if let (Some(lat_field), Some(lat_ref_field), Some(lon_field), Some(lon_ref_field)) =
        (lat_opt, lat_ref_opt, lon_opt, lon_ref_opt)
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
                let mut gps = GpsInfo::default();
                let lat_neg = lat_ref_val[0][0].eq_ignore_ascii_case(&b'S');
                let lon_neg = lon_ref_val[0][0].eq_ignore_ascii_case(&b'W');

                gps.latitude = Some(crate::utils::dms_to_decimal(
                    lat_val[0].to_f64(),
                    lat_val[1].to_f64(),
                    lat_val[2].to_f64(),
                    lat_neg,
                ));
                gps.longitude = Some(crate::utils::dms_to_decimal(
                    lon_val[0].to_f64(),
                    lon_val[1].to_f64(),
                    lon_val[2].to_f64(),
                    lon_neg,
                ));

                if let Some(alt_field) = alt_opt {
                    if let kamadak_exif::Value::Rational(alt_val) = &alt_field.value {
                        if !alt_val.is_empty() {
                            let alt_value = alt_val[0].to_f32();
                            let alt_sign = if let Some(alt_ref_field) = alt_ref_opt {
                                if let kamadak_exif::Value::Byte(alt_ref_val) = &alt_ref_field.value {
                                    if !alt_ref_val.is_empty() && alt_ref_val[0] == 1 { -1.0 } else { 1.0 }
                                } else { 1.0 }
                            } else { 1.0 };
                            gps.altitude = Some(alt_value * alt_sign);
                        }
                    }
                }

                // Advanced GPS details
                let get_gps_ascii = |tag: kamadak_exif::Tag| -> Option<String> {
                    exif_data.get_field(tag, kamadak_exif::In::PRIMARY)
                        .and_then(|field| match &field.value {
                            kamadak_exif::Value::Ascii(v) if !v.is_empty() => {
                                std::str::from_utf8(&v[0]).ok().map(|s| s.trim().to_string())
                            }
                            _ => None,
                        })
                };

                let get_gps_rational = |tag: kamadak_exif::Tag| -> Option<f64> {
                    exif_data.get_field(tag, kamadak_exif::In::PRIMARY)
                        .and_then(|field| match &field.value {
                            kamadak_exif::Value::Rational(v) if !v.is_empty() => Some(v[0].to_f64()),
                            _ => None,
                        })
                };

                gps.speed = get_gps_rational(kamadak_exif::Tag::GPSSpeed);
                gps.speed_ref = get_gps_ascii(kamadak_exif::Tag::GPSSpeedRef);
                gps.track = get_gps_rational(kamadak_exif::Tag::GPSTrack);
                gps.track_ref = get_gps_ascii(kamadak_exif::Tag::GPSTrackRef);
                gps.img_direction = get_gps_rational(kamadak_exif::Tag::GPSImgDirection);
                gps.img_direction_ref = get_gps_ascii(kamadak_exif::Tag::GPSImgDirectionRef);
                gps.date_stamp = get_gps_ascii(kamadak_exif::Tag::GPSDateStamp);

                if let Some(field) = exif_data.get_field(kamadak_exif::Tag::GPSTimeStamp, kamadak_exif::In::PRIMARY) {
                    if let kamadak_exif::Value::Rational(val) = &field.value {
                        if val.len() >= 3 {
                            gps.time_stamp = Some(format!(
                                "{:02.0}:{:02.0}:{:02.0}",
                                val[0].to_f32(), val[1].to_f32(), val[2].to_f32()
                            ));
                        }
                    }
                }

                metadata.gps = Some(gps);
            }
        }
    }

    metadata
}
