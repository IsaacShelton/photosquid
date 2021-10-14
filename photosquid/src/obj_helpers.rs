use crate::vertex::Vertex;
use stringreader::StringReader;

pub fn obj_data_to_shape(data: &str) -> Vec<Vertex> {
    let (models, _materials) = tobj::load_obj_buf(
        &mut std::io::BufReader::new(StringReader::new(data)),
        &tobj::LoadOptions {
            triangulate: true,
            single_index: false,
            ..Default::default()
        },
        |_path| unreachable!(),
    )
    .unwrap();

    let mut vertices: Vec<Vertex> = vec![];

    for (_i, m) in models.iter().enumerate() {
        let mesh = &m.mesh;

        for index in mesh.indices.iter() {
            vertices.push(Vertex {
                position: [mesh.positions[3 * *index as usize], mesh.positions[3 * *index as usize + 2]],
            });
        }
    }

    return vertices;
}
