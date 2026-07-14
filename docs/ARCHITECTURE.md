# 🧬 Crate Architecture & Design Manual

This document details the modular layout, parsing flowcharts, and technical data structures of `jpeg_meta_rs`.

---

## 🎨 1. Modular Core (Separation of Concerns & Workspaces)

The codebase is structured as a **Cargo Workspace** containing two distinct crates:
1.  `jpeg_meta_rs` (Root Crate): Contains the binary CLI application and library parsers for JPEG and PNG formats.
2.  `jpeg_meta_utils` (Sub-Crate): A dedicated library crate containing OJP validation engines, path/permission checkers, signature validation, custom errors, and math helpers.

```mermaid
graph TD
    subgraph Client [CLI Orchestrator]
        Main[src/main.rs]
    end

    subgraph Library [jpeg_meta_rs lib]
        Main --> JpegEngine[src/jpeg.rs]
        Main --> PngEngine[src/png.rs]
        
        JpegEngine --> Common[src/common.rs]
        PngEngine --> Common
        
        JpegEngine --> ExifCrate[kamadak-exif]
        PngEngine --> ExifCrate
    end

    subgraph SubCrate [jpeg_meta_utils sub-crate]
        Main --> PathVal[utils/src/path.rs]
        Main --> FTVal[utils/src/file_type.rs]
        Main --> ArgVal[utils/src/validation.rs]
        Main --> ErrVal[utils/src/error.rs]
        
        PngEngine --> CrcVal[utils/src/crc.rs]
        Common --> GeoVal[utils/src/geo.rs]
    end

    classDef bin fill:#1e293b,stroke:#3b82f6,stroke-width:2px,color:#f8fafc;
    classDef lib fill:#0f172a,stroke:#10b981,stroke-width:2px,color:#f8fafc;
    classDef util fill:#312e81,stroke:#6366f1,stroke-width:2px,color:#f8fafc;
    class Main bin;
    class JpegEngine,PngEngine,Common,ExifCrate lib;
    class PathVal,FTVal,ArgVal,ErrVal,CrcVal,GeoVal util;
```

---

## 🗺️ 2. Multi-File Dictionary Structures

When multiple files are analyzed, `main.rs` builds a `BTreeMap<String, FileAnalysis>` mapping file paths to their polymorphic metadata records:

```rust
#[derive(Serialize)]
#[serde(untagged)]
pub enum FileAnalysis {
    Jpeg(JpegInfo),
    Png(PngInfo),
}
```

This ensures that the final structured JSON output is returned as a single unified map, making it extremely easy to pipeline with downstream search utilities or databases.

---

## 📸 3. JPEG Segment Scanning Flow

The JPEG engine (`src/jpeg.rs`) parses the JPEG binary layout sequentially. It loops through segments using marker offsets:

```mermaid
flowchart TD
    Start([🚀 Start JPEG Scan]) --> CheckSOI{SOI Header 0xFFD8?}
    CheckSOI -- No --> Err[InvalidFormat Error]
    CheckSOI -- Yes --> FindMarker{Read 0xFFxx Marker}
    
    FindMarker --> EOI{0xFFD9 EOI?}
    EOI -- Yes --> SaveSegment[Save EOI Segment] --> End([🏁 End Scan])
    
    EOI -- No --> SOS{0xFFDA SOS?}
    SOS -- Yes --> SaveSOS[Save SOS Segment] --> End
    
    SOS -- No --> ReadLen[Read 2-Byte Segment Length]
    ReadLen --> SaveSeg[Save Segment Metadata]
    
    SaveSeg --> IsAPP1{Is APP1 EXIF?}
    IsAPP1 -- Yes --> ExtractExif[Extract Raw EXIF Bytes]
    IsAPP1 -- No --> IsXMP{Is APP1 XMP?}
    
    IsXMP -- Yes --> ExtractXMP[Extract Raw XML String]
    IsXMP -- No --> IsSOF{Is SOF0/SOF2?}
    
    IsSOF -- Yes --> ParseDim[Parse Width, Height & Precision]
    IsSOF -- No --> Skip[Skip Segment Payload]
    
    ExtractExif --> Skip
    ExtractXMP --> Skip
    ParseDim --> Skip
    Skip --> FindMarker
```

---

## 🖼️ 4. PNG Chunk Scanning Flow

The PNG engine (`src/png.rs`) processes chunks according to the W3C PNG specification. Chunks consist of a 4-byte length, 4-byte ASCII type, payload, and a 4-byte CRC checksum:

```mermaid
flowchart TD
    Start([🚀 Start PNG Scan]) --> CheckSig{PNG Signature?}
    CheckSig -- No --> Err[InvalidFormat Error]
    CheckSig -- Yes --> ReadChunk{Read Length & Type}
    
    ReadChunk --> IsIEND{IEND Chunk?}
    IsIEND -- Yes --> End([🏁 End Scan])
    
    IsIEND -- No --> VerifyCRC[Compute & Verify CRC32]
    VerifyCRC --> Route{Match Chunk Type}
    
    Route -- IHDR --> ParseIHDR[Parse Dimensions & Color Type]
    Route -- tEXt/iTXt --> ParseText[Parse Key-Value Metadata & XMP]
    Route -- tIME --> ParseTime[Parse Modification Time]
    Route -- pHYs --> ParsePhys[Parse Pixel Aspect Resolution]
    Route -- eXIf --> ExtractExif[Extract Raw EXIF Bytes]
    Route -- sBIT/bKGD/oFFs/sCAL --> ParseAncillary[Parse Chunk Specific Properties]
    Route -- Other --> Skip[Skip Payload]
    
    ParseIHDR --> Next[Read Next Chunk]
    ParseText --> Next
    ParseTime --> Next
    ParsePhys --> Next
    ExtractExif --> Next
    ParseAncillary --> Next
    Skip --> Next
    Next --> ReadChunk
```

---

## 🧭 5. WebP Chunk Scanning Flow

The WebP parser (`src/webp.rs`) reads the RIFF container format. Sub-chunks are read sequentially:
1.  **Header Check**: Validates `RIFF` and `WEBP` magic tags.
2.  **Extended Headers (`VP8X`)**: Parses flags and gets canvas width/height (24-bit).
3.  **Bitstream Chunks (`VP8 ` / `VP8L`)**: Parses frame headers (lossy) or bit streams (lossless) to resolve dimensions.
4.  **Metadata Extraction**: Gathers `EXIF` (passed to decode engine) and `XMP ` chunks.

---

## 🎨 6. GIF Block Scanning Flow

The GIF parser (`src/gif.rs`) scans blocks sequentially:
1.  **Logical Screen Descriptor**: Resolves canvas width and height.
2.  **Global Color Table**: Skips if present.
3.  **ExtensionBlocks (`0x21`)**:
    *   `Comment Extension` (`0xFE`): Concatenates sub-blocks as comments.
    *   `Application Extension` (`0xFF`): Looks for `XMP Data` App Identifier and decodes.
4.  **ImageDescriptor (`0x2C`)**: Skips local table and LZW frame sub-blocks.

---

## 🏗️ 7. HEIC Box Scanning Flow

The HEIC parser (`src/heic.rs`) scans the ISOBMFF structure:
1.  **Box Layout**: Loops through 4-byte box types.
2.  **Image Spatial Extents (`ispe`)**: Resolves dimensions.
3.  **Item Location & Info (`iloc`, `iinf`)**: Matches the `Exif` item identifier, fetches its offset coordinates and lengths, and decodes the TIFF payload.

---

## 🔍 8. Embedded Payload & Overlay Scanning Flow

To inspect files for hidden, appended, or malicious payloads (steganography / overlay), the `scan_embedded_payloads` utility inside [utils/src/embedded.rs](file:///home/emhcet/private/projects/desktop/rust/jpeg_meta_rs/utils/src/embedded.rs) executes a post-parsing pipeline:

```mermaid
graph TD
    Start([🚀 Start Scanner]) --> LoadBytes[Load File Bytes]
    LoadBytes --> GetEOF[Fetch Parser's Scanned EOF Offset]
    
    subgraph OverlayCheck [Overlay Detection]
        GetEOF --> CompareLen{File Size > Scanned EOF?}
        CompareLen -- Yes --> CreateOverlay[Report Trailing Data Payload]
        CompareLen -- No --> SigScan[Signature Walk]
    end
    
    subgraph PatternWalk [Signature Magic Matching]
        CreateOverlay --> SigScan
        SigScan --> InitOffset[Start Search at Offset 12]
        InitOffset --> WindowMatch{Match Magic Signature?}
        WindowMatch -- ZIP/ELF/PE/PDF/PHP/Script --> CreatePayload[Report Hidden Payload]
        WindowMatch -- None --> NextByte[Increment Scan Offset]
        CreatePayload --> NextByte
        NextByte --> Done{EndOfFile?}
        Done -- No --> WindowMatch
        Done -- Yes --> End([🏁 Done])
    end
```

---

## 💾 9. Cross-Platform File Copy & De-embedding (Scrubbing) Flow

The copier engine inside [utils/src/copy.rs](file:///home/emhcet/private/projects/desktop/rust/jpeg_meta_rs/utils/src/copy.rs) provides two OJP operations:
1.  **Platform-Aware Pictures Directory Resolution**: Resolves the default pictures folder for Windows (`%USERPROFILE%/Pictures`), macOS (`$HOME/Pictures`), and Linux (`$HOME/Pictures`).
2.  **File Copying & Directory Creation**: Safely copies images, creating nested parent folders on demand.
3.  **Automatic Name Collision Resolution**: Checks if the target file already exists (or is being copied to the same folder as the original). If so, it automatically generates a non-colliding filename by appending `_copy_1`, `_copy_2`, etc. sequentially.
4.  **Sanitization/De-embedding (Scrubbing)**: Creates a sanitized copy of an image by truncating all trailing data starting at the `official_end_offset`. This scrubs any binder executables, appended ZIPs, or overlay payloads, preserving the original logical image container exactly:

```mermaid
graph TD
    Start([🚀 Start Scrubbing]) --> ReadFile[Read Source Bytes]
    ReadFile --> GetOffset[Fetch official_end_offset]
    GetOffset --> Compare{official_end_offset < Total Size?}
    
    Compare -- Yes --> Slice[Slice Bytes 0..official_end_offset]
    Compare -- No --> KeepAll[Keep Original Bytes]
    
    Slice --> WriteFile[Write to Destination]
    KeepAll --> WriteFile
    WriteFile --> End([🏁 Sanitized Copy Created])
```



