use crate::{aabb::AABB, color::Color, matrix_helpers::reach_inside_mat4, render_ctx::RenderCtx, text_helpers};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct ContextMenu {
    position: glm::Vec2,
    options: Vec<ContextMenuOption>,
    background_color: Color,
}

pub struct ContextMenuOption {
    friendly_name: String,
    friendly_shortcut: String,
    action: ContextAction,
    text_display: Option<TextDisplay<Rc<FontTexture>>>,
    shortcut_display: Option<TextDisplay<Rc<FontTexture>>>,
}

#[derive(Copy, Clone)]
pub enum ContextAction {
    DeleteSelected,
    DuplicateSelected,
    GrabSelected,
}

impl ContextMenu {
    pub fn new(position: glm::Vec2, options: Vec<ContextMenuOption>, background_color: Color) -> Self {
        Self {
            position,
            options,
            background_color,
        }
    }

    pub fn click(&self, button: MouseButton, position: &glm::Vec2) -> Option<ContextAction> {
        let area = self.get_area();

        if button == MouseButton::Left && area.intersecting_point(position.x, position.y) {
            let y_offset = 8.0f32 * 0.8;
            let height_per_entry = 30.0f32;
            let option_index = ((position.y - self.position.y + y_offset) / height_per_entry) as usize;
            let option_index = option_index.clamp(0, self.options.len() - 1);
            Some(self.options[option_index].action)
        } else {
            None
        }
    }

    pub fn get_area(&self) -> AABB {
        let width = 192.0;
        let height = (16.0 * 0.8) + 30.0 * self.options.len() as f32;
        AABB::new(self.position.x, self.position.y - 12.0, width, height)
    }

    pub fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        let area = self.get_area();

        // Render context menu background
        {
            let mesh = ctx.square_xyzuv;
            let identity = glm::identity::<f32, 4>();
            let quad_dimensions = glm::vec2(area.width() + 32.0, area.height() + 32.0);
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
                rectangle_color: Into::<[f32; 4]>::into(self.background_color),
                dimensions: [quad_dimensions.x, quad_dimensions.y],
                height_scale: 1.0f32,
                do_shadow: 1
            };

            let draw_parameters = glium::DrawParameters {
                blend: glium::draw_parameters::Blend::alpha_blending(),
                ..Default::default()
            };

            ctx.draw(&mesh.vertex_buffer, &mesh.indices, ctx.rounded_rectangle_shader, &uniforms, &draw_parameters)
                .unwrap();
        }

        for (i, option) in self.options.iter_mut().enumerate() {
            // Draw friendly name
            let text_display = option.get_text_display(text_system, font.clone());
            let transformation = glm::translation(&glm::vec2_to_vec3(&self.position));
            let transformation = glm::translate(&transformation, &glm::vec3(16.0, (16.0 * 0.8) + 30.0 * i as f32, 0.0));
            let transformation = glm::scale(&transformation, &glm::vec3(16.0, -16.0, 0.0));
            let matrix = ctx.projection * transformation;
            ctx.draw_text(&text_display, text_system, matrix, (1.0, 1.0, 1.0, 1.0)).unwrap();

            // Draw friendly shortcut
            let text_display = option.get_shortcut_display(text_system, font.clone());
            let transformation = glm::translation(&glm::vec2_to_vec3(&self.position));
            let transformation = glm::translate(
                &transformation,
                &glm::vec3(area.width() - 14.0 - text_display.get_width() * 16.0, (16.0 * 0.8) + 30.0 * i as f32, 0.0),
            );
            let transformation = glm::scale(&transformation, &glm::vec3(16.0, -16.0, 0.0));
            let matrix = ctx.projection * transformation;
            ctx.draw_text(&text_display, text_system, matrix, (0.5, 0.5, 0.5, 1.0)).unwrap();
        }
    }
}

impl ContextMenuOption {
    pub fn new(friendly_name: String, friendly_shortcut: String, action: ContextAction) -> Self {
        Self {
            friendly_name,
            friendly_shortcut,
            action,
            text_display: None,
            shortcut_display: None,
        }
    }

    pub fn get_text_display(&mut self, text_system: &TextSystem, font: Rc<FontTexture>) -> &TextDisplay<Rc<FontTexture>> {
        text_helpers::get_or_make_display(&mut self.text_display, text_system, font, &self.friendly_name)
    }

    pub fn get_shortcut_display(&mut self, text_system: &TextSystem, font: Rc<FontTexture>) -> &TextDisplay<Rc<FontTexture>> {
        text_helpers::get_or_make_display(&mut self.shortcut_display, text_system, font, &self.friendly_shortcut)
    }
}
