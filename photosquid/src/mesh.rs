use crate::{
    as_values::AsValues,
    color::Color,
    data::rect::BorderRadii,
    obj,
    render_ctx::RenderCtx,
    vertex::{Vertex, VertexXYUV},
};
use glium::{index::PrimitiveType, Display, VertexBuffer};
use itertools::Itertools;
use lyon::geom::Box2D;
use nalgebra_glm as glm;

pub enum MeshIndices {
    None(glium::index::NoIndices),
    TrianglesU16(Box<glium::IndexBuffer<u16>>),
}

impl<'a> From<&'a MeshIndices> for glium::index::IndicesSource<'a> {
    fn from(indices: &'a MeshIndices) -> Self {
        match indices {
            MeshIndices::None(indices) => indices.into(),
            MeshIndices::TrianglesU16(indices) => indices.as_ref().into(),
        }
    }
}

pub struct MeshXyz {
    pub vertex_buffer: VertexBuffer<Vertex>,
    pub indices: MeshIndices,
}

impl MeshXyz {
    pub fn new(obj_src_code: &str, display: &Display) -> Self {
        let shape = obj::data_to_vertices(obj_src_code);
        Self::from_vertices(&shape, display)
    }

    pub fn from_vertices(vertices: &[Vertex], display: &Display) -> Self {
        let vertex_buffer = VertexBuffer::new(display, vertices).unwrap();
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

        Self {
            vertex_buffer,
            indices: MeshIndices::None(indices),
        }
    }

    pub fn from_vertices_and_indices(vertices: &[Vertex], indices: &[u16], display: &Display) -> Self {
        let vertex_buffer = VertexBuffer::new(display, vertices).unwrap();
        let indices = glium::IndexBuffer::new(display, PrimitiveType::TrianglesList, indices).unwrap();

        Self {
            vertex_buffer,
            indices: MeshIndices::TrianglesU16(Box::new(indices)),
        }
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
        let shape = obj::data_to_vertices(include_str!("_src_objs/ring.obj"));
        Self::from_vertices(&shape, display)
    }

    pub fn new_ui_check(display: &glium::Display) -> Self {
        let shape = obj::data_to_vertices(include_str!("_src_objs/check.obj"));
        Self::from_vertices(&shape, display)
    }

    pub fn new_shape_square(display: &Display) -> Self {
        Self::new(include_str!("_src_objs/shape/rect.obj"), display)
    }

    pub fn new_shape_triangle(display: &glium::Display, p: [glm::Vec2; 3]) -> Self {
        // We disregard normals and don't do back-face culling, so this is okay

        let shape = p.iter().map(|point| Vertex { position: (*point).into() }).collect_vec();

        Self::from_vertices(&shape, display)
    }

    pub fn new_shape_circle(display: &Display) -> Self {
        Self::new(include_str!("_src_objs/shape/circle.obj"), display)
    }

    pub fn new_rect(display: &Display, size: glm::Vec2, radii: BorderRadii) -> Self {
        use lyon::{
            path::{math::point, Winding},
            tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers},
        };

        let width = size.x.abs();
        let height = size.y.abs();

        let mut builder = lyon::path::Path::builder();
        builder.add_rounded_rectangle(
            &Box2D::new(point(-width / 2.0, -height / 2.0), point(width / 2.0, height / 2.0)),
            &radii.into(),
            Winding::Positive,
        );
        let lyon_path = builder.build();

        // Will contain the result of the tessellation.
        let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();

        // Create tessellated geometry for fill
        tessellator
            .tessellate_path(
                &lyon_path,
                &FillOptions::default(),
                &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| Vertex {
                    position: vertex.position().to_array(),
                }),
            )
            .unwrap();

        Self::from_vertices_and_indices(&geometry.vertices, &geometry.indices, display)
    }

    pub fn render(&self, ctx: &mut RenderCtx, position: glm::Vec2, scale: glm::Vec2, color: &Color) {
        let identity = glm::identity::<f32, 4>();
        let transformation = glm::translation(&glm::vec2_to_vec3(&position));
        let transformation = glm::scale(&transformation, &glm::vec2_to_vec3(&scale));

        let uniforms = glium::uniform! {
            transformation: transformation.as_values(),
            view: identity.as_values(),
            projection: ctx.projection.as_values(),
            color: color.as_values()
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
