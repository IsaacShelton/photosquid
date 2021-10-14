use crate::{
    color::Color,
    matrix_helpers::reach_inside_mat4,
    obj_helpers,
    render_ctx::RenderCtx,
    vertex::{Vertex, VertexXYUV},
};
use glium::{Display, VertexBuffer};
use nalgebra_glm as glm;

pub struct MeshXyz {
    pub vertex_buffer: VertexBuffer<Vertex>,
    pub indices: glium::index::NoIndices,
}

impl MeshXyz {
    pub fn new(obj_src_code: &str, display: &Display) -> Self {
        let shape = obj_helpers::obj_data_to_shape(obj_src_code);
        Self::from_vertices(&shape, display)
    }

    pub fn from_vertices(vertices: &Vec<Vertex>, display: &Display) -> Self {
        let vertex_buffer = VertexBuffer::new(display, vertices).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        Self { vertex_buffer, indices }
    }

    pub fn new_ui_rect(display: &glium::Display) -> Self {
        let shape = vec![
            Vertex { position: [0.0, 0.0] },
            Vertex { position: [1.0, 0.0] },
            Vertex { position: [1.0, 1.0] },
            Vertex { position: [0.0, 0.0] },
            Vertex { position: [0.0, 1.0] },
            Vertex { position: [1.0, 1.0] },
        ];

        Self::from_vertices(&shape, display)
    }

    pub fn new_ui_ring(display: &glium::Display) -> Self {
        let shape = obj_helpers::obj_data_to_shape(include_str!("_src_objs/ring.obj"));
        Self::from_vertices(&shape, display)
    }

    pub fn new_shape_square(display: &Display) -> Self {
        Self::new(include_str!("_src_objs/shape/rect.obj"), &display)
    }

    pub fn render(&self, ctx: &mut RenderCtx, x: f32, y: f32, w_scale: f32, h_scale: f32, color: &Color) {
        let identity = glm::identity::<f32, 4>();
        let transformation = glm::translation(&glm::vec3(x, y, 0.0));
        let transformation = glm::scale(&transformation, &glm::vec3(w_scale, h_scale, 0.0));

        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: reach_inside_mat4(&identity),
            projection: reach_inside_mat4(ctx.projection),
            color: Into::<[f32; 4]>::into(color)
        };

        ctx.draw(&self.vertex_buffer, &self.indices, ctx.color_shader, &uniforms, &Default::default())
            .unwrap();
    }
}

pub struct MeshXyzUv {
    pub vertex_buffer: VertexBuffer<VertexXYUV>,
    pub indices: glium::index::NoIndices,
}

impl MeshXyzUv {
    pub fn new_square(display: &Display) -> Self {
        let shape = [
            VertexXYUV {
                position: [-1.0, -1.0],
                uvs: [0.0, 0.0],
            },
            VertexXYUV {
                position: [-1.0, 1.0],
                uvs: [0.0, 1.0],
            },
            VertexXYUV {
                position: [1.0, 1.0],
                uvs: [1.0, 1.0],
            },
            VertexXYUV {
                position: [-1.0, -1.0],
                uvs: [0.0, 0.0],
            },
            VertexXYUV {
                position: [1.0, 1.0],
                uvs: [1.0, 1.0],
            },
            VertexXYUV {
                position: [1.0, -1.0],
                uvs: [1.0, 0.0],
            },
        ];
        let vertex_buffer = VertexBuffer::new(display, &shape).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        Self { vertex_buffer, indices }
    }
}
