use std::path::Path;

/// Load font bytes for a given style name.
/// Returns raw font bytes that ab_glyph can parse.
pub fn load_font(style_name: &str) -> Vec<u8> {
    let style_lower = style_name.to_lowercase();

    // SHX fonts → always use Helvetica fallback
    if style_lower.ends_with(".shx") {
        return load_fallback_font();
    }

    // Try Arial Narrow for known annotation styles
    if style_lower.contains("narrow")
        || style_lower.contains("tfnsw")
        || style_lower.contains("anno")
    {
        let candidates = [
            "/System/Library/Fonts/Supplemental/Arial Narrow.ttf",
            "/Library/Fonts/Arial Narrow.ttf",
        ];
        for path in &candidates {
            if let Ok(bytes) = std::fs::read(path) {
                return bytes;
            }
        }
    }

    // Try Arial for Arial style
    if style_lower.contains("arial") {
        let candidates = [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/Library/Fonts/Arial.ttf",
            "/System/Library/Fonts/Supplemental/Arial Narrow.ttf",
        ];
        for path in &candidates {
            if let Ok(bytes) = std::fs::read(path) {
                return bytes;
            }
        }
    }

    load_fallback_font()
}

fn load_fallback_font() -> Vec<u8> {
    // Try Helvetica first (commonly available on macOS)
    let candidates: &[&str] = &[
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Geneva.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    ];

    for path in candidates {
        if Path::new(path).exists() {
            if let Ok(bytes) = std::fs::read(path) {
                return bytes;
            }
        }
    }

    // Last resort: embed a minimal font subset
    // This should not normally be reached on macOS or Linux with fonts installed
    eprintln!("Warning: No system font found, text rendering will be disabled");
    Vec::new()
}
