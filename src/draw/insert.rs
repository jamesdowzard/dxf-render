use tiny_skia::{Pixmap, Transform};

use crate::transform::Viewport;
use crate::types::*;

use super::circle::draw_circle;
use super::line::draw_line;
use super::polyline::draw_lwpolyline;
use super::text::{draw_mtext, draw_text};

/// 2D affine transform mapping block-local coords to parent-frame coords.
/// Stored row-major as [a b tx; c d ty]; bottom row is implicit [0 0 1].
#[derive(Clone, Copy)]
struct Affine2 {
    a: f64,
    b: f64,
    tx: f64,
    c: f64,
    d: f64,
    ty: f64,
}

impl Affine2 {
    /// World-from-local for an INSERT: scale, rotate (CCW degrees), then translate.
    fn from_insert(insert: Vec2, scale: (f64, f64), rotation_deg: f64) -> Self {
        let theta = rotation_deg.to_radians();
        let (s, cs) = (theta.sin(), theta.cos());
        let (sx, sy) = scale;
        Self {
            a: cs * sx,
            b: -s * sy,
            tx: insert.x,
            c: s * sx,
            d: cs * sy,
            ty: insert.y,
        }
    }

    /// Compose: (self ∘ other).apply(p) == self.apply(other.apply(p)).
    fn compose(&self, other: &Affine2) -> Affine2 {
        Affine2 {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            tx: self.a * other.tx + self.b * other.ty + self.tx,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            ty: self.c * other.tx + self.d * other.ty + self.ty,
        }
    }

    fn apply(&self, p: Vec2) -> Vec2 {
        Vec2::new(
            self.a * p.x + self.b * p.y + self.tx,
            self.c * p.x + self.d * p.y + self.ty,
        )
    }

    /// Average column length — uniform-scale magnification factor.
    /// For non-uniform scale (sx ≠ sy) the result is an approximation,
    /// fine for circle radii / text heights / line widths.
    fn scale_factor(&self) -> f64 {
        let col0 = (self.a * self.a + self.c * self.c).sqrt();
        let col1 = (self.b * self.b + self.d * self.d).sqrt();
        (col0 + col1) / 2.0
    }

    /// Rotation extracted from the first column (degrees CCW).
    fn rotation_deg(&self) -> f64 {
        self.c.atan2(self.a).to_degrees()
    }
}

pub fn draw_insert(
    entity: &InsertEntity,
    dxf: &DxfFile,
    viewport: &Viewport,
    pixmap: &mut Pixmap,
    _parent_transform: Transform,
    depth: usize,
) {
    if depth > 8 {
        return;
    }

    let xform = Affine2::from_insert(
        entity.insert,
        (entity.scale_x, entity.scale_y),
        entity.rotation,
    );

    let block_entities = match dxf.blocks.get(&entity.block_name) {
        Some(ents) => ents.clone(),
        None => Vec::new(),
    };

    for block_entity in &block_entities {
        draw_block_entity(block_entity, &xform, dxf, viewport, pixmap, depth + 1);
    }

    // ATTRIBs are written at absolute world coords already (DXF convention).
    for attrib in &entity.attribs {
        draw_text(attrib, dxf, viewport, pixmap, Transform::identity());
    }
}

/// Render a block-internal entity by pre-transforming its coordinates from
/// block-local to world space, then delegating to the standard drawing
/// function with an identity tiny-skia transform.
fn draw_block_entity(
    entity: &DxfEntity,
    xform: &Affine2,
    dxf: &DxfFile,
    viewport: &Viewport,
    pixmap: &mut Pixmap,
    depth: usize,
) {
    if depth > 8 {
        return;
    }

    match entity {
        DxfEntity::Line(e) => {
            let new_e = LineEntity {
                start: xform.apply(e.start),
                end: xform.apply(e.end),
                layer: e.layer.clone(),
                color: e.color,
            };
            draw_line(&new_e, dxf, viewport, pixmap, Transform::identity());
        }
        DxfEntity::LwPolyline(e) => {
            let new_e = LwPolylineEntity {
                vertices: e
                    .vertices
                    .iter()
                    .map(|(v, b)| (xform.apply(*v), *b))
                    .collect(),
                closed: e.closed,
                const_width: e.const_width * xform.scale_factor(),
                layer: e.layer.clone(),
                color: e.color,
            };
            draw_lwpolyline(&new_e, dxf, viewport, pixmap, Transform::identity());
        }
        DxfEntity::Circle(e) => {
            let new_e = CircleEntity {
                center: xform.apply(e.center),
                radius: e.radius * xform.scale_factor(),
                layer: e.layer.clone(),
                color: e.color,
            };
            draw_circle(&new_e, dxf, viewport, pixmap, Transform::identity());
        }
        DxfEntity::Text(e) => {
            let new_e = TextEntity {
                text: e.text.clone(),
                insert: xform.apply(e.insert),
                height: e.height * xform.scale_factor(),
                rotation: e.rotation + xform.rotation_deg(),
                layer: e.layer.clone(),
                color: e.color,
                style: e.style.clone(),
                h_align: e.h_align,
                v_align: e.v_align,
            };
            draw_text(&new_e, dxf, viewport, pixmap, Transform::identity());
        }
        DxfEntity::MText(e) => {
            let new_e = MTextEntity {
                text: e.text.clone(),
                insert: xform.apply(e.insert),
                height: e.height * xform.scale_factor(),
                rotation: e.rotation + xform.rotation_deg(),
                layer: e.layer.clone(),
                color: e.color,
                style: e.style.clone(),
                attachment: e.attachment,
            };
            draw_mtext(&new_e, dxf, viewport, pixmap, Transform::identity());
        }
        DxfEntity::Insert(e) => {
            let inner = Affine2::from_insert(e.insert, (e.scale_x, e.scale_y), e.rotation);
            let combined = xform.compose(&inner);

            if let Some(block_ents) = dxf.blocks.get(&e.block_name) {
                let block_ents = block_ents.clone();
                for be in &block_ents {
                    draw_block_entity(be, &combined, dxf, viewport, pixmap, depth + 1);
                }
            }
            for attrib in &e.attribs {
                draw_text(attrib, dxf, viewport, pixmap, Transform::identity());
            }
        }
    }
}
