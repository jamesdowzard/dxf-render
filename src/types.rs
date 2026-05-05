use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone)]
pub struct LineEntity {
    pub start: Vec2,
    pub end: Vec2,
    pub layer: String,
    pub color: i32, // ACI color, 0=BYBLOCK, 256=BYLAYER
}

#[derive(Debug, Clone)]
pub struct LwPolylineEntity {
    pub vertices: Vec<(Vec2, f64)>, // (point, bulge_leaving_this_vertex)
    pub closed: bool,
    pub const_width: f64,
    pub layer: String,
    pub color: i32,
}

#[derive(Debug, Clone)]
pub struct CircleEntity {
    pub center: Vec2,
    pub radius: f64,
    pub layer: String,
    pub color: i32,
}

#[derive(Debug, Clone)]
pub struct TextEntity {
    pub text: String,
    pub insert: Vec2,
    pub height: f64,
    pub rotation: f64, // degrees
    pub layer: String,
    pub color: i32,
    pub style: String,
    #[allow(dead_code)]
    pub h_align: i32,
    #[allow(dead_code)]
    pub v_align: i32,
}

#[derive(Debug, Clone)]
pub struct MTextEntity {
    pub text: String, // already concatenated from group 3+1
    pub insert: Vec2,
    pub height: f64,
    pub rotation: f64, // degrees (from direction vector or group 50)
    pub layer: String,
    pub color: i32,
    pub style: String,
    pub attachment: i32, // group 71: 1=top-left, 5=middle-centre, 9=bottom-right
}

#[derive(Debug, Clone)]
pub struct InsertEntity {
    pub block_name: String,
    pub insert: Vec2,
    pub scale_x: f64,
    pub scale_y: f64,
    pub rotation: f64, // degrees
    #[allow(dead_code)]
    pub layer: String,
    #[allow(dead_code)]
    pub color: i32,
    pub attribs: Vec<TextEntity>,
}

#[derive(Debug, Clone)]
pub enum DxfEntity {
    Line(LineEntity),
    LwPolyline(LwPolylineEntity),
    Circle(CircleEntity),
    Text(TextEntity),
    MText(MTextEntity),
    Insert(InsertEntity),
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub color: i32,
}

#[derive(Debug)]
pub struct DxfFile {
    pub entities: Vec<DxfEntity>,
    pub blocks: HashMap<String, Vec<DxfEntity>>,
    pub layers: HashMap<String, Layer>,
}
