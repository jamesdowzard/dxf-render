use tiny_skia::{Color, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::color::aci_to_rgb;
use crate::transform::Viewport;
use crate::types::{DxfFile, LineEntity};

pub fn draw_line(
    entity: &LineEntity,
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

    let (x1, y1) = viewport.world_to_px(entity.start.x, entity.start.y);
    let (x2, y2) = viewport.world_to_px(entity.end.x, entity.end.y);

    let mut pb = PathBuilder::new();
    pb.move_to(x1, y1);
    pb.line_to(x2, y2);

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
