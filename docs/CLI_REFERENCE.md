# 🛠️ Command-Line Interface Reference

The `jpeg_meta_rs` CLI provides a flexible, high-contrast, multi-file analyzer for JPEG and PNG images.

---

## 📡 1. Global Syntax

```bash
./analyzer [FLAGS] [OPTIONS] <FILES>...
```

### 📥 Arguments
*   `<FILES>...`: One or more paths to JPEG/PNG files to analyze.

---

## ⚙️ 2. Option Matrix

| Option / Flag | Short | Description | Allowed Values |
|---|---|---|---|
| `--format` | `-f` | Output presentation format. | `table`, `json` |
| `--file-type` | `-t` | Force parsing file as a specific format. | `auto`, `jpeg`, `png` |
| `--exclude-structure`| N/A | Hide the segment/chunk layout table. | N/A |
| `--exclude-properties`| N/A | Hide basic image properties table. | N/A |
| `--exclude-exif` | N/A | Hide EXIF metadata parameters. | N/A |
| `--exclude-xmp` | N/A | Hide raw XMP XML metadata. | N/A |
| `--exclude-text` | N/A | Hide PNG textual chunks (PNG only). | N/A |
| `--filter-keys` | `-k` | Filter specific metadata fields to print. | Comma-separated list (e.g., `gps,iso,make`) |
| `--raw-sizes` | N/A | Display sizes in raw bytes instead of human-readable formats. | N/A |
| `--exclude-embedded` | N/A | Hide the embedded payloads scanner table. | N/A |

---

## 🚀 3. Usage Scenarios

### 📸 Multi-File Scanning
Analyze multiple files simultaneously. Each file output is separated by a bold border:
```bash
./analyzer image1.jpg image2.png
```

### 🔍 Metadata Scaling & Exclusion
To scale back output and print only EXIF metadata without segment structure maps or raw XML blocks:
```bash
./analyzer --exclude-structure --exclude-properties --exclude-xmp image1.jpg
```

### 🎯 Smart Key Filtering
Filter metadata fields using substring matching (case-insensitive). For example, to view only GPS coordinates and ISO values:
```bash
./analyzer --filter-keys gps,iso image1.jpg
```
*Outputs:*
*   `GPS Latitude`
*   `GPS Longitude`
*   `GPS Altitude`
*   `ISO`

### 💾 Structured JSON Pipeline
Outputs a single JSON dictionary mapping file paths to their extracted analysis:
```bash
./analyzer --format json image1.jpg image2.png
```
