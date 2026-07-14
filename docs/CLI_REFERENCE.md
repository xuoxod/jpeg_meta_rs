# 🛠️ Command-Line Interface Reference

The `jpeg_meta_rs` CLI provides a unified interface to execute the JPEG and PNG parsers.

---

## 📡 1. Global Syntax

```bash
cargo run -- [FLAGS] [OPTIONS] <FILE_PATH>
```

### 📥 Arguments
*   `<FILE_PATH>`: Path to a JPEG or PNG file.

---

## ⚙️ 2. Option Matrix

| Flag / Option | Description | Allowed Values / Formats |
|---|---|---|
| `-h, --help` | Display syntax help and examples. | N/A |
| `-V, --version` | Display executable version. | N/A |
| `-f, --format` | Set output presentation format (default: `table`). | `table`, `json` |
| `-f, --file-type` | Force parser type instead of auto-detecting signature. | `auto`, `jpeg`, `png` |
| `--structure-only` | Scan and output only the segment/chunk layout. | N/A |

---

## 🚀 3. Usage Scenarios

### 📸 JPEG Metadata Extraction

#### Display Standard Tables
Extract segment structure, dimensions, and EXIF parameters from a JPEG:
```bash
cargo run -- test_images/img6-gps.jpg
```

#### Output Structured JSON Records
Serialize segment coordinates and EXIF tags to JSON:
```bash
cargo run -- --format json test_images/img6-gps.jpg
```

---

### 🖼️ PNG Metadata Extraction

#### Display Chunks and Text tags
Extract chunks, header properties, resolution, text metadata, and embedded EXIF parameters:
```bash
cargo run -- sample.png
```

#### Scan Chunks Layout Only
Inspect chunk offsets, lengths, and CRC status:
```bash
cargo run -- --structure-only sample.png
```
