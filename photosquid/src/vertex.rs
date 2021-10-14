#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
}

#[derive(Copy, Clone)]
pub struct VertexXYUV {
    pub position: [f32; 2],
    pub uvs: [f32; 2],
}

glium::implement_vertex!(Vertex, position);
glium::implement_vertex!(VertexXYUV, position, uvs);
