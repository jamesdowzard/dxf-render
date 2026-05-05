use tiny_skia::{Pixmap, Transform};

use crate::transform::Viewport;
use crate::types::{DxfEntity, DxfFile, InsertEntity};

use super::circle::draw_circle;
use super::line::draw_line;
use super::polyline::draw_lwpolyline;
use super::text::{draw_mtext, draw_text};

pub fn draw_insert(
    entity: &InsertEntity,
    dxf: &DxfFile,
    viewport: &Viewport,
    pixmap: &mut Pixmap,
    parent_transform: Transform,
    depth: usize,
) {
    if depth > 8 {
        return;
    }

    let (px, py) = viewport.world_to_px(entity.insert.x, entity.insert.y);

    // DXF rotation is CCW degrees; screen Y is flipped so negate rotation
    let rot_deg = -entity.rotation as f32;

    // Build the transform for this insert:
    // 1. Scale the block content
    // 2. Rotate (negative because Y-flipped)
    // 3. Translate to insertion point
    let local_transform = Transform::from_translate(px, py)
        .pre_concat(Transform::from_rotate(rot_deg))
        .pre_concat(Transform::from_scale(
            entity.scale_x as f32,
            entity.scale_y as f32,
        ));

    let combined = parent_transform.pre_concat(local_transform);

    // Get the block entities
    let block_entities = match dxf.blocks.get(&entity.block_name) {
        Some(ents) => ents.clone(),
        None => return,
    };

    for block_entity in &block_entities {
        // For entities inside a block, coordinates are in block-local space.
        // We render them at their local coords, using the combined transform
        // to position them correctly in screen space.
        draw_entity_in_block(block_entity, dxf, viewport, pixmap, combined, depth + 1);
    }

    // Draw ATTRIBs at their own positions (absolute, not block-relative)
    for attrib in &entity.attribs {
        draw_text(attrib, dxf, viewport, pixmap, parent_transform);
    }
}

/// Draw an entity that lives inside a block, using the block's local coordinate system.
/// The transform maps block-local pixels to screen pixels.
fn draw_entity_in_block(
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
        DxfEntity::Insert(e) => {
            // Nested INSERT — recurse
            // For nested inserts we need to handle the coordinate conversion differently.
            // The inner insert's position is in block-local world space, so we need to
            // convert it to pixels, then apply the outer transform.
            let (local_px, local_py) =
                viewport.world_to_px(e.insert.x, e.insert.y);

            let rot_deg = -e.rotation as f32;
            let inner_local = Transform::from_translate(local_px, local_py)
                .pre_concat(Transform::from_rotate(rot_deg))
                .pre_concat(Transform::from_scale(
                    e.scale_x as f32,
                    e.scale_y as f32,
                ));

            let combined = transform.pre_concat(inner_local);

            if let Some(block_ents) = dxf.blocks.get(&e.block_name) {
                let block_ents = block_ents.clone();
                for be in &block_ents {
                    draw_entity_in_block(be, dxf, viewport, pixmap, combined, depth + 1);
                }
            }

            for attrib in &e.attribs {
                draw_text(attrib, dxf, viewport, pixmap, Transform::identity());
            }
        }
    }
}
