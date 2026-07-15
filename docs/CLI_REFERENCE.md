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
| `--list-editable` | N/A | List all editable/deletable metadata fields in the file with copy-pasteable example edit commands. | N/A |
| `--set-comment` | N/A | Set or update the image comment (JPEG COM segment or PNG 'Comment' text chunk). | String comment value |
| `--set-text` | N/A | Set or update custom PNG text metadata key-value pair. | `KEYWORD:VALUE` |
| `--delete-text` | N/A | Delete a PNG text metadata keyword or the JPEG comment. | Keyword to delete |
| `--out` | `-o` | Output file path for edited or sanitized images. | Output file path |

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

### ✏️ Metadata Editing & Creation
Add, modify, or delete comments and text metadata keywords for JPEG and PNG.

#### 💡 Discovering Editable Fields (Non-Technical User Helper)
If you do not know which metadata fields exist in an image file, use the `--list-editable` flag. It lists all editable tags and prints copy-pasteable command examples customized for your file:
```bash
./analyzer --list-editable original.png
```
*Expected Console Output:*
```text
ℹ️ Editable metadata fields in 'original.png':
  - 'Author' (currently: "Rick Walker")

✏️ Example commands to edit:
  • To set/create a tag:     ./analyzer --set-text "Author:John Doe" original.png
  • To set general comment:   ./analyzer --set-comment "My Comment" original.png
  • To delete a tag:         ./analyzer --delete-text Author original.png
```

#### 🛡️ Auto-Copy Logic Sugar (OOM / Safety Resolution)
To protect original files from accidental modifications, if you execute an edit command **without** specifying an output destination `-o / --out`, the program automatically writes to a copy (appending `_edited` to the name) in the same directory:
```bash
./analyzer --set-comment "Copyright 2026 Walkers" original.jpg
# Automatically saves to: original_edited.jpg
```
If the edited target filename already exists, the engine's built-in collision handler automatically resolves the conflict (e.g. saving to `original_edited_copy_1.jpg`) to avoid overwriting existing work.

If you explicitly wish to write to a custom filename, specify `-o / --out`:
```bash
./analyzer --set-text "Author:Rick Walker" -o output.png original.png
```


