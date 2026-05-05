use std::collections::HashMap;

use crate::types::*;

pub struct Viewport {
    pub min_x: f64,
    pub max_y: f64,
    pub scale: f64,
    pub margin: f64,
    pub width_px: u32,
    pub height_px: u32,
}

impl Viewport {
    pub fn from_entities(
        entities: &[DxfEntity],
        blocks: &HashMap<String, Vec<DxfEntity>>,
        output_width: u32,
    ) -> Self {
        let mut xs: Vec<f64> = Vec::new();
        let mut ys: Vec<f64> = Vec::new();

        collect_coords(entities, blocks, &mut xs, &mut ys, 0);

        if xs.is_empty() || ys.is_empty() {
            return Viewport {
                min_x: 0.0,
                max_y: 1.0,
                scale: output_width as f64,
                margin: 0.0,
                width_px: output_width,
                height_px: output_width,
            };
        }

        let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_y = ys.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_y = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let world_w = (max_x - min_x).max(1e-9);
        let world_h = (max_y - min_y).max(1e-9);

        // 2% margin on each side
        let margin_frac = 0.02;
        let draw_w = output_width as f64 * (1.0 - 2.0 * margin_frac);
        let scale = draw_w / world_w;

        let height_px = (world_h * scale + 2.0 * margin_frac * output_width as f64 * (world_h / world_w).max(1.0)).ceil() as u32;
        let height_px = height_px.max(1);

        let margin = output_width as f64 * margin_frac;

        Viewport {
            min_x,
            max_y,
            scale,
            margin,
            width_px: output_width,
            height_px,
        }
    }

    /// Convert world coordinates to pixel coordinates.
    /// Y-axis is flipped: DXF Y increases upward, screen Y increases downward.
    pub fn world_to_px(&self, x: f64, y: f64) -> (f32, f32) {
        let px = ((x - self.min_x) * self.scale + self.margin) as f32;
        let py = ((self.max_y - y) * self.scale + self.margin) as f32;
        (px, py)
    }

    /// Convert a world-space length to pixels.
    pub fn world_len_to_px(&self, len: f64) -> f32 {
        (len * self.scale) as f32
    }
}

fn collect_coords(
    entities: &[DxfEntity],
    blocks: &HashMap<String, Vec<DxfEntity>>,
    xs: &mut Vec<f64>,
    ys: &mut Vec<f64>,
    depth: usize,
) {
    if depth > 8 {
        return;
    }

    for entity in entities {
        match entity {
            DxfEntity::Line(l) => {
                xs.push(l.start.x);
                xs.push(l.end.x);
                ys.push(l.start.y);
                ys.push(l.end.y);
            }
            DxfEntity::LwPolyline(p) => {
                for (v, _) in &p.vertices {
                    xs.push(v.x);
                    ys.push(v.y);
                }
            }
            DxfEntity::Circle(c) => {
                xs.push(c.center.x - c.radius);
                xs.push(c.center.x + c.radius);
                ys.push(c.center.y - c.radius);
                ys.push(c.center.y + c.radius);
            }
            DxfEntity::Text(t) => {
                xs.push(t.insert.x);
                ys.push(t.insert.y);
            }
            DxfEntity::MText(m) => {
                xs.push(m.insert.x);
                ys.push(m.insert.y);
            }
            DxfEntity::Insert(ins) => {
                xs.push(ins.insert.x);
                ys.push(ins.insert.y);
                // Also recurse into the block to get better bounds
                if let Some(block_ents) = blocks.get(&ins.block_name) {
                    // Approximate: collect block coords offset by insertion point
                    // (ignoring rotation/scale for bounds estimation)
                    let mut bxs = Vec::new();
                    let mut bys = Vec::new();
                    collect_coords(block_ents, blocks, &mut bxs, &mut bys, depth + 1);
                    for bx in &bxs {
                        xs.push(ins.insert.x + bx * ins.scale_x);
                    }
                    for by in &bys {
                        ys.push(ins.insert.y + by * ins.scale_y);
                    }
                }
            }
        }
    }
}
