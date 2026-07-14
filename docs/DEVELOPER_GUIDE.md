# 💻 Developer & Contribution Guide

This guide outlines setup instructions, testing frameworks, and coding standards for developers working on the `jpeg_meta_rs` workspace.

---

## 🛠️ 1. Compiling & Building

Compile the project workspace using:

```bash
cargo build --release
```

This compiles an optimized release binary located at `target/release/jpeg_meta_rs`.

---

## 🔬 2. Test-Driven Development (TDD)

We utilize a comprehensive TDD harness covering format converters, parsing anomalies, and segment lookups. 

### Running Tests
Execute the test runner script:
```bash
cargo test
```

---

## 📂 3. Mock Test Fixtures Setup

Our integration tests check raw binary parsing against known EXIF assets. If you are running tests on a clean environment:

1.  Create a directory named `test_images/` in the crate root:
    ```bash
    mkdir -p test_images/
    ```
2.  Add a JPEG with known EXIF coordinates named `img6-gps.jpg` inside the `test_images/` directory.

These files are ignored by git in `.gitignore` to prevent leaking personal PII metadata.

---

## 🚀 4. Customizing Parsers

### Adding support for new JPEG APP markers
Update `parse_jpeg` in `src/jpeg.rs`:
1. Add a case in the `marker` match block:
   ```rust
   0xE3 => "APP3 (Custom Application)".to_string(),
   ```

### Adding support for new PNG chunks
Update `parse_png` in `src/png.rs`:
1. Add a case to the chunk type match block:
   ```rust
   "gAMA" => {
       // Parse image gamma
   }
   ```
