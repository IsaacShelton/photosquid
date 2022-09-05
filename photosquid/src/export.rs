use std::path::PathBuf;

use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers};
use svg::{
    node::element::{path::Data, Path},
    Document,
};

use crate::{data::RectData, ocean::Ocean};

// Let's use our own custom vertex type instead of the default one.
#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 2],
}

pub fn export(filename: PathBuf, viewport: &RectData, ocean: &Ocean) -> std::io::Result<()> {
    let mut builder = lyon::path::Path::builder();

    for squid_ref in ocean.get_squids_lowest() {
        if let Some(squid) = ocean.get(squid_ref) {
            squid.build(&mut builder);
        }
    }

    // Create svg representation
    let path = builder.build();
    let document = stroke(&path, viewport);
    let document = document.add(fill(&path, viewport));

    // Save svg file
    svg::save(&filename, &document)
}

fn stroke(lyon_path: &lyon::path::Path, viewport: &RectData) -> svg::Document {
    // Will contain the result of the tessellation.
    let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();

    // Create tessellated geometry for stroke
    tessellator
        .tessellate_path(
            lyon_path,
            &StrokeOptions::default(),
            &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| Vertex {
                position: vertex.position().to_array(),
            }),
        )
        .unwrap();

    // Create svg representation for stroke geometry
    make_document(geometry, "blue", viewport)
}

fn fill(lyon_path: &lyon::path::Path, viewport: &RectData) -> svg::Document {
    // Will contain the result of the tessellation.
    let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    // Create tessellated geometry for fill
    tessellator
        .tessellate_path(
            lyon_path,
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| Vertex {
                position: vertex.position().to_array(),
            }),
        )
        .unwrap();

    // Create svg representation for fill geometry
    make_document(geometry, "red", viewport)
}

fn make_document(geometry: VertexBuffers<Vertex, u16>, fill_color: &str, viewport: &RectData) -> svg::Document {
    let RectData { position, size, .. } = viewport;
    let position = position.reveal();

    // Create empty svg document with view window
    let mut document = Document::new().set("viewBox", (position.x, position.y, size.x, size.y));

    // Add each triangle
    for triangle in 0..(geometry.indices.len() / 3) {
        let mut data = Data::new();

        for i in 0..3 {
            let vertex = geometry.vertices[geometry.indices[triangle * 3 + i] as usize];

            if i == 0 {
                data = data.move_to((vertex.position[0], vertex.position[1]));
            } else {
                data = data.line_to((vertex.position[0], vertex.position[1]));
            }
        }

        data = data.close();

        let path = Path::new()
            .set("fill", fill_color)
            .set("stroke", "black")
            .set("stroke-width", 0.1)
            .set("d", data);

        document = document.add(path);
    }

    document
}
