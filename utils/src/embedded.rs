#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct EmbeddedPayload {
    pub name: String,
    pub offset: usize,
    pub length: usize,
    pub category: String,
    pub preview: String,
}

struct SignaturePattern {
    magic: &'static [u8],
    name: &'static str,
    category: &'static str,
}

const PATTERNS: &[SignaturePattern] = &[
    SignaturePattern { magic: &[0x50, 0x4B, 0x03, 0x04], name: "ZIP Archive", category: "Archive" },
    SignaturePattern { magic: &[0x4D, 0x5A], name: "Windows Portable Executable (PE)", category: "Binary" },
    SignaturePattern { magic: &[0x7F, 0x45, 0x4C, 0x46], name: "Linux ELF Binary", category: "Binary" },
    SignaturePattern { magic: &[0x25, 0x50, 0x44, 0x46], name: "PDF Document", category: "Document" },
    SignaturePattern { magic: &[0x3C, 0x3F, 0x70, 0x68, 0x70], name: "PHP Script Block", category: "Script" }, // "<?php"
    SignaturePattern { magic: &[0x3C, 0x73, 0x63, 0x72, 0x69, 0x70, 0x74], name: "JavaScript/HTML Script Block", category: "Script" }, // "<script"
    SignaturePattern { magic: &[0x65, 0x76, 0x61, 0x6C, 0x28], name: "Suspicious Eval Function Call", category: "Script Injection" }, // "eval("
    SignaturePattern { magic: &[0x73, 0x79, 0x73, 0x74, 0x65, 0x6D, 0x28], name: "Suspicious System Execution Call", category: "Script Injection" }, // "system("
    SignaturePattern { magic: &[0x62, 0x61, 0x73, 0x65, 0x36, 0x34, 0x5F, 0x64, 0x65, 0x63, 0x6F, 0x64, 0x65, 0x28], name: "Suspicious Base64 Payload Decoder", category: "Script Injection" }, // "base64_decode("
    SignaturePattern { magic: &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A], name: "Embedded PNG Image", category: "Image" },
    SignaturePattern { magic: &[0xFF, 0xD8, 0xFF], name: "Embedded JPEG Image", category: "Image" },
];

fn is_valid_payload(bytes: &[u8], offset: usize, name: &str) -> bool {
    match name {
        "Windows Portable Executable (PE)" => {
            // MZ signature must have PE signature at offset specified by e_lfanew at 0x3C
            if offset + 0x40 <= bytes.len() {
                let pe_ptr = u32::from_le_bytes([
                    bytes[offset + 0x3C],
                    bytes[offset + 0x3D],
                    bytes[offset + 0x3E],
                    bytes[offset + 0x3F],
                ]) as usize;
                if offset + pe_ptr + 4 <= bytes.len() {
                    return &bytes[offset + pe_ptr..offset + pe_ptr + 4] == b"PE\0\0";
                }
            }
            false
        }
        _ => true,
    }
}

/// Scans the binary payload of an image for trailing data (overlay) or embedded files with known signatures.
pub fn scan_embedded_payloads(bytes: &[u8], official_end_offset: usize) -> Vec<EmbeddedPayload> {
    let mut payloads = Vec::new();

    // 1. Check for Trailing Data (Overlay)
    if bytes.len() > official_end_offset {
        let length = bytes.len() - official_end_offset;
        let preview_len = std::cmp::min(16, length);
        let preview_bytes = &bytes[official_end_offset..official_end_offset + preview_len];
        payloads.push(EmbeddedPayload {
            name: "Trailing Data (Overlay)".to_string(),
            offset: official_end_offset,
            length,
            category: "Overlay".to_string(),
            preview: format_preview(preview_bytes),
        });
    }

    // 2. Scan for magic signatures starting after offset 12 (to ignore the parent file's own headers)
    if bytes.len() > 12 {
        let scan_limit = bytes.len();
        for pattern in PATTERNS {
            let n = pattern.magic.len();
            let mut i = 12;
            while i + n <= scan_limit {
                if &bytes[i..i+n] == pattern.magic {
                    // Make sure it doesn't overlap with trailing data
                    if i < official_end_offset && is_valid_payload(bytes, i, pattern.name) {
                        let preview_len = std::cmp::min(16, scan_limit - i);
                        let preview_bytes = &bytes[i..i + preview_len];
                        payloads.push(EmbeddedPayload {
                            name: pattern.name.to_string(),
                            offset: i,
                            length: 0, // length is unknown/variable
                            category: pattern.category.to_string(),
                            preview: format_preview(preview_bytes),
                        });
                    }
                }
                i += 1;
            }
        }
    }

    payloads
}

fn format_preview(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }
    let hex = data.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" ");
    let ascii = data.iter()
        .map(|&b| {
            if (b.is_ascii_graphic() || b == b' ') && b != b'\r' && b != b'\n' {
                b as char
            } else {
                '.'
            }
        })
        .collect::<String>();
    format!("{} | {}", hex, ascii)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_trailing_data() {
        let bytes = b"RIFF\0\0\0\x0cWEBPVP8X\0\0\0\0\0\0EXTRA_TRAILING_BYTES".to_vec();
        let payloads = scan_embedded_payloads(&bytes, 20);
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].name, "Trailing Data (Overlay)");
        assert_eq!(payloads[0].offset, 20);
        assert_eq!(payloads[0].length, 22);
    }

    #[test]
    fn test_scan_embedded_signature() {
        let mut bytes = b"GIF89a\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec();
        bytes.extend_from_slice(b"some prefix PK\x03\x04zip_content");
        
        let payloads = scan_embedded_payloads(&bytes, bytes.len());
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].name, "ZIP Archive");
        assert_eq!(payloads[0].category, "Archive");
    }

    #[test]
    fn test_scan_embedded_pe_signature() {
        let mut bytes = b"GIF89a\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec();
        // Construct DOS header with e_lfanew pointer pointing to a valid PE signature
        let mut dos_header = vec![0u8; 0x40];
        dos_header[0..2].copy_from_slice(b"MZ");
        dos_header[0x3C..0x40].copy_from_slice(&0x40u32.to_le_bytes()); // e_lfanew points to offset 0x40
        dos_header.extend_from_slice(b"PE\0\0remaining_pe_header");

        bytes.extend_from_slice(&dos_header);
        
        let payloads = scan_embedded_payloads(&bytes, bytes.len());
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].name, "Windows Portable Executable (PE)");
    }

    #[test]
    fn test_scan_embedded_script_injection() {
        let mut bytes = b"GIF89a\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec();
        bytes.extend_from_slice(b"some prefix eval(base64_decode('payload'))");

        let payloads = scan_embedded_payloads(&bytes, bytes.len());
        // Should detect both eval( and base64_decode(
        assert_eq!(payloads.len(), 2);
        assert!(payloads.iter().any(|p| p.name == "Suspicious Eval Function Call"));
        assert!(payloads.iter().any(|p| p.name == "Suspicious Base64 Payload Decoder"));
    }
}
