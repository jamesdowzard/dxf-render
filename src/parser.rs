use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::types::*;

/// Read all line pairs from a DXF file as (group_code, value) tuples.
fn read_pairs(path: &Path) -> std::io::Result<Vec<(i32, String)>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut pairs = Vec::new();

    loop {
        let code_line = match lines.next() {
            Some(Ok(l)) => l,
            Some(Err(_)) | None => break,
        };
        let val_line = match lines.next() {
            Some(Ok(l)) => l,
            Some(Err(_)) | None => break,
        };
        let code_str = code_line.trim();
        let val = val_line.trim().to_string();
        if let Ok(code) = code_str.parse::<i32>() {
            pairs.push((code, val));
        }
    }

    Ok(pairs)
}

/// Parse a DXF file into a DxfFile structure.
pub fn parse_dxf(path: &Path) -> std::io::Result<DxfFile> {
    let pairs = read_pairs(path)?;
    let mut parser = DxfParser::new(pairs);
    let dxf = parser.parse();
    Ok(dxf)
}

struct DxfParser {
    pairs: Vec<(i32, String)>,
    pos: usize,
}

impl DxfParser {
    fn new(pairs: Vec<(i32, String)>) -> Self {
        Self { pairs, pos: 0 }
    }

    fn peek_code(&self) -> Option<i32> {
        self.pairs.get(self.pos).map(|(c, _)| *c)
    }

    fn peek_pair(&self) -> Option<(i32, &str)> {
        self.pairs.get(self.pos).map(|(c, v)| (*c, v.as_str()))
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn next_pair(&mut self) -> Option<(i32, String)> {
        if self.pos < self.pairs.len() {
            let (c, v) = &self.pairs[self.pos];
            let result = (*c, v.clone());
            self.pos += 1;
            Some(result)
        } else {
            None
        }
    }

    fn parse(&mut self) -> DxfFile {
        let mut layers: HashMap<String, Layer> = HashMap::new();
        let mut styles: HashMap<String, String> = HashMap::new();
        let mut blocks: HashMap<String, Vec<DxfEntity>> = HashMap::new();
        let mut entities: Vec<DxfEntity> = Vec::new();

        while let Some((code, val)) = self.next_pair() {
            if code == 0 && val == "SECTION" {
                if let Some((2, section_name)) = self.next_pair() {
                    match section_name.as_str() {
                        "HEADER" => self.skip_to_endsec(),
                        "CLASSES" => self.skip_to_endsec(),
                        "TABLES" => self.parse_tables(&mut layers, &mut styles),
                        "BLOCKS" => self.parse_blocks(&mut blocks, &layers, &styles),
                        "ENTITIES" => {
                            let ents = self.parse_entity_list(&layers, &styles);
                            entities.extend(ents);
                        }
                        "OBJECTS" => self.skip_to_endsec(),
                        "THUMBNAILIMAGE" => self.skip_to_endsec(),
                        _ => self.skip_to_endsec(),
                    }
                }
            }
        }

        DxfFile {
            entities,
            blocks,
            layers,
        }
    }

    fn skip_to_endsec(&mut self) {
        loop {
            match self.peek_pair() {
                Some((0, "ENDSEC")) | None => {
                    self.advance();
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_tables(
        &mut self,
        layers: &mut HashMap<String, Layer>,
        styles: &mut HashMap<String, String>,
    ) {
        let mut current_table: Option<String> = None;
        // Current layer being parsed
        let mut layer_name: Option<String> = None;
        let mut layer_color: i32 = 7;
        // Current style being parsed
        let mut style_name: Option<String> = None;
        let mut style_font: Option<String> = None;
        // Track if we've set the table type for the current TABLE entry
        let mut in_table_header = false;

        loop {
            match self.peek_pair() {
                None => break,
                Some((0, "ENDSEC")) => {
                    self.advance();
                    // Finalize any pending entries
                    if let (Some(name), Some(font)) = (style_name.take(), style_font.take()) {
                        styles.insert(name, font);
                    }
                    if let Some(name) = layer_name.take() {
                        layers.insert(name, Layer { color: layer_color });
                    }
                    return;
                }
                Some((0, "TABLE")) => {
                    self.advance();
                    in_table_header = true;
                    current_table = None;
                }
                Some((0, "ENDTAB")) => {
                    self.advance();
                    current_table = None;
                    in_table_header = false;
                }
                Some((0, "LAYER")) => {
                    self.advance();
                    // Finalize previous layer
                    if let Some(name) = layer_name.take() {
                        layers.insert(name, Layer { color: layer_color });
                    }
                    layer_name = None;
                    layer_color = 7;
                }
                Some((0, "STYLE")) => {
                    self.advance();
                    // Finalize previous style
                    if let (Some(name), Some(font)) = (style_name.take(), style_font.take()) {
                        styles.insert(name, font);
                    }
                    style_name = None;
                    style_font = None;
                }
                Some((0, _)) => {
                    // Other table entry type (LTYPE, VPORT, etc.) — skip its contents
                    self.advance();
                }
                Some((2, _)) => {
                    let val = self.pairs[self.pos].1.clone();
                    self.advance();
                    if in_table_header && current_table.is_none() {
                        // This is the table type name
                        current_table = Some(val.clone());
                        in_table_header = false;
                    } else {
                        // It's a name within the current record
                        match current_table.as_deref() {
                            Some("LAYER") => {
                                // Finalize previous if any
                                if let Some(prev) = layer_name.take() {
                                    layers.insert(prev, Layer { color: layer_color });
                                }
                                layer_name = Some(val);
                                layer_color = 7;
                            }
                            Some("STYLE") => {
                                if style_name.is_none() {
                                    style_name = Some(val);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Some((62, _)) => {
                    let val = self.pairs[self.pos].1.clone();
                    self.advance();
                    if current_table.as_deref() == Some("LAYER") {
                        layer_color = val.trim().parse().unwrap_or(7);
                    }
                }
                Some((3, _)) | Some((4, _)) => {
                    let val = self.pairs[self.pos].1.clone();
                    self.advance();
                    if current_table.as_deref() == Some("STYLE") && style_font.is_none() {
                        style_font = Some(val);
                    }
                }
                Some(_) => {
                    self.advance();
                }
            }
        }
    }

    fn parse_blocks(
        &mut self,
        blocks: &mut HashMap<String, Vec<DxfEntity>>,
        layers: &HashMap<String, Layer>,
        styles: &HashMap<String, String>,
    ) {
        loop {
            match self.peek_pair() {
                None => return,
                Some((0, "ENDSEC")) => {
                    self.advance();
                    return;
                }
                Some((0, "BLOCK")) => {
                    self.advance();
                    // Read block header until we hit an entity or ENDBLK
                    let mut block_name = String::new();
                    loop {
                        match self.peek_pair() {
                            Some((2, _)) => {
                                block_name = self.pairs[self.pos].1.clone();
                                self.advance();
                            }
                            Some((0, _)) => break, // hit an entity or ENDBLK
                            None => break,
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    // Parse entities inside the block
                    let mut block_entities = Vec::new();
                    loop {
                        match self.peek_pair() {
                            Some((0, "ENDBLK")) => {
                                self.advance();
                                // skip ENDBLK's properties
                                while self.peek_code().map(|c| c != 0).unwrap_or(false) {
                                    self.advance();
                                }
                                break;
                            }
                            Some((0, "ENDSEC")) | None => break,
                            Some((0, _)) => {
                                if let Some(entity) =
                                    self.parse_one_entity(layers, styles)
                                {
                                    block_entities.push(entity);
                                }
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    blocks.insert(block_name, block_entities);
                }
                Some((0, "ENDBLK")) => {
                    self.advance();
                    while self.peek_code().map(|c| c != 0).unwrap_or(false) {
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn parse_entity_list(
        &mut self,
        layers: &HashMap<String, Layer>,
        styles: &HashMap<String, String>,
    ) -> Vec<DxfEntity> {
        let mut entities = Vec::new();

        loop {
            match self.peek_pair() {
                None => break,
                Some((0, "ENDSEC")) | Some((0, "ENDBLK")) | Some((0, "EOF")) => {
                    self.advance();
                    break;
                }
                Some((0, "SEQEND")) => {
                    self.advance();
                    while self.peek_code().map(|c| c != 0).unwrap_or(false) {
                        self.advance();
                    }
                    break;
                }
                Some((0, _)) => {
                    if let Some(entity) = self.parse_one_entity(layers, styles) {
                        entities.push(entity);
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }

        entities
    }

    fn parse_one_entity(
        &mut self,
        layers: &HashMap<String, Layer>,
        styles: &HashMap<String, String>,
    ) -> Option<DxfEntity> {
        let (_, entity_type) = self.next_pair()?;

        match entity_type.as_str() {
            "LINE" => self.parse_line(),
            "LWPOLYLINE" => self.parse_lwpolyline(),
            "CIRCLE" => self.parse_circle(),
            "ARC" => self.parse_arc(),
            "TEXT" | "ATTRIB" => self.parse_text(styles),
            "MTEXT" => self.parse_mtext(styles),
            "INSERT" => self.parse_insert(layers, styles),
            // Skip these entity types
            _ => {
                self.skip_entity();
                None
            }
        }
    }

    /// Skip all group codes belonging to the current entity (until next code 0).
    fn skip_entity(&mut self) {
        while self.peek_code().map(|c| c != 0).unwrap_or(false) {
            self.advance();
        }
    }

    /// Read all remaining group codes for this entity (until next code 0).
    fn collect_entity_pairs(&mut self) -> Vec<(i32, String)> {
        let mut pairs = Vec::new();
        while self.peek_code().map(|c| c != 0).unwrap_or(false) {
            if let Some(pair) = self.next_pair() {
                pairs.push(pair);
            }
        }
        pairs
    }

    fn parse_line(&mut self) -> Option<DxfEntity> {
        let pairs = self.collect_entity_pairs();
        let mut x1 = 0.0_f64;
        let mut y1 = 0.0_f64;
        let mut x2 = 0.0_f64;
        let mut y2 = 0.0_f64;
        let mut layer = String::from("0");
        let mut color = 256_i32;

        for (code, val) in &pairs {
            match code {
                8 => layer = val.clone(),
                62 => color = val.trim().parse().unwrap_or(256),
                10 => x1 = val.trim().parse().unwrap_or(0.0),
                20 => y1 = val.trim().parse().unwrap_or(0.0),
                11 => x2 = val.trim().parse().unwrap_or(0.0),
                21 => y2 = val.trim().parse().unwrap_or(0.0),
                _ => {}
            }
        }

        Some(DxfEntity::Line(LineEntity {
            start: Vec2::new(x1, y1),
            end: Vec2::new(x2, y2),
            layer,
            color,
        }))
    }

    fn parse_lwpolyline(&mut self) -> Option<DxfEntity> {
        let pairs = self.collect_entity_pairs();
        let mut flags = 0_i32;
        let mut const_width = 0.0_f64;
        let mut layer = String::from("0");
        let mut color = 256_i32;
        let mut vertices: Vec<(Vec2, f64)> = Vec::new();
        let mut cur_x: Option<f64> = None;
        let mut cur_y: Option<f64> = None;
        let mut cur_bulge = 0.0_f64;

        for (code, val) in &pairs {
            match code {
                8 => layer = val.clone(),
                62 => color = val.trim().parse().unwrap_or(256),
                70 => flags = val.trim().parse().unwrap_or(0),
                43 => const_width = val.trim().parse().unwrap_or(0.0),
                10 => {
                    // New vertex x — push previous vertex if we have one
                    if let (Some(px), Some(py)) = (cur_x.take(), cur_y.take()) {
                        vertices.push((Vec2::new(px, py), cur_bulge));
                        cur_bulge = 0.0;
                    }
                    cur_x = Some(val.trim().parse().unwrap_or(0.0));
                }
                20 => {
                    cur_y = Some(val.trim().parse().unwrap_or(0.0));
                }
                42 => {
                    cur_bulge = val.trim().parse().unwrap_or(0.0);
                }
                _ => {}
            }
        }

        // Push last vertex
        if let (Some(px), Some(py)) = (cur_x, cur_y) {
            vertices.push((Vec2::new(px, py), cur_bulge));
        }

        let closed = (flags & 1) != 0;

        Some(DxfEntity::LwPolyline(LwPolylineEntity {
            vertices,
            closed,
            const_width,
            layer,
            color,
        }))
    }

    fn parse_circle(&mut self) -> Option<DxfEntity> {
        let pairs = self.collect_entity_pairs();
        let mut cx = 0.0_f64;
        let mut cy = 0.0_f64;
        let mut radius = 1.0_f64;
        let mut layer = String::from("0");
        let mut color = 256_i32;

        for (code, val) in &pairs {
            match code {
                8 => layer = val.clone(),
                62 => color = val.trim().parse().unwrap_or(256),
                10 => cx = val.trim().parse().unwrap_or(0.0),
                20 => cy = val.trim().parse().unwrap_or(0.0),
                40 => radius = val.trim().parse().unwrap_or(1.0),
                _ => {}
            }
        }

        Some(DxfEntity::Circle(CircleEntity {
            center: Vec2::new(cx, cy),
            radius,
            layer,
            color,
        }))
    }

    fn parse_arc(&mut self) -> Option<DxfEntity> {
        let pairs = self.collect_entity_pairs();
        let mut cx = 0.0_f64;
        let mut cy = 0.0_f64;
        let mut radius = 1.0_f64;
        let mut start_angle = 0.0_f64;
        let mut end_angle = 360.0_f64;
        let mut layer = String::from("0");
        let mut color = 256_i32;

        for (code, val) in &pairs {
            match code {
                8 => layer = val.clone(),
                62 => color = val.trim().parse().unwrap_or(256),
                10 => cx = val.trim().parse().unwrap_or(0.0),
                20 => cy = val.trim().parse().unwrap_or(0.0),
                40 => radius = val.trim().parse().unwrap_or(1.0),
                50 => start_angle = val.trim().parse().unwrap_or(0.0),
                51 => end_angle = val.trim().parse().unwrap_or(360.0),
                _ => {}
            }
        }

        // Convert arc to polyline approximation
        let mut end = end_angle;
        if end <= start_angle {
            end += 360.0;
        }
        let span = end - start_angle;
        let segments = ((span / 5.0).ceil() as usize).max(4).min(360);

        let mut vertices = Vec::new();
        for i in 0..=segments {
            let angle = (start_angle + span * i as f64 / segments as f64).to_radians();
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin();
            vertices.push((Vec2::new(x, y), 0.0));
        }

        Some(DxfEntity::LwPolyline(LwPolylineEntity {
            vertices,
            closed: false,
            const_width: 0.0,
            layer,
            color,
        }))
    }

    fn parse_text(&mut self, _styles: &HashMap<String, String>) -> Option<DxfEntity> {
        let pairs = self.collect_entity_pairs();
        let mut ins_x = 0.0_f64;
        let mut ins_y = 0.0_f64;
        let mut height = 1.0_f64;
        let mut rotation = 0.0_f64;
        let mut text = String::new();
        let mut layer = String::from("0");
        let mut color = 256_i32;
        let mut style = String::from("Standard");
        let mut h_align = 0_i32;
        let mut v_align = 0_i32;

        for (code, val) in &pairs {
            match code {
                8 => layer = val.clone(),
                62 => color = val.trim().parse().unwrap_or(256),
                10 => ins_x = val.trim().parse().unwrap_or(0.0),
                20 => ins_y = val.trim().parse().unwrap_or(0.0),
                40 => height = val.trim().parse().unwrap_or(1.0),
                50 => rotation = val.trim().parse().unwrap_or(0.0),
                1 => text = val.clone(),
                7 => style = val.clone(),
                72 => h_align = val.trim().parse().unwrap_or(0),
                73 => v_align = val.trim().parse().unwrap_or(0),
                _ => {}
            }
        }

        Some(DxfEntity::Text(TextEntity {
            text,
            insert: Vec2::new(ins_x, ins_y),
            height,
            rotation,
            layer,
            color,
            style,
            h_align,
            v_align,
        }))
    }

    fn parse_mtext(&mut self, _styles: &HashMap<String, String>) -> Option<DxfEntity> {
        let pairs = self.collect_entity_pairs();
        let mut ins_x = 0.0_f64;
        let mut ins_y = 0.0_f64;
        let mut height = 1.0_f64;
        let mut rotation = 0.0_f64;
        let mut layer = String::from("0");
        let mut color = 256_i32;
        let mut style = String::from("Standard");
        let mut attachment = 1_i32;
        let mut continuation_parts: Vec<String> = Vec::new();
        let mut main_text = String::new();
        let mut dir_x = 0.0_f64;
        let mut dir_y = 0.0_f64;
        let mut has_dir = false;
        let mut has_rotation = false;

        for (code, val) in &pairs {
            match code {
                8 => layer = val.clone(),
                62 => color = val.trim().parse().unwrap_or(256),
                10 => ins_x = val.trim().parse().unwrap_or(0.0),
                20 => ins_y = val.trim().parse().unwrap_or(0.0),
                40 => height = val.trim().parse().unwrap_or(1.0),
                50 => {
                    rotation = val.trim().parse().unwrap_or(0.0);
                    has_rotation = true;
                }
                1 => main_text = val.clone(),
                3 => continuation_parts.push(val.clone()),
                7 => style = val.clone(),
                71 => attachment = val.trim().parse().unwrap_or(1),
                11 => {
                    dir_x = val.trim().parse().unwrap_or(0.0);
                    has_dir = true;
                }
                21 => {
                    dir_y = val.trim().parse().unwrap_or(0.0);
                    has_dir = true;
                }
                _ => {}
            }
        }

        let mut text = continuation_parts.join("");
        text.push_str(&main_text);

        // If direction vector given and no explicit rotation, compute from direction
        if !has_rotation && has_dir && (dir_x.abs() > 1e-9 || dir_y.abs() > 1e-9) {
            rotation = dir_y.atan2(dir_x).to_degrees();
        }

        Some(DxfEntity::MText(MTextEntity {
            text,
            insert: Vec2::new(ins_x, ins_y),
            height,
            rotation,
            layer,
            color,
            style,
            attachment,
        }))
    }

    fn parse_insert(
        &mut self,
        _layers: &HashMap<String, Layer>,
        styles: &HashMap<String, String>,
    ) -> Option<DxfEntity> {
        let mut ins_x = 0.0_f64;
        let mut ins_y = 0.0_f64;
        let mut scale_x = 1.0_f64;
        let mut scale_y = 1.0_f64;
        let mut rotation = 0.0_f64;
        let mut block_name = String::new();
        let mut layer = String::from("0");
        let mut color = 256_i32;
        let mut has_attribs = false;

        while self.peek_code().map(|c| c != 0).unwrap_or(false) {
            if let Some((code, val)) = self.next_pair() {
                match code {
                    8 => layer = val.clone(),
                    62 => color = val.trim().parse().unwrap_or(256),
                    2 => block_name = val.clone(),
                    10 => ins_x = val.trim().parse().unwrap_or(0.0),
                    20 => ins_y = val.trim().parse().unwrap_or(0.0),
                    41 => scale_x = val.trim().parse().unwrap_or(1.0),
                    42 => scale_y = val.trim().parse().unwrap_or(1.0),
                    50 => rotation = val.trim().parse().unwrap_or(0.0),
                    66 => has_attribs = val.trim() == "1",
                    _ => {}
                }
            }
        }

        let mut attribs: Vec<TextEntity> = Vec::new();

        if has_attribs {
            // Parse ATTRIB entities until SEQEND
            loop {
                match self.peek_pair() {
                    None => break,
                    Some((0, "SEQEND")) => {
                        self.advance();
                        self.skip_entity();
                        break;
                    }
                    Some((0, "ATTRIB")) => {
                        self.advance();
                        if let Some(DxfEntity::Text(t)) = self.parse_text(styles) {
                            attribs.push(t);
                        }
                    }
                    Some((0, _)) => break,
                    _ => {
                        self.advance();
                    }
                }
            }
        }

        Some(DxfEntity::Insert(InsertEntity {
            block_name,
            insert: Vec2::new(ins_x, ins_y),
            scale_x,
            scale_y,
            rotation,
            layer,
            color,
            attribs,
        }))
    }
}
