use std::fs;
use std::path::{Path, PathBuf};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub fn unique_output_path(
    output_dir: &Path,
    input_path: &Path,
    suffix: &str,
    ext: &str,
) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let ts = OffsetDateTime::now_local()
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
        .format(&Rfc3339)
        .unwrap_or_else(|_| "now".to_string())
        .replace(':', "-");
    let filename = if suffix.is_empty() {
        format!("{stem}_{ts}.{ext}")
    } else {
        format!("{stem}_{suffix}_{ts}.{ext}")
    };
    output_dir.join(filename)
}

pub fn save_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, bytes)
}
