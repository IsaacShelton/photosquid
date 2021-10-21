use crate::{
    aabb::AABB,
    matrix_helpers::reach_inside_mat4,
    render_ctx::RenderCtx,
    text_helpers,
    tool::{Capture, KeyCapture},
};
use glium::glutin::event::{MouseButton, VirtualKeyCode};
use glium_text_rusttype::TextSystem;
use glium_text_rusttype::{FontTexture, TextDisplay};
use nalgebra_glm as glm;
use std::{collections::HashMap, rc::Rc};

pub struct TextInput {
    text: String,
    label: String,
    text_display: Option<TextDisplay<Rc<FontTexture>>>,
    label_display: Option<TextDisplay<Rc<FontTexture>>>,
    pre_edit: String,
    has_new_content: bool,
    focused: bool,
    just_focused: bool,
    input_error: bool,
    suffix: String,
    default_text: String,
}

impl TextInput {
    pub fn new(default_text: String, default_label: String, suffix: String) -> Self {
        Self {
            pre_edit: default_text.clone(),
            default_text: default_text.clone(),
            text: default_text,
            label: default_label,
            text_display: None,
            label_display: None,
            has_new_content: false,
            focused: false,
            just_focused: false,
            input_error: false,
            suffix,
        }
    }

    pub fn standard_area(position: &glm::Vec2) -> AABB {
        let width = 176.0;
        let height = (16.0 * 0.8) + 30.0;
        let height = height * 0.8;
        AABB::new(position.x, position.y, width, height)
    }

    pub fn click(&mut self, _button: MouseButton, position: &glm::Vec2, area: &AABB) -> Capture {
        let was_focused = self.focused;
        self.focused = area.intersecting_point(position.x, position.y);
        self.just_focused = self.focused && !was_focused;
        self.input_error = false;

        if self.focused {
            if self.just_focused {
                self.pre_edit = self.text.clone();
            }
            Capture::NoDrag
        } else {
            self.ensure_not_empty();

            if was_focused && self.text != self.pre_edit {
                self.has_new_content = true;
            }

            Capture::Miss
        }
    }

    pub fn key_press(&mut self, virtual_keycode: VirtualKeyCode, mappings: &HashMap<VirtualKeyCode, char>, shift: bool) -> KeyCapture {
        if !self.focused {
            return KeyCapture::Miss;
        }

        if virtual_keycode == VirtualKeyCode::Back {
            if shift {
                self.clear();
                self.input_error = false;
            } else {
                self.backspace();
                self.input_error = false;
            }
            return KeyCapture::Capture;
        }

        if virtual_keycode == VirtualKeyCode::Escape {
            self.focused = false;
            self.text = self.pre_edit.clone();
            self.text_display = None;
            return KeyCapture::Capture;
        }

        if virtual_keycode == VirtualKeyCode::Return {
            self.focused = false;
            self.has_new_content = true;
            self.ensure_not_empty();
            return KeyCapture::Capture;
        }

        if let Some(character) = Self::to_character(virtual_keycode, mappings) {
            self.type_character(character);
            self.input_error = false;
            return KeyCapture::Capture;
        } else if virtual_keycode != VirtualKeyCode::LShift {
            self.input_error = true;
        }

        KeyCapture::Miss
    }

    fn type_character(&mut self, character: char) {
        self.text.push(character);
        self.text_display = None;
    }

    fn backspace(&mut self) {
        if self.text.len() > 0 {
            self.text.pop();
            self.text_display = None;
        }
    }

    fn clear(&mut self) {
        self.text.clear();
        self.text_display = None;
    }

    pub fn poll<'a>(&'a mut self) -> Option<&'a str> {
        if self.has_new_content {
            self.has_new_content = false;
            Some(&self.text)
        } else {
            None
        }
    }

    pub fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, area: &AABB) {
        self.render_background(ctx, area);
        self.render_text(ctx, text_system, font.clone(), area);
        self.render_label(ctx, text_system, font, area);
    }

    fn render_background(&self, ctx: &mut RenderCtx, area: &AABB) {
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
            rectangle_color: Into::<[f32; 4]>::into(ctx.color_scheme.light_ribbon),
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

        let color: (f32, f32, f32, f32) = if self.focused {
            if self.input_error {
                ctx.color_scheme.error.into()
            } else {
                ctx.color_scheme.foreground.into()
            }
        } else {
            (0.5, 0.5, 0.5, 1.0)
        };

        Self::render_text_display_centered(
            ctx,
            &mut self.text_display,
            &format!("{}{}", &self.text, &self.suffix),
            text_system,
            font,
            &input_area_center,
            &glm::vec2(0.0, 4.0),
            color,
        );
    }

    fn render_label(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, input_area: &AABB) {
        let input_area_center = glm::vec2(input_area.min_x + input_area.width() / 2.0, input_area.min_y + input_area.height() / 2.0);

        Self::render_text_display_centered(
            ctx,
            &mut self.label_display,
            &self.label,
            text_system,
            font,
            &input_area_center,
            &glm::vec2(0.0, -28.0),
            (0.5, 0.5, 0.5, 1.0),
        );
        return;
    }

    fn render_text_display_centered(
        ctx: &mut RenderCtx,
        text_display: &mut Option<TextDisplay<Rc<FontTexture>>>,
        content: &str,
        text_system: &TextSystem,
        font: Rc<FontTexture>,
        input_area_center: &glm::Vec2,
        relative_position: &glm::Vec2,
        color_tuple: (f32, f32, f32, f32),
    ) {
        text_helpers::get_or_make_display(text_display, text_system, font, content);

        let text_display = text_display.as_ref().unwrap();
        let transformation = glm::translation(&glm::vec3(
            input_area_center.x + relative_position.x - 0.5 * text_display.get_width() * 16.0,
            input_area_center.y + relative_position.y,
            0.0,
        ));
        let transformation = glm::scale(&transformation, &glm::vec3(16.0, -16.0, 0.0));
        let matrix = ctx.projection * transformation;
        ctx.draw_text(&text_display, text_system, matrix, color_tuple).unwrap();
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn set(&mut self, content: &str) {
        self.clear();

        for character in content.chars() {
            self.type_character(character);
        }
    }

    fn to_character(virtual_keycode: VirtualKeyCode, mappings: &HashMap<VirtualKeyCode, char>) -> Option<char> {
        match mappings.get(&virtual_keycode) {
            Some(c) => Some(*c),
            None => None,
        }
    }

    fn ensure_not_empty(&mut self) {
        if self.text.is_empty() {
            self.text = self.default_text.clone();
            self.text_display = None;
        }
    }
}
