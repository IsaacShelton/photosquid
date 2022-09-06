use std::rc::Rc;

use crate::{app::App, as_values::AsValues, color::Color, draw_text::draw_text_centered};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};
use nalgebra_glm as glm;

use crate::{aabb::AABB, capture::Capture, render_ctx::RenderCtx};

pub struct Button {
    text: String,
    text_display: Option<TextDisplay<Rc<FontTexture>>>,
    action: Box<dyn FnMut(&mut App)>,
}

impl Button {
    pub fn new(text: String, action: Box<dyn FnMut(&mut App)>) -> Self {
        Self {
            text,
            text_display: None,
            action,
        }
    }

    pub fn click(&mut self, _mouse_button: MouseButton, position: &glm::Vec2, area: &AABB, app: &mut App) -> Capture {
        if area.intersecting_point(position.x, position.y) {
            (self.action)(app);
            return Capture::TakeFocus;
        }

        Capture::Miss
    }

    pub fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, area: &AABB) {
        self.render_box(ctx, area);
        self.render_text(ctx, text_system, font, area);
    }

    fn render_box(&self, ctx: &mut RenderCtx, area: &AABB) {
        let mesh = ctx.square_xyzuv;
        let identity = glm::identity::<f32, 4>();

        let quad_dimensions = glm::vec2(area.width(), area.height() + 32.0);
        let dead_space = quad_dimensions - glm::vec2(area.width(), area.height());
        let min = glm::vec2(area.min_x, area.min_y);

        let transformation = glm::translation(&glm::vec2_to_vec3(&(min + quad_dimensions * 0.5 - dead_space * 0.5)));
        let transformation = glm::scale(&transformation, &glm::vec2_to_vec3(&(quad_dimensions * 0.5)));

        let uniforms = glium::uniform! {
            transformation: transformation.as_values(),
            view: identity.as_values(),
            projection: ctx.projection.as_values(),
            rectangle_color: ctx.color_scheme.dark_foreground.as_values(),
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

        draw_text_centered(
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
