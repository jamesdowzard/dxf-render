use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::color::aci_to_rgb;
use crate::transform::Viewport;
use crate::types::{CircleEntity, DxfFile};

pub fn draw_circle(
    entity: &CircleEntity,
    dxf: &DxfFile,
    viewport: &Viewport,
    pixmap: &mut Pixmap,
    transform: Transform,
) {
    let layer_color = dxf
        .layers
        .get(&entity.layer)
        .map(|l| l.color)
        .unwrap_or(7);
    let (r, g, b) = aci_to_rgb(entity.color, layer_color);

    let (cx, cy) = viewport.world_to_px(entity.center.x, entity.center.y);
    let radius_px = viewport.world_len_to_px(entity.radius).max(0.5);

    // Approximate circle with bezier curves (4 cubic beziers)
    // Magic constant k = 4*(sqrt(2)-1)/3 ≈ 0.5522847
    let k = 0.5522847_f32 * radius_px;

    let mut pb = PathBuilder::new();
    pb.move_to(cx + radius_px, cy);

    // Top-right arc
    pb.cubic_to(
        cx + radius_px,
        cy - k,
        cx + k,
        cy - radius_px,
        cx,
        cy - radius_px,
    );
    // Top-left arc
    pb.cubic_to(
        cx - k,
        cy - radius_px,
        cx - radius_px,
        cy - k,
        cx - radius_px,
        cy,
    );
    // Bottom-left arc
    pb.cubic_to(
        cx - radius_px,
        cy + k,
        cx - k,
        cy + radius_px,
        cx,
        cy + radius_px,
    );
    // Bottom-right arc
    pb.cubic_to(
        cx + k,
        cy + radius_px,
        cx + radius_px,
        cy + k,
        cx + radius_px,
        cy,
    );
    pb.close();

    let path = match pb.finish() {
        Some(p) => p,
        None => return,
    };

    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(r, g, b, 255));
    paint.anti_alias = true;

    let stroke = Stroke {
        width: 1.0_f32.max(0.5),
        ..Default::default()
    };

    pixmap.stroke_path(&path, &paint, &stroke, transform, None);
}
