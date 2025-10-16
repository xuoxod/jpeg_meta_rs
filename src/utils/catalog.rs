use crate::JpegMetadata;
use crate::parse_metadata;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::print::{FieldSet, available_fields};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CatalogEntry {
    pub path: String,
    pub metadata: JpegMetadata,
}

pub fn list_jpeg_files(dir: &Path, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if recursive {
                    stack.push(path);
                }
                continue;
            }
            if let Some(ext) = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase())
            {
                if ext == "jpg" || ext == "jpeg" {
                    out.push(path);
                }
            }
        }
    }
    Ok(out)
}

pub fn index_directory(dir: &Path, recursive: bool) -> Result<Vec<CatalogEntry>, String> {
    let files = list_jpeg_files(dir, recursive).map_err(|e| e.to_string())?;
    index_paths(&files)
}

pub fn index_paths(paths: &[PathBuf]) -> Result<Vec<CatalogEntry>, String> {
    let mut entries = Vec::new();
    for p in paths {
        match fs::read(p) {
            Ok(bytes) => match parse_metadata(&bytes) {
                Ok(meta) => entries.push(CatalogEntry {
                    path: p.display().to_string(),
                    metadata: meta,
                }),
                Err(_e) => {
                    // Skip files we cannot parse; could log if needed
                }
            },
            Err(_e) => {
                // Skip unreadable files
            }
        }
    }
    Ok(entries)
}

fn meta_kv_map(meta: &JpegMetadata) -> HashMap<&'static str, String> {
    let mut rows: HashMap<&'static str, String> = HashMap::new();
    let mut add = |name: &'static str, value: Option<String>| {
        if let Some(v) = value {
            rows.insert(name, v);
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

    rows
}

pub fn entries_to_csv(entries: &[CatalogEntry], fields: &FieldSet) -> String {
    let keys: Vec<&str> = match fields {
        FieldSet::All => available_fields().into_iter().map(|(k, _)| k).collect(),
        FieldSet::Names(v) => v.iter().map(|s| s.as_str()).collect(),
    };
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(vec![]);
    let mut header = vec!["path".to_string()];
    header.extend(keys.iter().map(|k| k.to_string()));
    let _ = wtr.write_record(header);
    for e in entries {
        let map = meta_kv_map(&e.metadata);
        let mut row = vec![e.path.clone()];
        for k in &keys {
            row.push(map.get(k).cloned().unwrap_or_default());
        }
        let _ = wtr.write_record(row);
    }
    let data = wtr.into_inner().unwrap_or_default();
    String::from_utf8_lossy(&data).to_string()
}

pub fn entries_to_markdown(entries: &[CatalogEntry], fields: &FieldSet) -> String {
    let keys: Vec<&str> = match fields {
        FieldSet::All => available_fields().into_iter().map(|(k, _)| k).collect(),
        FieldSet::Names(v) => v.iter().map(|s| s.as_str()).collect(),
    };
    let mut s = String::new();
    // header
    s.push_str("| path |");
    for k in &keys {
        s.push_str(" ");
        s.push_str(k);
        s.push_str(" |");
    }
    s.push('\n');
    s.push_str("|");
    s.push_str(&"---|".repeat(keys.len() + 1));
    s.push('\n');
    for e in entries {
        let map = meta_kv_map(&e.metadata);
        s.push_str("|");
        s.push_str(&e.path.replace('|', "\\|"));
        s.push_str("|");
        for k in &keys {
            let v = map.get(k).cloned().unwrap_or_default().replace('|', "\\|");
            s.push_str(" ");
            s.push_str(&v);
            s.push_str(" |");
        }
        s.push('\n');
    }
    s
}

pub fn entries_to_paths(entries: &[CatalogEntry]) -> String {
    let mut s = String::new();
    for e in entries {
        s.push_str(&e.path);
        s.push('\n');
    }
    s
}

pub fn entries_to_jsonl(entries: &[CatalogEntry]) -> Result<String, String> {
    let mut s = String::new();
    for e in entries {
        let line = serde_json::to_string(e).map_err(|e| e.to_string())?;
        s.push_str(&line);
        s.push('\n');
    }
    Ok(s)
}

pub fn entries_to_tsv(entries: &[CatalogEntry], fields: &FieldSet) -> String {
    let keys: Vec<&str> = match fields {
        FieldSet::All => available_fields().into_iter().map(|(k, _)| k).collect(),
        FieldSet::Names(v) => v.iter().map(|s| s.as_str()).collect(),
    };
    let mut s = String::new();
    s.push_str("path");
    for k in &keys {
        s.push('\t');
        s.push_str(k);
    }
    s.push('\n');
    for e in entries {
        let map = meta_kv_map(&e.metadata);
        s.push_str(&e.path);
        for k in &keys {
            s.push('\t');
            let mut v = map.get(k).cloned().unwrap_or_default();
            v = v.replace('\t', " ");
            s.push_str(&v);
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entries_to_tsv_basic() {
        let entry = CatalogEntry {
            path: "/tmp/a.jpg".to_string(),
            metadata: JpegMetadata {
                camera_make: Some("Nikon".to_string()),
                camera_model: Some("D750".to_string()),
                ..Default::default()
            },
        };
        let tsv = entries_to_tsv(
            &[entry],
            &FieldSet::Names(vec!["camera_make".to_string(), "camera_model".to_string()]),
        );
        let lines: Vec<&str> = tsv.trim_end().split('\n').collect();
        assert_eq!(lines[0], "path\tcamera_make\tcamera_model");
        assert!(lines[1].starts_with("/tmp/a.jpg\tNikon\tD750"));
    }
}
