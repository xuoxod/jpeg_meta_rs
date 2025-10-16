use clap::{Parser, ValueEnum, ValueHint};
use jpeg_meta_rs::JpegMetadata;
use serde_json;
use std::path::PathBuf;
mod utils;
use std::fs;
use utils::{
    catalog::{
        entries_to_csv, entries_to_jsonl, entries_to_markdown, entries_to_paths, entries_to_tsv,
        index_directory,
    },
    print::{
        FieldSet, available_fields, filter_fields, metadata_to_table, rows_to_csv,
        rows_to_markdown, rows_to_tsv,
    },
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
    Csv,
    Md,
    Tsv,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum SaveFormat {
    Json,
    Txt,
    Csv,
    Md,
    Tsv,
}

/// Simple program to parse JPEG metadata
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Extract EXIF metadata from JPEGs with flexible output (table, JSON, CSV, Markdown, TSV).",
    long_about = "A simple yet flexible CLI and library to parse common EXIF metadata from JPEG images.\n\nFeatures:\n- Table/JSON/CSV/Markdown/TSV outputs\n- Field filtering (e.g., --fields camera_make,iso)\n- Optional JPEG segment listing\n- Save outputs with unique filenames\n\nUse --list-fields to see all available field keys.",
    after_help = "Examples:\n  # Default table output\n  jpeg_meta_rs image.jpg\n\n  # JSON output\n  jpeg_meta_rs --format json image.jpg\n\n  # CSV/Markdown/TSV with selected fields\n  jpeg_meta_rs --format csv --fields camera_make,iso image.jpg\n  jpeg_meta_rs --format md --fields all image.jpg\n  jpeg_meta_rs --format tsv --fields camera_make,camera_model,f_number image.jpg\n\n  # Save outputs (independent of display format)\n  jpeg_meta_rs --format table --save-format json --output-dir out image.jpg\n\n  # List available field keys\n  jpeg_meta_rs --list-fields\n\n  # Index a directory (flat) and print JSON catalog\n  jpeg_meta_rs --index-dir ./photos --format json\n\n  # Index recursively and export only paths\n  jpeg_meta_rs --index-dir ./photos --recursive --export-paths --output-dir out\n\n  # Index and export JSON Lines (one per line)\n  jpeg_meta_rs --index-dir ./photos --export-jsonl --output-dir out\n\n  # Index and display TSV, saving Markdown\n  jpeg_meta_rs --index-dir ./photos --format tsv --save-format md --output-dir out"
)]
struct Args {
    /// Path to the JPEG file to parse
    #[arg(value_hint = ValueHint::FilePath, required_unless_present_any = ["list_fields", "index_dir"])]
    file_path: Option<PathBuf>,

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

    /// Fields to include: "all" or comma-separated keys (e.g., camera_make,iso)
    #[arg(long, default_value = "all")]
    fields: String,

    /// List available field keys and exit
    #[arg(long, action)]
    list_fields: bool,
    /// Build a catalog by indexing a directory of JPEGs; prints and/or saves the collection
    #[arg(long, value_hint = ValueHint::DirPath, conflicts_with = "file_path")]
    index_dir: Option<PathBuf>,

    /// Recurse into subdirectories when indexing
    #[arg(long, requires = "index_dir")]
    recursive: bool,

    /// When indexing, export only file paths (one per line) for external tools
    #[arg(long, requires = "index_dir")]
    export_paths: bool,

    /// When indexing, export JSON Lines (one CatalogEntry per line)
    #[arg(long, requires = "index_dir")]
    export_jsonl: bool,
}

fn main() -> Result<(), String> {
    // Return a Result for easier error handling
    let args = Args::parse();
    // If user asked to list available fields, do that and exit early
    if args.list_fields {
        let mut lines = Vec::new();
        for (key, label) in available_fields() {
            lines.push(format!("{key:22}  {label}"));
        }
        println!(
            "Available fields (use with --fields):\n{}",
            lines.join("\n")
        );
        return Ok(());
    }

    // If indexing a directory, produce a catalog output using chosen format and save_format
    if let Some(dir) = args.index_dir.as_ref() {
        let entries = index_directory(dir, args.recursive)?;
        if args.export_paths {
            let s = entries_to_paths(&entries);
            println!("{}", s.trim_end());
            if let Some(outdir) = args.output_dir.as_deref() {
                let out = unique_output_path(outdir, dir.as_path(), "paths", "txt");
                save_bytes(&out, s.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
                println!("Saved paths to {}", out.display());
            }
            return Ok(());
        }
        if args.export_jsonl {
            let s = entries_to_jsonl(&entries).map_err(|e| format!("JSONL error: {}", e))?;
            print!("{}", s);
            if let Some(outdir) = args.output_dir.as_deref() {
                let out = unique_output_path(outdir, dir.as_path(), "catalog", "jsonl");
                save_bytes(&out, s.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
                println!("Saved catalog JSONL to {}", out.display());
            }
            return Ok(());
        }
        match args.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&entries)
                    .map_err(|e| format!("Error serializing JSON: {}", e))?;
                println!("{}", json);
                if let Some(outdir) = args.output_dir.as_deref() {
                    let out = unique_output_path(outdir, dir.as_path(), "catalog", "json");
                    save_bytes(&out, json.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
                    println!("Saved catalog JSON to {}", out.display());
                }
            }
            OutputFormat::Csv => {
                let fields = if args.fields.trim().eq_ignore_ascii_case("all") {
                    FieldSet::All
                } else {
                    let names: Vec<String> = args
                        .fields
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    FieldSet::Names(names)
                };
                let csv = entries_to_csv(&entries, &fields);
                println!("{}", csv.trim_end());
                if let Some(outdir) = args.output_dir.as_deref() {
                    let out = unique_output_path(outdir, dir.as_path(), "catalog", "csv");
                    save_bytes(&out, csv.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
                    println!("Saved catalog CSV to {}", out.display());
                }
            }
            OutputFormat::Md => {
                let fields = if args.fields.trim().eq_ignore_ascii_case("all") {
                    FieldSet::All
                } else {
                    let names: Vec<String> = args
                        .fields
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    FieldSet::Names(names)
                };
                let md = entries_to_markdown(&entries, &fields);
                println!("{}", md);
                if let Some(outdir) = args.output_dir.as_deref() {
                    let out = unique_output_path(outdir, dir.as_path(), "catalog", "md");
                    save_bytes(&out, md.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
                    println!("Saved catalog Markdown to {}", out.display());
                }
            }
            OutputFormat::Tsv => {
                let fields = if args.fields.trim().eq_ignore_ascii_case("all") {
                    FieldSet::All
                } else {
                    let names: Vec<String> = args
                        .fields
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    FieldSet::Names(names)
                };
                let tsv = entries_to_tsv(&entries, &fields);
                println!("{}", tsv.trim_end());
                if let Some(outdir) = args.output_dir.as_deref() {
                    let out = unique_output_path(outdir, dir.as_path(), "catalog", "tsv");
                    save_bytes(&out, tsv.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
                    println!("Saved catalog TSV to {}", out.display());
                }
            }
            OutputFormat::Table | OutputFormat::Debug => {
                println!("Indexed {} JPEG(s) under {}", entries.len(), dir.display());
                if let Some(outdir) = args.output_dir.as_deref() {
                    match args.save_format {
                        SaveFormat::Json => {
                            let json = serde_json::to_string_pretty(&entries)
                                .map_err(|e| format!("Error serializing JSON: {}", e))?;
                            let out = unique_output_path(outdir, dir.as_path(), "catalog", "json");
                            save_bytes(&out, json.as_bytes())
                                .map_err(|e| format!("Save failed: {}", e))?;
                            println!("Saved catalog JSON to {}", out.display());
                        }
                        SaveFormat::Csv => {
                            let csv = entries_to_csv(&entries, &FieldSet::All);
                            let out = unique_output_path(outdir, dir.as_path(), "catalog", "csv");
                            save_bytes(&out, csv.as_bytes())
                                .map_err(|e| format!("Save failed: {}", e))?;
                            println!("Saved catalog CSV to {}", out.display());
                        }
                        SaveFormat::Md => {
                            let md = entries_to_markdown(&entries, &FieldSet::All);
                            let out = unique_output_path(outdir, dir.as_path(), "catalog", "md");
                            save_bytes(&out, md.as_bytes())
                                .map_err(|e| format!("Save failed: {}", e))?;
                            println!("Saved catalog Markdown to {}", out.display());
                        }
                        SaveFormat::Txt => {
                            let s = format!(
                                "Indexed {} JPEG(s) under {}\n",
                                entries.len(),
                                dir.display()
                            );
                            let out = unique_output_path(outdir, dir.as_path(), "catalog", "txt");
                            save_bytes(&out, s.as_bytes())
                                .map_err(|e| format!("Save failed: {}", e))?;
                            println!("Saved catalog summary to {}", out.display());
                        }
                        SaveFormat::Tsv => {
                            let tsv = entries_to_tsv(&entries, &FieldSet::All);
                            let out = unique_output_path(outdir, dir.as_path(), "catalog", "tsv");
                            save_bytes(&out, tsv.as_bytes())
                                .map_err(|e| format!("Save failed: {}", e))?;
                            println!("Saved catalog TSV to {}", out.display());
                        }
                    }
                }
            }
        }
        return Ok(());
    }

    let Some(pathbuf) = args.file_path.as_ref() else {
        return Err("Missing <file_path>; provide a file or use --list-fields".to_string());
    };
    let path = pathbuf.as_path();

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
                try_save(&args, path, &metadata, Some(&json))?;
            } else {
                match selected_format {
                    OutputFormat::Table => {
                        let table = metadata_to_table(&metadata);
                        println!("{}", table);
                        try_save(&args, path, &metadata, Some(&format!("{}\n", table)))?;
                    }
                    OutputFormat::Csv => {
                        let fields = if args.fields.trim().eq_ignore_ascii_case("all") {
                            FieldSet::All
                        } else {
                            let names: Vec<String> = args
                                .fields
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            FieldSet::Names(names)
                        };
                        let rows = filter_fields(&metadata, &fields);
                        let csv = rows_to_csv(&rows);
                        println!("{}", csv.trim_end());
                        try_save(&args, path, &metadata, Some(&csv))?;
                    }
                    OutputFormat::Md => {
                        let fields = if args.fields.trim().eq_ignore_ascii_case("all") {
                            FieldSet::All
                        } else {
                            let names: Vec<String> = args
                                .fields
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            FieldSet::Names(names)
                        };
                        let rows = filter_fields(&metadata, &fields);
                        let md = rows_to_markdown(&rows);
                        println!("{}", md);
                        try_save(&args, path, &metadata, Some(&md))?;
                    }
                    OutputFormat::Tsv => {
                        let fields = if args.fields.trim().eq_ignore_ascii_case("all") {
                            FieldSet::All
                        } else {
                            let names: Vec<String> = args
                                .fields
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            FieldSet::Names(names)
                        };
                        let rows = filter_fields(&metadata, &fields);
                        let tsv = rows_to_tsv(&rows);
                        println!("{}", tsv.trim_end());
                        try_save(&args, path, &metadata, Some(&tsv))?;
                    }
                    OutputFormat::Debug => {
                        println!("Successfully parsed metadata:");
                        println!("{:#?}", metadata);
                        try_save(&args, path, &metadata, Some(&format!("{:#?}\n", metadata)))?;
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

fn try_save(
    args: &Args,
    path: &std::path::Path,
    metadata: &JpegMetadata,
    rendered: Option<&str>,
) -> Result<(), String> {
    let Some(dir) = args.output_dir.as_deref() else {
        return Ok(());
    };
    match args.save_format {
        SaveFormat::Json => {
            // Always save the full structured JSON regardless of display format
            let json = serde_json::to_string_pretty(metadata)
                .map_err(|e| format!("Error serializing JSON: {}", e))?;
            let out = unique_output_path(dir, path, "exif", "json");
            save_bytes(&out, json.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
            println!("Saved JSON to {}", out.display());
        }
        SaveFormat::Txt => {
            // Save as plain text of selected rows (if provided), else table text
            let content = if let Some(s) = rendered {
                s.to_string()
            } else {
                let table = metadata_to_table(metadata).to_string();
                format!("{}\n", table)
            };
            let out = unique_output_path(dir, path, "exif", "txt");
            save_bytes(&out, content.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
            println!("Saved text to {}", out.display());
        }
        SaveFormat::Csv => {
            // Save CSV of the selected fields
            // Use --fields to decide selection
            let fields = if args.fields.trim().eq_ignore_ascii_case("all") {
                FieldSet::All
            } else {
                let names: Vec<String> = args
                    .fields
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                FieldSet::Names(names)
            };
            let rows = filter_fields(metadata, &fields);
            let csv = rows_to_csv(&rows);
            let out = unique_output_path(dir, path, "exif", "csv");
            save_bytes(&out, csv.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
            println!("Saved CSV to {}", out.display());
        }
        SaveFormat::Md => {
            let fields = if args.fields.trim().eq_ignore_ascii_case("all") {
                FieldSet::All
            } else {
                let names: Vec<String> = args
                    .fields
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                FieldSet::Names(names)
            };
            let rows = filter_fields(metadata, &fields);
            let md = rows_to_markdown(&rows);
            let out = unique_output_path(dir, path, "exif", "md");
            save_bytes(&out, md.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
            println!("Saved Markdown to {}", out.display());
        }
        SaveFormat::Tsv => {
            let fields = if args.fields.trim().eq_ignore_ascii_case("all") {
                FieldSet::All
            } else {
                let names: Vec<String> = args
                    .fields
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                FieldSet::Names(names)
            };
            let rows = filter_fields(metadata, &fields);
            let tsv = rows_to_tsv(&rows);
            let out = unique_output_path(dir, path, "exif", "tsv");
            save_bytes(&out, tsv.as_bytes()).map_err(|e| format!("Save failed: {}", e))?;
            println!("Saved TSV to {}", out.display());
        }
    }
    Ok(())
}
