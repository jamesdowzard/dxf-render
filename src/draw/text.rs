use ab_glyph::{Font, FontRef, FontVec, PxScale, ScaleFont};
use tiny_skia::{Pixmap, Transform};

use crate::color::aci_to_rgb;
use crate::font::load_font;
use crate::transform::Viewport;
use crate::types::{DxfFile, MTextEntity, TextEntity};

/// Strip MTEXT escape sequences and formatting codes.
pub fn clean_mtext(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    let n = chars.len();

    while i < n {
        // Backslash escape sequences
        if chars[i] == '\\' && i + 1 < n {
            match chars[i + 1] {
                'P' => {
                    out.push('\n');
                    i += 2;
                }
                'n' => {
                    out.push('\n');
                    i += 2;
                }
                'f' | 'F' => {
                    // \f...;  — skip until semicolon
                    i += 2;
                    while i < n && chars[i] != ';' {
                        i += 1;
                    }
                    if i < n {
                        i += 1; // skip semicolon
                    }
                }
                'H' | 'h' => {
                    // \H...;  — height code
                    i += 2;
                    while i < n && chars[i] != ';' {
                        i += 1;
                    }
                    if i < n {
                        i += 1;
                    }
                }
                'C' | 'c' => {
                    // \C...;  — color code
                    i += 2;
                    while i < n && chars[i] != ';' {
                        i += 1;
                    }
                    if i < n {
                        i += 1;
                    }
                }
                'p' => {
                    // \p...;  — paragraph code
                    i += 2;
                    while i < n && chars[i] != ';' {
                        i += 1;
                    }
                    if i < n {
                        i += 1;
                    }
                }
                'T' | 't' => {
                    // \T...;  — tracking
                    i += 2;
                    while i < n && chars[i] != ';' {
                        i += 1;
                    }
                    if i < n {
                        i += 1;
                    }
                }
                'W' | 'w' => {
                    // \W...;  — width factor
                    i += 2;
                    while i < n && chars[i] != ';' {
                        i += 1;
                    }
                    if i < n {
                        i += 1;
                    }
                }
                'Q' | 'q' => {
                    // \Q...;  — oblique angle
                    i += 2;
                    while i < n && chars[i] != ';' {
                        i += 1;
                    }
                    if i < n {
                        i += 1;
                    }
                }
                'A' | 'a' => {
                    // \A...;  — alignment
                    i += 2;
                    while i < n && chars[i] != ';' {
                        i += 1;
                    }
                    if i < n {
                        i += 1;
                    }
                }
                'S' | 's' => {
                    // \S...;  — stacking
                    i += 2;
                    while i < n && chars[i] != ';' {
                        i += 1;
                    }
                    if i < n {
                        i += 1;
                    }
                }
                'L' | 'l' => {
                    // \L or \l — underline on/off (ignore)
                    i += 2;
                }
                'O' | 'o' => {
                    // \O or \o — overline on/off (ignore)
                    i += 2;
                }
                'K' | 'k' => {
                    // \K or \k — strikethrough (ignore)
                    i += 2;
                }
                'U' | 'u' => {
                    // \U+XXXX — unicode
                    i += 2;
                    if i < n && chars[i] == '+' {
                        i += 1;
                        let mut hex = String::new();
                        while i < n && chars[i].is_ascii_hexdigit() {
                            hex.push(chars[i]);
                            i += 1;
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(c) = char::from_u32(code) {
                                out.push(c);
                            }
                        }
                    }
                }
                '\\' => {
                    out.push('\\');
                    i += 2;
                }
                '{' => {
                    out.push('{');
                    i += 2;
                }
                '}' => {
                    out.push('}');
                    i += 2;
                }
                _ => {
                    // Unknown escape, skip backslash
                    i += 1;
                }
            }
        } else if chars[i] == '{' {
            // Opening brace — just skip it (formatting group)
            i += 1;
        } else if chars[i] == '}' {
            // Closing brace
            i += 1;
        } else if chars[i] == '%' && i + 2 < n && chars[i + 1] == '%' {
            match chars[i + 2].to_ascii_lowercase() {
                'd' => {
                    out.push('°');
                    i += 3;
                }
                'p' => {
                    out.push('±');
                    i += 3;
                }
                'c' => {
                    out.push('⌀');
                    i += 3;
                }
                _ => {
                    out.push(chars[i]);
                    i += 1;
                }
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }

    out
}

pub fn draw_text(
    entity: &TextEntity,
    dxf: &DxfFile,
    viewport: &Viewport,
    pixmap: &mut Pixmap,
    transform: Transform,
) {
    if entity.text.is_empty() {
        return;
    }

    let layer_color = dxf
        .layers
        .get(&entity.layer)
        .map(|l| l.color)
        .unwrap_or(7);
    let (r, g, b) = aci_to_rgb(entity.color, layer_color);

    let height_px = viewport.world_len_to_px(entity.height).max(4.0);
    if height_px < 2.0 {
        return;
    }

    let (ox, oy) = viewport.world_to_px(entity.insert.x, entity.insert.y);

    let font_bytes = load_font(&entity.style);
    if font_bytes.is_empty() {
        return;
    }

    render_text_line(
        &entity.text,
        ox,
        oy,
        height_px,
        entity.rotation,
        r,
        g,
        b,
        &font_bytes,
        pixmap,
        transform,
    );
}

pub fn draw_mtext(
    entity: &MTextEntity,
    dxf: &DxfFile,
    viewport: &Viewport,
    pixmap: &mut Pixmap,
    transform: Transform,
) {
    if entity.text.is_empty() {
        return;
    }

    let layer_color = dxf
        .layers
        .get(&entity.layer)
        .map(|l| l.color)
        .unwrap_or(7);
    let (r, g, b) = aci_to_rgb(entity.color, layer_color);

    let height_px = viewport.world_len_to_px(entity.height).max(4.0);
    if height_px < 2.0 {
        return;
    }

    let (ox, oy) = viewport.world_to_px(entity.insert.x, entity.insert.y);

    let font_bytes = load_font(&entity.style);
    if font_bytes.is_empty() {
        return;
    }

    let cleaned = clean_mtext(&entity.text);
    let lines: Vec<&str> = cleaned.split('\n').collect();
    let line_height = height_px * 1.4;

    // Attachment point determines vertical alignment of the block
    // 1,2,3 = top row; 4,5,6 = middle row; 7,8,9 = bottom row
    let v_start = match entity.attachment {
        1..=3 => oy,                                             // top-anchored
        4..=6 => oy - (lines.len() as f32 * line_height) / 2.0, // middle
        7..=9 => oy - lines.len() as f32 * line_height,          // bottom
        _ => oy,
    };

    for (i, line) in lines.iter().enumerate() {
        let ly = v_start + i as f32 * line_height;
        render_text_line(
            line,
            ox,
            ly,
            height_px,
            entity.rotation,
            r,
            g,
            b,
            &font_bytes,
            pixmap,
            transform,
        );
    }
}

fn render_text_line(
    text: &str,
    ox: f32,
    oy: f32,
    height_px: f32,
    rotation_deg: f64,
    r: u8,
    g: u8,
    b: u8,
    font_bytes: &[u8],
    pixmap: &mut Pixmap,
    _transform: Transform,
) {
    if text.is_empty() {
        return;
    }

    let font = match FontVec::try_from_vec(font_bytes.to_vec()) {
        Ok(f) => f,
        Err(_) => match FontRef::try_from_slice(font_bytes) {
            Ok(_) => {
                // Use FontRef path
                render_text_line_ref(text, ox, oy, height_px, rotation_deg, r, g, b, font_bytes, pixmap, _transform);
                return;
            }
            Err(_) => return,
        },
    };

    let scale = PxScale::from(height_px);
    let scaled = font.as_scaled(scale);

    // DXF rotation is CCW degrees; screen Y is flipped so we negate
    let rot_rad = (-rotation_deg as f32).to_radians();
    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();

    let mut cursor_x = 0.0_f32;

    for c in text.chars() {
        let glyph = scaled.scaled_glyph(c);
        let advance = scaled.h_advance(glyph.id);

        if let Some(og) = font.outline_glyph(glyph) {
            let bounds = og.px_bounds();

            og.draw(|dx, dy, cov| {
                if cov < 0.01 {
                    return;
                }
                let local_x = cursor_x + bounds.min.x + dx as f32;
                let local_y = bounds.min.y + dy as f32;

                // Apply rotation around (ox, oy)
                let world_x = ox + local_x * cos_r - local_y * sin_r;
                let world_y = oy + local_x * sin_r + local_y * cos_r;

                let px = world_x as i32;
                let py = world_y as i32;

                if px >= 0
                    && py >= 0
                    && px < pixmap.width() as i32
                    && py < pixmap.height() as i32
                {
                    let alpha = (cov * 255.0) as u8;
                    blend_pixel(pixmap, px as u32, py as u32, r, g, b, alpha);
                }
            });
        }

        cursor_x += advance;
    }
}

fn render_text_line_ref(
    text: &str,
    ox: f32,
    oy: f32,
    height_px: f32,
    rotation_deg: f64,
    r: u8,
    g: u8,
    b: u8,
    font_bytes: &[u8],
    pixmap: &mut Pixmap,
    _transform: Transform,
) {
    let font = match FontRef::try_from_slice(font_bytes) {
        Ok(f) => f,
        Err(_) => return,
    };

    let scale = PxScale::from(height_px);
    let scaled = font.as_scaled(scale);

    let rot_rad = (-rotation_deg as f32).to_radians();
    let cos_r = rot_rad.cos();
    let sin_r = rot_rad.sin();

    let mut cursor_x = 0.0_f32;

    for c in text.chars() {
        let glyph = scaled.scaled_glyph(c);
        let advance = scaled.h_advance(glyph.id);

        if let Some(og) = font.outline_glyph(glyph) {
            let bounds = og.px_bounds();

            og.draw(|dx, dy, cov| {
                if cov < 0.01 {
                    return;
                }
                let local_x = cursor_x + bounds.min.x + dx as f32;
                let local_y = bounds.min.y + dy as f32;

                let world_x = ox + local_x * cos_r - local_y * sin_r;
                let world_y = oy + local_x * sin_r + local_y * cos_r;

                let px = world_x as i32;
                let py = world_y as i32;

                if px >= 0
                    && py >= 0
                    && px < pixmap.width() as i32
                    && py < pixmap.height() as i32
                {
                    let alpha = (cov * 255.0) as u8;
                    blend_pixel(pixmap, px as u32, py as u32, r, g, b, alpha);
                }
            });
        }

        cursor_x += advance;
    }
}

/// Alpha-blend a pixel over the current pixmap contents.
fn blend_pixel(pixmap: &mut Pixmap, x: u32, y: u32, r: u8, g: u8, b: u8, alpha: u8) {
    let idx = (y * pixmap.width() + x) as usize;
    let data = pixmap.pixels_mut();
    if idx >= data.len() {
        return;
    }

    let a = alpha as u32;
    let ia = 255 - a;
    let existing = data[idx];

    // tiny-skia stores pixels as premultiplied RGBA
    let er = existing.red() as u32;
    let eg = existing.green() as u32;
    let eb = existing.blue() as u32;
    let ea = existing.alpha() as u32;

    let nr = ((r as u32 * a + er * ia) / 255) as u8;
    let ng = ((g as u32 * a + eg * ia) / 255) as u8;
    let nb = ((b as u32 * a + eb * ia) / 255) as u8;
    let na = ((a + ea * ia / 255)) as u8;

    if let Some(px) = tiny_skia::PremultipliedColorU8::from_rgba(nr, ng, nb, na) {
        data[idx] = px;
    }
}
