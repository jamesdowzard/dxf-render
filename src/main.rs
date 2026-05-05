mod color;
mod draw;
mod font;
mod parser;
mod renderer;
mod transform;
mod types;

use clap::Parser;

#[derive(Parser)]
#[command(name = "dxf-render", about = "Convert DXF files to PNG images")]
struct Args {
    /// Input DXF file path
    input: std::path::PathBuf,
    /// Output PNG file path
    output: std::path::PathBuf,
    /// Output image width in pixels
    #[arg(long, default_value = "2048")]
    width: u32,
}

fn main() {
    let args = Args::parse();

    let dxf = parser::parse_dxf(&args.input).expect("Failed to parse DXF");
    eprintln!(
        "Parsed {} entities, {} blocks, {} layers",
        dxf.entities.len(),
        dxf.blocks.len(),
        dxf.layers.len()
    );

    let viewport = transform::Viewport::from_entities(&dxf.entities, &dxf.blocks, args.width);
    eprintln!(
        "Viewport: {}x{} px, scale={:.4}",
        viewport.width_px, viewport.height_px, viewport.scale
    );

    let pixmap = renderer::render(&dxf, &viewport);
    pixmap
        .save_png(&args.output)
        .expect("Failed to save PNG");

    eprintln!(
        "Rendered {}x{} → {}",
        pixmap.width(),
        pixmap.height(),
        args.output.display()
    );
}
