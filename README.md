# jpeg_meta_rs

A small Rust library and CLI to extract common EXIF metadata from JPEG files using the `kamadak-exif` crate.

## Features

- Extract a wide set of EXIF fields (make/model, f-number, ISO, exposure time, focal lengths, lens, date/time, orientation, dimensions, software, GPS)
- Multiple output formats: table, JSON, CSV, Markdown, TSV
- Filter which fields to display/export with `--fields` (e.g., `camera_make,iso` or `all`)
- Save outputs with unique filenames (format independent of display via `--save-format`)
- Optional JPEG segment listing (`--segments`)
- Index a directory of JPEGs into a catalog and export JSON/CSV/Markdown/TSV, JSON Lines, or paths-only
- Gracefully returns default metadata when EXIF is not present

## Install and Build

Requirements: Rust toolchain with 2024 edition support (recent stable), Cargo

```bash
# Build
cargo build

# Run (parse an image)
cargo run -- path/to/image.jpg
```

## CLI

Key arguments:

- `<FILE_PATH>`: path to a JPEG. Omit when using `--list-fields` or `--index-dir`.
- `--format [table|json|csv|md|tsv|debug]`: choose display format (default: `table`).
- `--fields <list|all>`: limit fields by comma-separated keys or use `all` (default `all`).
- `--output-dir <DIR>`: save output files here (filenames are unique and derived from the image path).
- `--save-format [json|txt|csv|md|tsv]`: file format to save regardless of display.
- `--segments`: list JPEG segments.
- `--list-fields`: list available field keys and exit.
- `--index-dir <DIR>`: build a catalog by scanning a directory of JPEGs.
- `--recursive`: recurse when indexing.
- `--export-paths`: when indexing, output only paths (one per line).
- `--export-jsonl`: when indexing, output JSON Lines (one entry per line).

Examples:

```bash
# Default table output
cargo run -- path/to/image.jpg

# JSON output
cargo run -- --format json path/to/image.jpg

# CSV/Markdown/TSV with selected fields
cargo run -- --format csv --fields camera_make,iso path/to/image.jpg
cargo run -- --format md --fields all path/to/image.jpg
cargo run -- --format tsv --fields camera_make,camera_model,f_number path/to/image.jpg

# Save outputs (independent of display format)
cargo run -- --format table --save-format json --output-dir out path/to/image.jpg

# List available field keys
cargo run -- --list-fields

# Index a directory (flat) and print JSON catalog
cargo run -- --index-dir ./photos --format json

# Index recursively and export only paths
cargo run -- --index-dir ./photos --recursive --export-paths --output-dir out

# Index and export JSON Lines (one per line)
cargo run -- --index-dir ./photos --export-jsonl --output-dir out

# Index and display TSV, saving Markdown
cargo run -- --index-dir ./photos --format tsv --save-format md --output-dir out
```

## Library

```rust
use jpeg_meta_rs::parse_metadata;

let bytes = std::fs::read("path/to/image.jpg").unwrap();
let meta = parse_metadata(&bytes).unwrap();
println!("{:?#}", meta);
```

### Data structures

- `JpegMetadata` with many common fields, including:
  - `camera_make: Option<String>`
  - `camera_model: Option<String>`
  - `f_number: Option<f32>`
  - `iso: Option<u16>`
  - `exposure_time: Option<String>`
  - `focal_length: Option<f32>`
  - `focal_length_35mm: Option<u16>`
  - `lens_make: Option<String>`
  - `lens_model: Option<String>`
  - `date_time_original: Option<String>`
  - `orientation: Option<String>`
  - `pixel_x_dimension: Option<u32>`
  - `pixel_y_dimension: Option<u32>`
  - `software: Option<String>`
  - `gps: Option<GpsInfo>` where `GpsInfo` contains `latitude`, `longitude`, `altitude`

### Errors

- `ParseError` maps IO and EXIF errors; no EXIF is treated as success with default metadata

## Tests

This repository includes test scaffolding that expects image fixtures:

1. Create `test_images/` in the project root.
2. Add a JPEG with EXIF (e.g., `sample_with_exif.jpg`) and one without EXIF (`sample_no_exif.jpg`).
3. Update the expected values in `src/lib.rs` test `test_parse_basic_exif` to match your fixture.

Run tests:

```bash
cargo test
```

## License

MIT
