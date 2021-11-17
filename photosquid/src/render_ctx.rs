use crate::{
    color::Color,
    color_impls::Clearable,
    color_scheme::ColorScheme,
    mesh::{MeshXyz, MeshXyzUv},
};
use glium::{framebuffer::SimpleFrameBuffer, Display, Frame};
use glium_text_rusttype::{self as glium_text, FontTexture, TextDisplay, TextSystem};
use nalgebra_glm as glm;

pub struct RenderCtx<'a, 'f> {
    pub target: &'f mut Frame,
    pub framebuffer: &'f mut SimpleFrameBuffer<'f>,
    pub color_shader: &'a glium::Program,
    pub hue_value_picker_shader: &'a glium::Program,
    pub saturation_picker_shader: &'a glium::Program,
    pub rounded_rectangle_shader: &'a glium::Program,
    pub projection: &'a glm::Mat4,
    pub view: &'a glm::Mat4,
    pub width: f32,
    pub height: f32,
    pub scale_factor: f64,
    pub ribbon_mesh: &'a MeshXyz,
    pub ring_mesh: &'a MeshXyz,
    pub check_mesh: &'a MeshXyz,
    pub square_xyzuv: &'a MeshXyzUv,
    pub color_scheme: &'a ColorScheme,
    pub camera: &'a glm::Vec2,
    pub display: &'a Display,
}

impl RenderCtx<'_, '_> {
    pub fn clear_color(&mut self, color: &Color) {
        if self.scale_factor > 1.0 {
            // Non-MSAA
            color.clear_framebuffer_with(self.framebuffer);
        } else {
            // MSAA
            color.clear_with(self.target);
        }
    }

    pub fn draw<'a, 'b, V, I, U>(
        &mut self,
        vertex_buffer: V,
        index_buffer: I,
        program: &glium::Program,
        uniforms: &U,
        draw_parameters: &glium::DrawParameters<'_>,
    ) -> Result<(), glium::DrawError>
    where
        I: Into<glium::index::IndicesSource<'a>>,
        U: glium::uniforms::Uniforms,
        V: glium::vertex::MultiVerticesSource<'b>,
    {
        use glium::Surface;

        if self.scale_factor > 1.0 {
            // Non-MSAA
            self.framebuffer.draw(vertex_buffer, index_buffer, program, uniforms, draw_parameters)
        } else {
            // MSAA
            self.target.draw(vertex_buffer, index_buffer, program, uniforms, draw_parameters)
        }
    }

    pub fn draw_text<F, M>(&mut self, text: &TextDisplay<F>, text_system: &TextSystem, matrix: M, color: (f32, f32, f32, f32)) -> Result<(), glium::DrawError>
    where
        M: Into<[[f32; 4]; 4]>,
        F: std::ops::Deref<Target = FontTexture>,
    {
        if self.scale_factor > 1.0 {
            // Non-MSAA
            glium_text::draw(text, text_system, self.framebuffer, matrix, color)
        } else {
            // MSAA
            glium_text::draw(text, text_system, self.target, matrix, color)
        }
    }
}
