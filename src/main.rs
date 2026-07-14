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
    /// Path to the JPEG or PNG file to analyze
    #[arg(value_hint = ValueHint::FilePath)]
    file_path: PathBuf,

    /// Output format
    #[arg(long, short, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,

    /// Force parsing file as specific type instead of auto-detecting signature
    #[arg(long, short, value_enum, default_value_t = FileType::Auto)]
    file_type: FileType,

    /// Only print the structural segments/chunks map and exit
    #[arg(long)]
    structure_only: bool,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let path = args.file_path.as_path();

    if !path.exists() {
        return Err(format!("Error: File not found at '{}'", path.display()));
    }
    if !path.is_file() {
        return Err(format!("Error: Path '{}' is not a file.", path.display()));
    }

    let bytes = fs::read(path).map_err(|e| format!("Error reading file: {e}"))?;
    if bytes.is_empty() {
        return Err("Error: File is empty".to_string());
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
                // Try extension check fallback
                let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
                match ext.as_deref() {
                    Some("png") => FileType::Png,
                    Some("jpg") | Some("jpeg") => FileType::Jpeg,
                    _ => return Err("Error: Could not auto-detect file signature. Use --file-type to specify explicitly.".to_string()),
                }
            }
        }
    };

    match resolved_type {
        FileType::Jpeg => {
            let info = parse_jpeg(&bytes).map_err(|e| format!("JPEG parsing error: {e}"))?;
            match args.format {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&info)
                        .map_err(|e| format!("JSON serialization error: {e}"))?;
                    println!("{json}");
                }
                OutputFormat::Table => {
                    print_jpeg_tables(path, &info, args.structure_only);
                }
            }
        }
        FileType::Png => {
            let info = parse_png(&bytes).map_err(|e| format!("PNG parsing error: {e}"))?;
            match args.format {
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&info)
                        .map_err(|e| format!("JSON serialization error: {e}"))?;
                    println!("{json}");
                }
                OutputFormat::Table => {
                    print_png_tables(path, &info, args.structure_only);
                }
            }
        }
        FileType::Auto => unreachable!(),
    }

    Ok(())
}

fn print_jpeg_tables(path: &Path, info: &JpegInfo, structure_only: bool) {
    println!("File: {}", path.display());
    println!("Type: JPEG Image Container");
    println!();

    // 1. Structure Segment Table
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

    if structure_only {
        return;
    }

    // 2. JPEG Header Properties Table
    let mut header_table = Table::new();
    header_table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Property").fg(Color::Blue).add_attribute(Attribute::Bold),
            Cell::new("Value").fg(Color::Blue).add_attribute(Attribute::Bold),
        ]);

    let mut has_properties = false;
    if let Some(w) = info.width {
        header_table.add_row(vec!["Width", &format!("{w} px")]);
        has_properties = true;
    }
    if let Some(h) = info.height {
        header_table.add_row(vec!["Height", &format!("{h} px")]);
        has_properties = true;
    }
    if let Some(p) = info.precision {
        header_table.add_row(vec!["Data Precision", &format!("{p} bits")]);
        has_properties = true;
    }
    if let Some(c) = info.channels {
        let name = match c {
            1 => "Grayscale (1)".to_string(),
            3 => "RGB / YCbCr (3)".to_string(),
            4 => "CMYK (4)".to_string(),
            other => format!("Custom ({other})"),
        };
        header_table.add_row(vec!["Color Channels", &name]);
        has_properties = true;
    }
    if let Some(ref comment) = info.comment {
        header_table.add_row(vec!["Comment Payload", comment]);
        has_properties = true;
    }

    if has_properties {
        println!("ℹ️ Image properties:");
        println!("{header_table}");
        println!();
    }

    // 3. EXIF Tags Table
    print_exif_table(&info.metadata);

    // 4. XMP Metadata Block
    if let Some(ref xmp) = info.metadata.xmp {
        println!("📜 Embedded XMP Metadata Block:");
        println!("{xmp}");
        println!();
    }
}

fn print_png_tables(path: &Path, info: &PngInfo, structure_only: bool) {
    println!("File: {}", path.display());
    println!("Type: Portable Network Graphics (PNG)");
    println!();

    // 1. Structure Chunks Table
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

    if structure_only {
        return;
    }

    // 2. PNG Header Properties Table
    let mut header_table = Table::new();
    header_table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Property").fg(Color::Blue).add_attribute(Attribute::Bold),
            Cell::new("Value").fg(Color::Blue).add_attribute(Attribute::Bold),
        ]);

    let mut has_properties = false;
    if let Some(ref ihdr) = info.header {
        header_table.add_row(vec!["Width", &format!("{} px", ihdr.width)]);
        header_table.add_row(vec!["Height", &format!("{} px", ihdr.height)]);
        header_table.add_row(vec!["Bit Depth", &format!("{} bits/channel", ihdr.bit_depth)]);
        let color_name = match ihdr.color_type {
            0 => "Grayscale (0)".to_string(),
            2 => "Truecolor RGB (2)".to_string(),
            3 => "Indexed Color (3)".to_string(),
            4 => "Grayscale with Alpha (4)".to_string(),
            6 => "Truecolor RGBA (6)".to_string(),
            other => format!("Unknown ({other})"),
        };
        header_table.add_row(vec!["Color Type", &color_name]);
        header_table.add_row(vec!["Interlace Method", if ihdr.interlace_method == 1 { "Adam7 Interlace" } else { "No Interlace" }]);
        has_properties = true;
    }
    if let Some(ref t) = info.modification_time {
        header_table.add_row(vec!["Last Mod Time (tIME)", t]);
        has_properties = true;
    }
    if let Some(ref res) = info.resolution {
        let unit_name = if res.unit_specifier == 1 { "Meters" } else { "Unknown Unit" };
        header_table.add_row(vec!["Pixels Per Unit X", &format!("{} / {}", res.ppu_x, unit_name)]);
        header_table.add_row(vec!["Pixels Per Unit Y", &format!("{} / {}", res.ppu_y, unit_name)]);
        if res.unit_specifier == 1 {
            let dpi_x = (res.ppu_x as f64 * 0.0254).round();
            let dpi_y = (res.ppu_y as f64 * 0.0254).round();
            header_table.add_row(vec!["Calculated Resolution", &format!("{dpi_x}x{dpi_y} DPI")]);
        }
        has_properties = true;
    }
    if let Some(ref sbit) = info.significant_bits {
        header_table.add_row(vec!["Significant Bits (sBIT)", &format!("{:?}", sbit.bits)]);
        has_properties = true;
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
        header_table.add_row(vec!["Background Color (bKGD)", &desc]);
        has_properties = true;
    }
    if let Some(ref offs) = info.offset {
        let unit = if offs.unit_specifier == 1 { "micrometers" } else { "pixels" };
        header_table.add_row(vec!["Image Offset (oFFs)", &format!("X={}, Y={} {}", offs.offset_x, offs.offset_y, unit)]);
        has_properties = true;
    }
    if let Some(ref scal) = info.physical_scale {
        let unit = if scal.unit_specifier == 1 { "meters" } else { "radians" };
        header_table.add_row(vec!["Physical Scale (sCAL)", &format!("X={} {}, Y={} {}", scal.scale_x, unit, scal.scale_y, unit)]);
        has_properties = true;
    }

    if has_properties {
        println!("ℹ️ Image properties:");
        println!("{header_table}");
        println!();
    }

    // 3. PNG Text chunks Table
    if !info.text_metadata.is_empty() {
        let mut text_table = Table::new();
        text_table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Text Keyword").fg(Color::Cyan).add_attribute(Attribute::Bold),
                Cell::new("Text Value").fg(Color::Cyan).add_attribute(Attribute::Bold),
            ]);
        for (k, v) in &info.text_metadata {
            text_table.add_row(vec![k, v]);
        }
        println!("🏷️ Embedded textual records:");
        println!("{text_table}");
        println!();
    }

    // 4. EXIF Tags Table (if present in eXIf chunk)
    print_exif_table(&info.metadata);

    // 5. XMP Metadata Block
    if let Some(ref xmp) = info.metadata.xmp {
        println!("📜 Embedded XMP Metadata Block:");
        println!("{xmp}");
        println!();
    }
}

fn print_exif_table(meta: &ExifMetadata) {
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
            table.add_row(vec![Cell::new(name), Cell::new(v)]);
            has_exif = true;
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
