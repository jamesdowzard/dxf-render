use tiny_skia::{Pixmap, Transform};

use crate::draw::circle::draw_circle;
use crate::draw::insert::draw_insert;
use crate::draw::line::draw_line;
use crate::draw::polyline::draw_lwpolyline;
use crate::draw::text::{draw_mtext, draw_text};
use crate::transform::Viewport;
use crate::types::{DxfEntity, DxfFile};

pub fn render(dxf: &DxfFile, viewport: &Viewport) -> Pixmap {
    let mut pixmap = Pixmap::new(viewport.width_px, viewport.height_px)
        .expect("Failed to create pixmap");
    pixmap.fill(tiny_skia::Color::WHITE);

    let identity = Transform::identity();

    for entity in &dxf.entities {
        render_entity(entity, dxf, viewport, &mut pixmap, identity, 0);
    }

    pixmap
}

fn render_entity(
    entity: &DxfEntity,
    dxf: &DxfFile,
    viewport: &Viewport,
    pixmap: &mut Pixmap,
    transform: Transform,
    depth: usize,
) {
    match entity {
        DxfEntity::Line(e) => draw_line(e, dxf, viewport, pixmap, transform),
        DxfEntity::LwPolyline(e) => draw_lwpolyline(e, dxf, viewport, pixmap, transform),
        DxfEntity::Circle(e) => draw_circle(e, dxf, viewport, pixmap, transform),
        DxfEntity::Text(e) => draw_text(e, dxf, viewport, pixmap, transform),
        DxfEntity::MText(e) => draw_mtext(e, dxf, viewport, pixmap, transform),
        DxfEntity::Insert(e) => draw_insert(e, dxf, viewport, pixmap, transform, depth),
    }
}
