# 🧬 Crate Architecture & Design Manual

This document details the modular layout, parsing flowcharts, and technical data structures of `jpeg_meta_rs`.

---

## 🎨 1. Modular Core (Separation of Concerns)

`jpeg_meta_rs` splits the extraction pipeline into two **completely separate, distinct, and decoupled codebases** for JPEG and PNG formats. The binary CLI (`main.rs`) serves as the orchestrator to route inputs and print/serialize outputs.

```mermaid
graph TD
    subgraph Client [CLI Orchestrator]
        Main[src/main.rs]
    end

    subgraph Library [Modular Crate lib]
        Main --> JpegEngine[src/jpeg.rs]
        Main --> PngEngine[src/png.rs]
        
        JpegEngine --> Common[src/common.rs]
        PngEngine --> Common
        
        JpegEngine --> ExifCrate[kamadak-exif]
        PngEngine --> ExifCrate
    end

    classDef bin fill:#1e293b,stroke:#3b82f6,stroke-width:2px,color:#f8fafc;
    classDef lib fill:#0f172a,stroke:#10b981,stroke-width:2px,color:#f8fafc;
    class Main bin;
    class JpegEngine,PngEngine,Common,ExifCrate lib;
```

---

## 📸 2. JPEG Segment Scanning Flow

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
    IsAPP1 -- No --> IsSOF{Is SOF0/SOF2?}
    
    IsSOF -- Yes --> ParseDim[Parse Width, Height & Precision]
    IsSOF -- No --> Skip[Skip Segment Payload]
    
    ExtractExif --> Skip
    ParseDim --> Skip
    Skip --> FindMarker
```

---

## 🖼️ 3. PNG Chunk Scanning Flow

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
    Route -- tEXt/iTXt --> ParseText[Parse Key-Value Metadata]
    Route -- tIME --> ParseTime[Parse Modification Time]
    Route -- pHYs --> ParsePhys[Parse Pixel Aspect Resolution]
    Route -- eXIf --> ExtractExif[Extract Raw EXIF Bytes]
    Route -- Other --> Skip[Skip Payload]
    
    ParseIHDR --> Next[Read Next Chunk]
    ParseText --> Next
    ParseTime --> Next
    ParsePhys --> Next
    ExtractExif --> Next
    Skip --> Next
    Next --> ReadChunk
```
