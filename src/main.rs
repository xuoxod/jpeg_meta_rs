#![allow(clippy::collapsible_if)]
use clap::{Parser, ValueEnum, ValueHint};
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table, presets::UTF8_FULL};
use jpeg_meta_rs::jpeg::{parse_jpeg, JpegInfo};
use jpeg_meta_rs::png::{parse_png, PngInfo};
use jpeg_meta_rs::common::ExifMetadata;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
enum OutputFormat {
    Table,
    Json,
}

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
enum FileType {
    Auto,
    Jpeg,
    Png,
}

#[derive(Parser, Debug)]
#[command(
    name = "jpeg_meta_rs",
    author = "Rick Walker <ichglauben@gmail.com>",
    version,
    about = "Extract structure, headers, comments, text chunks, and EXIF metadata from JPEGs and PNGs.",
    long_about = "A dual-format analyzer and metadata parser that handles JPEG and PNG files. Runs segment/chunk scans, extracts raw headers, parses text tags, and decodes full EXIF blocks."
)]
struct Args {
    /// Paths to the JPEG or PNG files to analyze
    #[arg(value_name = "FILES", value_hint = ValueHint::FilePath, required = true)]
    file_paths: Vec<PathBuf>,

    /// Output format
    #[arg(long, short, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,

    /// Force parsing file as specific type instead of auto-detecting signature
    #[arg(long, short, value_enum, default_value_t = FileType::Auto)]
    file_type: FileType,

    /// Exclude the structural segments/chunks table from print layout
    #[arg(long)]
    exclude_structure: bool,

    /// Exclude basic image properties from print layout
    #[arg(long)]
    exclude_properties: bool,

    /// Exclude EXIF metadata parameters from print layout
    #[arg(long)]
    exclude_exif: bool,

    /// Exclude raw XMP block from print layout
    #[arg(long)]
    exclude_xmp: bool,

    /// Exclude embedded text records from print layout (PNG only)
    #[arg(long)]
    exclude_text: bool,

    /// Filter specific metadata keys to display (comma-separated, matches substrings)
    #[arg(long, short = 'k', value_name = "KEYS")]
    filter_keys: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum FileAnalysis {
    Jpeg(JpegInfo),
    Png(PngInfo),
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    // Parse filter list
    let filter_list: Option<Vec<String>> = args.filter_keys.as_ref().map(|s| {
        s.split(',')
            .map(|k| k.trim().to_lowercase())
            .filter(|k| !k.is_empty())
            .collect()
    });

    let mut dictionary = std::collections::BTreeMap::new();
    let mut errors = Vec::new();

    for path in &args.file_paths {
        if !path.exists() {
            errors.push(format!("File not found: '{}'", path.display()));
            continue;
        }
        if !path.is_file() {
            errors.push(format!("Path is not a file: '{}'", path.display()));
            continue;
        }

        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("Error reading '{}': {e}", path.display()));
                continue;
            }
        };

        if bytes.is_empty() {
            errors.push(format!("File is empty: '{}'", path.display()));
            continue;
        }

        // Determine file type
        let resolved_type = match args.file_type {
            FileType::Jpeg => FileType::Jpeg,
            FileType::Png => FileType::Png,
            FileType::Auto => {
                if bytes.len() >= 8 && bytes[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
                    FileType::Png
                } else if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
                    FileType::Jpeg
                } else {
                    let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
                    match ext.as_deref() {
                        Some("png") => FileType::Png,
                        Some("jpg") | Some("jpeg") => FileType::Jpeg,
                        _ => {
                            errors.push(format!("Could not auto-detect format for '{}'. Specify format with --file-type.", path.display()));
                            continue;
                        }
                    }
                }
            }
        };

        match resolved_type {
            FileType::Jpeg => {
                match parse_jpeg(&bytes) {
                    Ok(info) => {
                        dictionary.insert(path.to_string_lossy().to_string(), FileAnalysis::Jpeg(info));
                    }
                    Err(e) => {
                        errors.push(format!("JPEG parsing error on '{}': {e}", path.display()));
                    }
                }
            }
            FileType::Png => {
                match parse_png(&bytes) {
                    Ok(info) => {
                        dictionary.insert(path.to_string_lossy().to_string(), FileAnalysis::Png(info));
                    }
                    Err(e) => {
                        errors.push(format!("PNG parsing error on '{}': {e}", path.display()));
                    }
                }
            }
            FileType::Auto => unreachable!(),
        }
    }

    if args.format == OutputFormat::Json {
        let json = serde_json::to_string_pretty(&dictionary)
            .map_err(|e| format!("JSON serialization error: {e}"))?;
        println!("{json}");
    } else {
        // Pretty print Table format for each file
        for (i, (path_str, analysis)) in dictionary.iter().enumerate() {
            let path = Path::new(path_str);
            if i > 0 {
                // SPACIOUS AND BOLD INTER-FILE BOUNDARY
                println!();
                println!("{}", "=".repeat(80));
                println!();
            }

            match analysis {
                FileAnalysis::Jpeg(info) => {
                    print_jpeg_tables(path, info, &args, &filter_list);
                }
                FileAnalysis::Png(info) => {
                    print_png_tables(path, info, &args, &filter_list);
                }
            }
        }
    }

    if !errors.is_empty() {
        eprintln!();
        eprintln!("⚠️ Processing anomalies occurred during run:");
        for err in errors {
            eprintln!(" - {err}");
        }
        return Err("Execution completed with errors.".to_string());
    }

    Ok(())
}

use jpeg_meta_rs::utils::matches_filter;

fn print_jpeg_tables(path: &Path, info: &JpegInfo, args: &Args, filter: &Option<Vec<String>>) {
    println!("File: {}", path.display());
    println!("Type: JPEG Image Container");
    println!();

    // 1. Structure Segment Table
    if !args.exclude_structure {
        let mut seg_table = Table::new();
        seg_table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Marker Offset").fg(Color::Green).add_attribute(Attribute::Bold),
                Cell::new("Marker Byte").fg(Color::Green).add_attribute(Attribute::Bold),
                Cell::new("Segment Name").fg(Color::Green).add_attribute(Attribute::Bold),
                Cell::new("Segment Length (Bytes)").fg(Color::Green).add_attribute(Attribute::Bold),
            ]);

        for seg in &info.segments {
            seg_table.add_row(vec![
                format!("0x{:08X}", seg.offset),
                format!("0xFF{:02X}", seg.marker),
                seg.name.clone(),
                seg.length.to_string(),
            ]);
        }
        println!("📂 JPEG Segment Structure map:");
        println!("{seg_table}");
        println!();
    }

    // 2. JPEG Header Properties Table
    if !args.exclude_properties {
        let mut header_table = Table::new();
        header_table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Property").fg(Color::Blue).add_attribute(Attribute::Bold),
                Cell::new("Value").fg(Color::Blue).add_attribute(Attribute::Bold),
            ]);

        let mut has_properties = false;
        let mut add_prop = |name: &str, value: Option<String>| {
            if let Some(v) = value {
                if matches_filter(name, filter) {
                    header_table.add_row(vec![name, &v]);
                    has_properties = true;
                }
            }
        };

        add_prop("Width", info.width.map(|w| format!("{w} px")));
        add_prop("Height", info.height.map(|h| format!("{h} px")));
        add_prop("Data Precision", info.precision.map(|p| format!("{p} bits")));
        add_prop("Color Channels", info.channels.map(|c| match c {
            1 => "Grayscale (1)".to_string(),
            3 => "RGB / YCbCr (3)".to_string(),
            4 => "CMYK (4)".to_string(),
            other => format!("Custom ({other})"),
        }));
        add_prop("Comment Payload", info.comment.clone());

        if has_properties {
            println!("ℹ️ Image properties:");
            println!("{header_table}");
            println!();
        }
    }

    // 3. EXIF Tags Table
    if !args.exclude_exif {
        print_exif_table(&info.metadata, filter);
    }

    // 4. XMP Metadata Block
    if !args.exclude_xmp {
        if let Some(ref xmp) = info.metadata.xmp {
            if matches_filter("xmp", filter) {
                println!("📜 Embedded XMP Metadata Block:");
                println!("{xmp}");
                println!();
            }
        }
    }
}

fn print_png_tables(path: &Path, info: &PngInfo, args: &Args, filter: &Option<Vec<String>>) {
    println!("File: {}", path.display());
    println!("Type: Portable Network Graphics (PNG)");
    println!();

    // 1. Structure Chunks Table
    if !args.exclude_structure {
        let mut chunk_table = Table::new();
        chunk_table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Offset").fg(Color::Green).add_attribute(Attribute::Bold),
                Cell::new("Chunk Type").fg(Color::Green).add_attribute(Attribute::Bold),
                Cell::new("Payload Length (Bytes)").fg(Color::Green).add_attribute(Attribute::Bold),
                Cell::new("CRC Value").fg(Color::Green).add_attribute(Attribute::Bold),
                Cell::new("CRC Status").fg(Color::Green).add_attribute(Attribute::Bold),
            ]);

        for chunk in &info.chunks {
            let crc_status = if chunk.crc_valid {
                Cell::new("Valid").fg(Color::Green)
            } else {
                Cell::new("CORRUPT").fg(Color::Red).add_attribute(Attribute::Bold)
            };
            chunk_table.add_row(vec![
                Cell::new(format!("0x{:08X}", chunk.offset)),
                Cell::new(chunk.type_name.clone()),
                Cell::new((chunk.length - 12).to_string()),
                Cell::new(format!("0x{:08X}", chunk.crc)),
                crc_status,
            ]);
        }
        println!("📂 PNG Chunk Structure map:");
        println!("{chunk_table}");
        println!();
    }

    // 2. PNG Header Properties Table
    if !args.exclude_properties {
        let mut header_table = Table::new();
        header_table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Property").fg(Color::Blue).add_attribute(Attribute::Bold),
                Cell::new("Value").fg(Color::Blue).add_attribute(Attribute::Bold),
            ]);

        let mut has_properties = false;
        let mut add_prop = |name: &str, value: Option<String>| {
            if let Some(v) = value {
                if matches_filter(name, filter) {
                    header_table.add_row(vec![name, &v]);
                    has_properties = true;
                }
            }
        };

        if let Some(ref ihdr) = info.header {
            add_prop("Width", Some(format!("{} px", ihdr.width)));
            add_prop("Height", Some(format!("{} px", ihdr.height)));
            add_prop("Bit Depth", Some(format!("{} bits/channel", ihdr.bit_depth)));
            let color_name = match ihdr.color_type {
                0 => "Grayscale (0)".to_string(),
                2 => "Truecolor RGB (2)".to_string(),
                3 => "Indexed Color (3)".to_string(),
                4 => "Grayscale with Alpha (4)".to_string(),
                6 => "Truecolor RGBA (6)".to_string(),
                other => format!("Unknown ({other})"),
            };
            add_prop("Color Type", Some(color_name));
            add_prop("Interlace Method", Some(if ihdr.interlace_method == 1 { "Adam7 Interlace".to_string() } else { "No Interlace".to_string() }));
        }

        add_prop("Last Mod Time (tIME)", info.modification_time.clone());

        if let Some(ref res) = info.resolution {
            let unit_name = if res.unit_specifier == 1 { "Meters" } else { "Unknown Unit" };
            add_prop("Pixels Per Unit X", Some(format!("{} / {}", res.ppu_x, unit_name)));
            add_prop("Pixels Per Unit Y", Some(format!("{} / {}", res.ppu_y, unit_name)));
            if res.unit_specifier == 1 {
                let dpi_x = jpeg_meta_rs::utils::ppu_to_dpi(res.ppu_x);
                let dpi_y = jpeg_meta_rs::utils::ppu_to_dpi(res.ppu_y);
                add_prop("Calculated Resolution", Some(format!("{dpi_x}x{dpi_y} DPI")));
            }
        }

        if let Some(ref sbit) = info.significant_bits {
            add_prop("Significant Bits (sBIT)", Some(format!("{:?}", sbit.bits)));
        }

        if let Some(ref bkgd) = info.background_color {
            let desc = if let Some(p) = bkgd.palette_index {
                format!("Palette Index {p}")
            } else if let Some(g) = bkgd.gray {
                format!("Grayscale {g}")
            } else if let Some(rgb) = bkgd.rgb {
                format!("RGB ({}, {}, {})", rgb.0, rgb.1, rgb.2)
            } else {
                "Unknown".to_string()
            };
            add_prop("Background Color (bKGD)", Some(desc));
        }

        if let Some(ref offs) = info.offset {
            let unit = if offs.unit_specifier == 1 { "micrometers" } else { "pixels" };
            add_prop("Image Offset (oFFs)", Some(format!("X={}, Y={} {}", offs.offset_x, offs.offset_y, unit)));
        }

        if let Some(ref scal) = info.physical_scale {
            let unit = if scal.unit_specifier == 1 { "meters" } else { "radians" };
            add_prop("Physical Scale (sCAL)", Some(format!("X={} {}, Y={} {}", scal.scale_x, unit, scal.scale_y, unit)));
        }

        if has_properties {
            println!("ℹ️ Image properties:");
            println!("{header_table}");
            println!();
        }
    }

    // 3. PNG Text chunks Table
    if !args.exclude_text && !info.text_metadata.is_empty() {
        let mut text_table = Table::new();
        text_table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Text Keyword").fg(Color::Cyan).add_attribute(Attribute::Bold),
                Cell::new("Text Value").fg(Color::Cyan).add_attribute(Attribute::Bold),
            ]);

        let mut has_text = false;
        for (k, v) in &info.text_metadata {
            if matches_filter(k, filter) {
                text_table.add_row(vec![k, v]);
                has_text = true;
            }
        }

        if has_text {
            println!("🏷️ Embedded textual records:");
            println!("{text_table}");
            println!();
        }
    }

    // 4. EXIF Tags Table (if present in eXIf chunk)
    if !args.exclude_exif {
        print_exif_table(&info.metadata, filter);
    }

    // 5. XMP Metadata Block
    if !args.exclude_xmp {
        if let Some(ref xmp) = info.metadata.xmp {
            if matches_filter("xmp", filter) {
                println!("📜 Embedded XMP Metadata Block:");
                println!("{xmp}");
                println!();
            }
        }
    }
}

fn print_exif_table(meta: &ExifMetadata, filter: &Option<Vec<String>>) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("EXIF Field").fg(Color::Magenta).add_attribute(Attribute::Bold),
            Cell::new("Value").fg(Color::Magenta).add_attribute(Attribute::Bold),
        ]);

    let mut has_exif = false;
    let mut add = |name: &str, value: Option<String>| {
        if let Some(v) = value {
            if matches_filter(name, filter) {
                table.add_row(vec![Cell::new(name), Cell::new(v)]);
                has_exif = true;
            }
        }
    };

    add("Camera Make", meta.camera_make.clone());
    add("Camera Model", meta.camera_model.clone());

    if let Some(ref gps) = meta.gps {
        add("GPS Latitude", gps.latitude.map(|v| format!("{v:.6}")));
        add("GPS Longitude", gps.longitude.map(|v| format!("{v:.6}")));
        add("GPS Altitude (m)", gps.altitude.map(|v| format!("{v}")));
        add("GPS Speed", gps.speed.map(|v| {
            let ref_str = gps.speed_ref.as_deref().unwrap_or("K");
            format!("{v:.2} {ref_str}")
        }));
        add("GPS Track (Bearing)", gps.track.map(|v| {
            let ref_str = gps.track_ref.as_deref().unwrap_or("T");
            format!("{v:.2} deg {ref_str}")
        }));
        add("GPS Image Direction", gps.img_direction.map(|v| {
            let ref_str = gps.img_direction_ref.as_deref().unwrap_or("T");
            format!("{v:.2} deg {ref_str}")
        }));
        add("GPS Date Stamp", gps.date_stamp.clone());
        add("GPS Time Stamp (UTC)", gps.time_stamp.clone());
    }

    add("F-Number", meta.f_number.map(|v| format!("f/{v}")));
    add("ISO", meta.iso.map(|v| v.to_string()));
    add("Exposure Time", meta.exposure_time.clone());
    add("Focal Length (mm)", meta.focal_length_mm.map(|v| format!("{v} mm")));
    add("Focal Length (35mm)", meta.focal_length_35mm.map(|v| format!("{v} mm")));
    add("Lens Make", meta.lens_make.clone());
    add("Lens Model", meta.lens_model.clone());
    add("Original Timestamp", meta.date_time_original.clone());
    add("Orientation Offset", meta.orientation.map(|v| v.to_string()));
    add("Exif Image Width", meta.width.map(|v| v.to_string()));
    add("Exif Image Height", meta.height.map(|v| v.to_string()));
    add("Software", meta.software.clone());

    // Advanced & Forensic tags
    add("Body Serial Number", meta.body_serial_number.clone());
    add("Lens Serial Number", meta.lens_serial_number.clone());
    add("Flash State", meta.flash.map(|v| format!("0x{v:04X} ({v})")));
    add("Exposure Program", meta.exposure_program.map(|v| match v {
        1 => "Manual (1)".to_string(),
        2 => "Normal Program (2)".to_string(),
        3 => "Aperture Priority (3)".to_string(),
        4 => "Shutter Priority (4)".to_string(),
        other => format!("Other ({other})"),
    }));
    add("Metering Mode", meta.metering_mode.map(|v| match v {
        1 => "Average (1)".to_string(),
        2 => "Center-Weighted (2)".to_string(),
        3 => "Spot (3)".to_string(),
        4 => "Multi-Spot (4)".to_string(),
        5 => "Pattern / Multi-Segment (5)".to_string(),
        other => format!("Other ({other})"),
    }));
    add("White Balance", meta.white_balance.map(|v| match v {
        0 => "Auto (0)".to_string(),
        1 => "Manual (1)".to_string(),
        other => format!("Other ({other})"),
    }));
    add("Light Source", meta.light_source.map(|v| match v {
        0 => "Unknown (0)".to_string(),
        1 => "Daylight (1)".to_string(),
        2 => "Fluorescent (2)".to_string(),
        3 => "Tungsten / Incandescent (3)".to_string(),
        4 => "Flash (4)".to_string(),
        other => format!("Other ({other})"),
    }));
    add("User Comment", meta.user_comment.clone());

    if has_exif {
        println!("📸 Decoded EXIF Metadata parameters:");
        println!("{table}");
        println!();
    }
}
