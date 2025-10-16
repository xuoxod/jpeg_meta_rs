use crate::JpegMetadata;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Row, Table, presets::UTF8_FULL};
use std::fmt::Write as _;

pub fn metadata_to_table(meta: &JpegMetadata) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Field")
                .fg(Color::Green)
                .add_attribute(Attribute::Bold),
            Cell::new("Value")
                .fg(Color::Blue)
                .add_attribute(Attribute::Bold),
        ]);

    let mut add = |name: &str, value: Option<String>| {
        if let Some(v) = value {
            table.add_row(Row::from(vec![Cell::new(name), Cell::new(v)]));
        }
    };

    add("Camera Make", meta.camera_make.clone());
    add("Camera Model", meta.camera_model.clone());
    if let Some(gps) = meta.gps.as_ref() {
        add("Latitude", gps.latitude.map(|v| format!("{v:.6}")));
        add("Longitude", gps.longitude.map(|v| format!("{v:.6}")));
        add("Altitude (m)", gps.altitude.map(|v| format!("{v}")));
    }
    add("F-Number", meta.f_number.map(|v| format!("{v}")));
    add("ISO", meta.iso.map(|v| v.to_string()));
    add("Exposure Time", meta.exposure_time.clone());
    add(
        "Focal Length (mm)",
        meta.focal_length_mm.map(|v| format!("{v}")),
    );
    add(
        "Focal Length (35mm)",
        meta.focal_length_35mm.map(|v| v.to_string()),
    );
    add("Lens Make", meta.lens_make.clone());
    add("Lens Model", meta.lens_model.clone());
    add("Date/Time Original", meta.date_time_original.clone());
    add("Orientation", meta.orientation.map(|v| v.to_string()));
    add("Width (px)", meta.width.map(|v| v.to_string()));
    add("Height (px)", meta.height.map(|v| v.to_string()));
    add("Software", meta.software.clone());

    table
}

pub enum FieldSet {
    All,
    Names(Vec<String>),
}

/// Returns the list of available field keys and their human-friendly labels.
/// Keys can be used with the `--fields` option.
pub fn available_fields() -> Vec<(&'static str, &'static str)> {
    vec![
        ("camera_make", "Camera Make"),
        ("camera_model", "Camera Model"),
        ("gps.latitude", "Latitude"),
        ("gps.longitude", "Longitude"),
        ("gps.altitude", "Altitude (m)"),
        ("f_number", "F-Number"),
        ("iso", "ISO"),
        ("exposure_time", "Exposure Time"),
        ("focal_length_mm", "Focal Length (mm)"),
        ("focal_length_35mm", "Focal Length (35mm)"),
        ("lens_make", "Lens Make"),
        ("lens_model", "Lens Model"),
        ("date_time_original", "Date/Time Original"),
        ("orientation", "Orientation"),
        ("width", "Width (px)"),
        ("height", "Height (px)"),
        ("software", "Software"),
    ]
}

pub fn filter_fields(meta: &JpegMetadata, fields: &FieldSet) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut add = |name: &str, value: Option<String>| {
        if let Some(v) = value {
            rows.push((name.to_string(), v));
        }
    };

    add("camera_make", meta.camera_make.clone());
    add("camera_model", meta.camera_model.clone());
    if let Some(gps) = meta.gps.as_ref() {
        add("gps.latitude", gps.latitude.map(|v| format!("{v:.6}")));
        add("gps.longitude", gps.longitude.map(|v| format!("{v:.6}")));
        add("gps.altitude", gps.altitude.map(|v| format!("{v}")));
    }
    add("f_number", meta.f_number.map(|v| format!("{v}")));
    add("iso", meta.iso.map(|v| v.to_string()));
    add("exposure_time", meta.exposure_time.clone());
    add(
        "focal_length_mm",
        meta.focal_length_mm.map(|v| format!("{v}")),
    );
    add(
        "focal_length_35mm",
        meta.focal_length_35mm.map(|v| v.to_string()),
    );
    add("lens_make", meta.lens_make.clone());
    add("lens_model", meta.lens_model.clone());
    add("date_time_original", meta.date_time_original.clone());
    add("orientation", meta.orientation.map(|v| v.to_string()));
    add("width", meta.width.map(|v| v.to_string()));
    add("height", meta.height.map(|v| v.to_string()));
    add("software", meta.software.clone());

    match fields {
        FieldSet::All => rows,
        FieldSet::Names(wanted) => {
            let set: std::collections::HashSet<&str> = wanted.iter().map(|s| s.as_str()).collect();
            rows.into_iter()
                .filter(|(k, _)| set.contains(k.as_str()))
                .collect()
        }
    }
}

pub fn rows_to_csv(rows: &[(String, String)]) -> String {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(vec![]);
    let _ = wtr.write_record(["field", "value"]);
    for (k, v) in rows {
        let _ = wtr.write_record([k, v]);
    }
    let data = wtr.into_inner().unwrap_or_default();
    String::from_utf8_lossy(&data).to_string()
}

pub fn rows_to_markdown(rows: &[(String, String)]) -> String {
    let mut s = String::new();
    let _ = writeln!(&mut s, "| Field | Value |");
    let _ = writeln!(&mut s, "|---|---|");
    for (k, v) in rows {
        let _ = writeln!(&mut s, "| {} | {} |", k, v.replace('|', "\\|"));
    }
    s
}

pub fn rows_to_tsv(rows: &[(String, String)]) -> String {
    let mut s = String::new();
    s.push_str("field\tvalue\n");
    for (k, v) in rows {
        let v = v.replace('\t', " ");
        let _ = writeln!(&mut s, "{}\t{}", k, v);
    }
    s
}

// Plain text output is handled via comfy-table to_string for now.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rows_to_tsv_basic() {
        let rows = vec![
            ("camera_make".to_string(), "Canon".to_string()),
            ("iso".to_string(), "400".to_string()),
        ];
        let tsv = rows_to_tsv(&rows);
        let lines: Vec<&str> = tsv.trim_end().split('\n').collect();
        assert_eq!(lines[0], "field\tvalue");
        assert!(lines.iter().any(|l| *l == "camera_make\tCanon"));
        assert!(lines.iter().any(|l| *l == "iso\t400"));
    }
}
