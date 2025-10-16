# jpeg_meta_rs

A small Rust library and CLI to extract common EXIF metadata from JPEG files using the `kamadak-exif` crate.

## Features

- Extract camera make and model
- F-number (aperture)
- ISO (Photographic Sensitivity)
- GPS (latitude, longitude in decimal degrees; altitude with above/below sea level)
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

```text
Usage: jpeg_meta_rs <FILE_PATH>
```

- Validates the path exists and is a file
- Warns if extension is not .jpg/.jpeg but still attempts parsing
- Prints parsed metadata in a pretty Debug format

## Library

```rust
use jpeg_meta_rs::parse_metadata;

let bytes = std::fs::read("path/to/image.jpg").unwrap();
let meta = parse_metadata(&bytes).unwrap();
println!("{:?#}", meta);
```

### Data structures

- `JpegMetadata` with fields:
  - `camera_make: Option<String>`
  - `camera_model: Option<String>`
  - `f_number: Option<f32>`
  - `iso: Option<u16>`
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
