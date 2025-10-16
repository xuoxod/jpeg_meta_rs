use clap::{Parser, ValueEnum};
use jpeg_meta_rs::JpegMetadata;
use serde_json;
use std::path::PathBuf;
mod utils;
use std::fs;
use utils::{
    print::metadata_to_table,
    save::{save_bytes, unique_output_path},
    scan::list_jpeg_segments,
};

// Use the library crate we are building
use jpeg_meta_rs::parse_metadata; // Library function to parse EXIF metadata

#[derive(Copy, Clone, Debug, ValueEnum)]
enum OutputFormat {
    Debug,
    Json,
    Table,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum SaveFormat {
    Json,
    Txt,
}

/// Simple program to parse JPEG metadata
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the JPEG file to parse
    #[arg(required = true)]
    file_path: PathBuf,

    /// Output metadata as JSON (deprecated; use --format json)
    #[arg(long, hide = true)]
    json: bool,

    /// Output format: json|table|debug (overridden by --json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    format: OutputFormat,

    /// Save output to file in this directory
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Save format when saving to file
    #[arg(long, value_enum, default_value_t = SaveFormat::Json)]
    save_format: SaveFormat,

    /// List JPEG segments (APP markers, SOS, EOI)
    #[arg(long)]
    segments: bool,
}

fn main() -> Result<(), String> {
    // Return a Result for easier error handling
    let args = Args::parse();
    let path = args.file_path.as_path();

    // --- Validation Steps ---

    // 1. Check Existence and if it's a file
    if !path.exists() {
        return Err(format!("Error: File not found at '{}'", path.display()));
    }
    if !path.is_file() {
        return Err(format!("Error: Path '{}' is not a file.", path.display()));
    }

    // 2. Check Readability (Basic check - reading metadata might fail later anyway)
    // std::fs::metadata gives us file metadata, including permissions, but checking
    // exact read permission across OSes can be tricky. Often, just trying to read
    // is the most reliable check. We'll rely on the file reading later to fail if needed.
    println!("File exists and is a file: {}", path.display());

    // 3. Check File Type (Simple extension check for now)
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase());

    match extension.as_deref() {
        Some("jpg") | Some("jpeg") => {
            println!("File extension suggests JPEG.");
        }
        _ => {
            // For now, we'll just warn, but could make this an error
            // return Err(format!("Error: File '{}' does not have a .jpg or .jpeg extension.", file_path_str));
            println!("Warning: File extension is not .jpg or .jpeg. Attempting to parse anyway.");
        }
    }

    // --- Read File Content ---
    // Read the whole file into memory. For very large files, streaming might be better.
    let jpeg_data =
        fs::read(path).map_err(|e| format!("Error reading file '{}': {}", path.display(), e))?;

    // --- Call Parsing Logic (from lib.rs) ---
    println!("Attempting to parse metadata...");
    match parse_metadata(&jpeg_data) {
        Ok(metadata) => {
            let selected_format = if args.json {
                OutputFormat::Json
            } else {
                args.format
            };
            if matches!(selected_format, OutputFormat::Json) {
                let json = serde_json::to_string_pretty(&metadata)
                    .map_err(|e| format!("Error serializing JSON: {}", e))?;
                println!("{}", json);
                if let Some(dir) = args.output_dir.as_deref() {
                    let out = unique_output_path(dir, path, "exif", "json");
                    save_bytes(&out, json.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
                    println!("Saved JSON to {}", out.display());
                }
            } else {
                match selected_format {
                    OutputFormat::Table => {
                        let table = metadata_to_table(&metadata);
                        println!("{}", table);
                        if let Some(dir) = args.output_dir.as_deref() {
                            let out = unique_output_path(dir, path, "exif", "txt");
                            save_bytes(&out, format!("{}\n", table).as_bytes())
                                .map_err(|e| format!("Save failed: {}", e))?;
                            println!("Saved table to {}", out.display());
                        }
                    }
                    OutputFormat::Debug => {
                        println!("Successfully parsed metadata:");
                        println!("{:#?}", metadata);
                        if let Some(dir) = args.output_dir.as_deref() {
                            let out = unique_output_path(dir, path, "exif", "txt");
                            save_bytes(&out, format!("{:#?}\n", metadata).as_bytes())
                                .map_err(|e| format!("Save failed: {}", e))?;
                            println!("Saved debug text to {}", out.display());
                        }
                    }
                    OutputFormat::Json => unreachable!(),
                }
            }

            // Optional: list JPEG segments for insight
            if args.segments {
                let segments = list_jpeg_segments(&jpeg_data);
                if !segments.is_empty() {
                    println!("\nSegments:");
                    for s in segments {
                        println!("- {}", s);
                    }
                }
            }
            Ok(()) // Indicate success
        }
        Err(e) => {
            // Use the Debug representation of the ParseError enum
            Err(format!("Error parsing metadata: {:?}", e))
        }
    }
}
