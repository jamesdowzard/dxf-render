use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::color::aci_to_rgb;
use crate::transform::Viewport;
use crate::types::{DxfFile, LwPolylineEntity, Vec2};

pub fn draw_lwpolyline(
    entity: &LwPolylineEntity,
    dxf: &DxfFile,
    viewport: &Viewport,
    pixmap: &mut Pixmap,
    transform: Transform,
) {
    if entity.vertices.len() < 2 {
        return;
    }

    let layer_color = dxf
        .layers
        .get(&entity.layer)
        .map(|l| l.color)
        .unwrap_or(7);
    let (r, g, b) = aci_to_rgb(entity.color, layer_color);

    let stroke_width = if entity.const_width > 0.0 {
        viewport.world_len_to_px(entity.const_width).max(0.5)
    } else {
        1.0_f32.max(0.5)
    };

    let mut pb = PathBuilder::new();
    let (sx, sy) = viewport.world_to_px(entity.vertices[0].0.x, entity.vertices[0].0.y);
    pb.move_to(sx, sy);

    let n = entity.vertices.len();
    let segment_count = if entity.closed { n } else { n - 1 };

    for i in 0..segment_count {
        let (p1, bulge) = &entity.vertices[i];
        let p2 = &entity.vertices[(i + 1) % n].0;

        if bulge.abs() < 1e-9 {
            // Straight segment
            let (ex, ey) = viewport.world_to_px(p2.x, p2.y);
            pb.line_to(ex, ey);
        } else {
            // Arc approximation via 16 line segments
            let arc_pts = bulge_to_arc_points(p1, p2, *bulge, 16);
            for pt in &arc_pts {
                let (px, py) = viewport.world_to_px(pt.x, pt.y);
                pb.line_to(px, py);
            }
        }
    }

    if entity.closed {
        pb.close();
    }

    let path = match pb.finish() {
        Some(p) => p,
        None => return,
    };

    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(r, g, b, 255));
    paint.anti_alias = true;

    let stroke = Stroke {
        width: stroke_width,
        ..Default::default()
    };

    pixmap.stroke_path(&path, &paint, &stroke, transform, None);
}

/// Convert a bulge segment (P1 → P2 with bulge B) to a series of intermediate points.
/// bulge B: positive = CCW, negative = CW
/// theta = 4 * atan(|B|) = included angle
/// Returns points AFTER P1 (including P2) so they can be appended to the path.
fn bulge_to_arc_points(p1: &Vec2, p2: &Vec2, bulge: f64, segments: usize) -> Vec<Vec2> {
    let theta = 4.0 * bulge.abs().atan();
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    let chord = (dx * dx + dy * dy).sqrt();

    if chord < 1e-12 || theta < 1e-9 {
        return vec![*p2];
    }

    let radius = chord / (2.0 * (theta / 2.0).sin());

    // Midpoint of chord
    let mx = (p1.x + p2.x) / 2.0;
    let my = (p1.y + p2.y) / 2.0;

    // Perpendicular to chord
    let perp_x = -dy / chord;
    let perp_y = dx / chord;

    // Distance from midpoint to center
    let d = (radius * radius - (chord / 2.0).powi(2)).max(0.0).sqrt();

    // perp = (-dy, dx)/chord points to the LEFT of P1→P2 direction.
    // Positive bulge = CCW from P1 to P2, which requires the center on the
    // LEFT of P1→P2 (so the CCW sweep traces the SHORT arc on the right side).
    let sign = if bulge > 0.0 { 1.0 } else { -1.0 };
    let cx = mx + sign * perp_x * d;
    let cy = my + sign * perp_y * d;

    // Start angle from P1 to center
    let start_angle = (p1.y - cy).atan2(p1.x - cx);

    // Use theta directly for the sweep
    let sweep = if bulge > 0.0 { theta } else { -theta };

    let mut pts = Vec::with_capacity(segments);
    for i in 1..=segments {
        let frac = i as f64 / segments as f64;
        let angle = start_angle + sweep * frac;
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();
        pts.push(Vec2::new(x, y));
    }

    pts
}
