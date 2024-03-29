use crate::{
    aabb::AABB,
    as_values::AsValues,
    capture::{Capture, KeyCapture},
    color::Color,
    draw_text::draw_text_centered,
    render_ctx::RenderCtx,
};
use glium::glutin::event::{MouseButton, VirtualKeyCode};
use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

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

    pub fn click(&mut self, _button: MouseButton, position: &glm::Vec2, area: &AABB) -> Capture {
        let was_focused = self.focused;
        self.focused = area.intersecting_point(position.x, position.y);
        self.just_focused = self.focused && !was_focused;
        self.input_error = false;

        if self.focused {
            if self.just_focused {
                self.pre_edit = self.text.clone();
            }
            Capture::TakeFocus
        } else {
            self.ensure_not_empty();

            if was_focused && self.text != self.pre_edit {
                self.has_new_content = true;
            }

            Capture::Miss
        }
    }

    pub fn key_press(&mut self, virtual_keycode: VirtualKeyCode, shift: bool) -> KeyCapture {
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
            self.unfocus();
            return KeyCapture::Capture;
        }

        if let Some(character) = Self::numeric_map(virtual_keycode) {
            self.type_character(character);
            self.input_error = false;
            return KeyCapture::Capture;
        } else if virtual_keycode != VirtualKeyCode::LShift {
            self.input_error = true;
        }

        KeyCapture::Miss
    }

    pub fn unfocus(&mut self) {
        if self.focused {
            self.focused = false;
            self.has_new_content = true;
            self.ensure_not_empty();
        }
    }

    pub fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, area: &AABB) {
        self.render_background(ctx, area);
        self.render_text(ctx, text_system, font.clone(), area);
        self.render_label(ctx, text_system, font, area);
    }

    pub fn standard_area(position: &glm::Vec2) -> AABB {
        let width = 176.0;
        let height = (16.0 * 0.8) + 30.0;
        let height = height * 0.8;
        AABB::new(position.x, position.y, width, height)
    }

    fn type_character(&mut self, character: char) {
        self.text.push(character);
        self.text_display = None;
    }

    fn backspace(&mut self) {
        if !self.text.is_empty() {
            self.text.pop();
            self.text_display = None;
        }
    }

    fn clear(&mut self) {
        self.text.clear();
        self.text_display = None;
    }

    pub fn poll(&mut self) -> Option<&str> {
        if self.has_new_content {
            self.has_new_content = false;
            Some(&self.text)
        } else {
            None
        }
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
            transformation: transformation.as_values(),
            view: identity.as_values(),
            projection: ctx.projection.as_values(),
            rectangle_color: ctx.color_scheme.light_ribbon.as_values(),
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

        let color = if self.focused {
            if self.input_error {
                ctx.color_scheme.error
            } else {
                ctx.color_scheme.foreground
            }
        } else {
            Color::from_hex("#777777")
        };

        draw_text_centered(
            &mut self.text_display,
            text_system,
            font,
            &format!("{}{}", &self.text, &self.suffix),
            &(input_area_center + relative_position),
            ctx,
            color,
        );
    }

    fn render_label(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, input_area: &AABB) {
        let input_area_center = glm::vec2(input_area.min_x + input_area.width() / 2.0, input_area.min_y + input_area.height() / 2.0);
        let relative_position = glm::vec2(0.0, -28.0);

        draw_text_centered(
            &mut self.label_display,
            text_system,
            font,
            &self.label,
            &(input_area_center + relative_position),
            ctx,
            Color::from_hex("#777777"),
        );
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

    fn ensure_not_empty(&mut self) {
        if self.text.is_empty() {
            self.text = self.default_text.clone();
            self.text_display = None;
        }
    }

    pub fn numeric_map(virtual_keycode: VirtualKeyCode) -> Option<char> {
        match virtual_keycode {
            VirtualKeyCode::Key0 => Some('0'),
            VirtualKeyCode::Key1 => Some('1'),
            VirtualKeyCode::Key2 => Some('2'),
            VirtualKeyCode::Key3 => Some('3'),
            VirtualKeyCode::Key4 => Some('4'),
            VirtualKeyCode::Key5 => Some('5'),
            VirtualKeyCode::Key6 => Some('6'),
            VirtualKeyCode::Key7 => Some('7'),
            VirtualKeyCode::Key8 => Some('8'),
            VirtualKeyCode::Key9 => Some('9'),
            VirtualKeyCode::Period => Some('.'),
            VirtualKeyCode::Minus => Some('-'),
            _ => None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}
