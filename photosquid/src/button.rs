use std::rc::Rc;

use crate::{color::Color, matrix_helpers::reach_inside_mat4, text_helpers};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};
use nalgebra_glm as glm;

use crate::{aabb::AABB, capture::Capture, render_ctx::RenderCtx};

pub struct Button {
    text: String,
    text_display: Option<TextDisplay<Rc<FontTexture>>>,
}

impl Button {
    pub fn new(text: String) -> Self {
        Self { text, text_display: None }
    }

    pub fn click(&mut self, _mouse_button: MouseButton, position: &glm::Vec2, area: &AABB) -> Capture {
        if area.intersecting_point(position.x, position.y) {
            return Capture::TakeFocus;
        }

        Capture::Miss
    }

    pub fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, area: &AABB) {
        self.render_box(ctx, area);
        self.render_text(ctx, text_system, font.clone(), area);
    }

    fn render_box(&self, ctx: &mut RenderCtx, area: &AABB) {
        let mesh = ctx.square_xyzuv;
        let identity = glm::identity::<f32, 4>();
        let quad_dimensions = glm::vec2(area.width(), area.height() + 32.0);
        let dead_space = quad_dimensions - glm::vec2(area.width(), area.height());
        let transformation = glm::translation(&glm::vec3(
            area.min_x + quad_dimensions.x * 0.5 - dead_space.x * 0.5,
            area.min_y + quad_dimensions.y * 0.5 - dead_space.y * 0.5,
            0.0,
        ));
        let transformation = glm::scale(&transformation, &glm::vec3(quad_dimensions.x * 0.5, quad_dimensions.y * 0.5, 0.0));

        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: reach_inside_mat4(&identity),
            projection: reach_inside_mat4(ctx.projection),
            rectangle_color: Into::<[f32; 4]>::into(ctx.color_scheme.dark_foreground),
            dimensions: [quad_dimensions.x, quad_dimensions.y],
            height_scale: 1.0f32,
            do_shadow: 0
        };

        let draw_parameters = glium::DrawParameters {
            blend: glium::draw_parameters::Blend::alpha_blending(),
            ..Default::default()
        };

        ctx.draw(&mesh.vertex_buffer, &mesh.indices, ctx.rounded_rectangle_shader, &uniforms, &draw_parameters)
            .unwrap();
    }

    fn render_text(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, input_area: &AABB) {
        let input_area_center = glm::vec2(input_area.min_x + input_area.width() / 2.0, input_area.min_y + input_area.height() / 2.0);
        let relative_position = glm::vec2(0.0, 4.0);

        let color = Color::from_hex("#FFFFFF");

        text_helpers::draw_text_centered(
            &mut self.text_display,
            text_system,
            font,
            &self.text,
            &(input_area_center + relative_position),
            ctx,
            color,
        );
    }
}
