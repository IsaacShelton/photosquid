use crate::{aabb::AABB, capture::Capture, color::Color, interaction::Interaction, matrix_helpers::reach_inside_mat4, render_ctx::RenderCtx, smooth::Smooth};
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;
use std::time::Duration;

pub struct ColorPicker {
    is_selecting_hue_value: bool,
    is_selecting_saturation: bool,
    hue_value_point: Smooth<glm::Vec2>,
    saturation_point: Smooth<f32>,
    color_changed_to: Option<Color>,
    y: f32,
}

impl Default for ColorPicker {
    fn default() -> Self {
        Self {
            is_selecting_hue_value: false,
            is_selecting_saturation: false,
            hue_value_point: Smooth::new(glm::vec2(0.0, 0.0), Duration::from_millis(200)),
            saturation_point: Smooth::new(1.0, Duration::from_millis(200)),
            color_changed_to: None,
            y: 64.0,
        }
    }
}

impl ColorPicker {
    // Sets the selected color in the color picker without triggering a color change notification
    pub fn set_selected_color_no_notif(&mut self, color: Color) {
        let (h, s, v) = color.to_hsv();
        let v = 1.0 - v;

        self.hue_value_point.set(glm::vec2(h, v));
        self.saturation_point.set(s);
    }

    pub fn poll(&mut self) -> Option<Color> {
        self.color_changed_to.take()
    }

    pub fn click(&mut self, button: MouseButton, mouse: &glm::Vec2, screen_width: f32) -> bool {
        if button == MouseButton::Left && self.is_over_hue_value(mouse, screen_width) {
            self.is_selecting_hue_value = true;
            self.set_hue_value_with_mouse(mouse, screen_width);
            return true;
        }

        if button == MouseButton::Left && self.is_over_saturation(mouse, screen_width) {
            self.is_selecting_saturation = true;
            self.set_saturation_with_mouse(mouse, screen_width);
            return true;
        }

        false
    }

    pub fn drag(&mut self, interaction: &Interaction, screen_width: f32) -> Capture {
        if let Interaction::Drag { current, .. } = interaction {
            if self.is_selecting_hue_value {
                self.set_hue_value_with_mouse(current, screen_width);
            } else if self.is_selecting_saturation {
                self.set_saturation_with_mouse(current, screen_width);
            }
            return Capture::AllowDrag;
        }
        Capture::Miss
    }

    pub fn mouse_release(&mut self, button: MouseButton) {
        if button == MouseButton::Left {
            self.is_selecting_hue_value = false;
            self.is_selecting_saturation = false;
        }
    }

    pub fn is_over_hue_value(&self, mouse: &glm::Vec2, screen_width: f32) -> bool {
        if let Some(area) = self.get_hue_value_area(screen_width) {
            return area.intersecting_point(mouse.x, mouse.y);
        }

        false
    }

    pub fn is_over_saturation(&self, mouse: &glm::Vec2, screen_width: f32) -> bool {
        if let Some(area) = self.get_saturation_area(screen_width) {
            return area.intersecting_point(mouse.x, mouse.y);
        }

        false
    }

    pub fn get_hue_value_area(&self, screen_width: f32) -> Option<AABB> {
        Some(AABB::new(screen_width - 256.0, self.y, 256.0, 192.0))
    }

    pub fn get_hue_value_point(&self) -> &Smooth<glm::Vec2> {
        &self.hue_value_point
    }

    pub fn get_saturation_area(&self, screen_width: f32) -> Option<AABB> {
        Some(AABB::new(screen_width - 256.0, self.y + 196.0, 256.0, 24.0))
    }

    pub fn get_saturation_point(&self) -> &Smooth<f32> {
        &self.saturation_point
    }

    pub fn set_hue_value_with_mouse(&mut self, mouse: &glm::Vec2, screen_width: f32) {
        if let Some(area) = self.get_hue_value_area(screen_width) {
            let u = (mouse.x - area.min_x) / area.width();
            let v = (mouse.y - area.min_y) / area.height();
            let u = u.clamp(0.0, 1.0);
            let v = v.clamp(0.0, 1.0);
            self.hue_value_point.set(glm::vec2(u, v));
            self.color_changed_to = Some(self.calculate_color());
        }
    }

    pub fn set_saturation_with_mouse(&mut self, mouse: &glm::Vec2, screen_width: f32) {
        if let Some(area) = self.get_saturation_area(screen_width) {
            let u = (mouse.x - area.min_x) / area.width();
            self.saturation_point.set(u.clamp(0.0, 1.0));
            self.color_changed_to = Some(self.calculate_color());
        }
    }

    pub fn calculate_color(&self) -> Color {
        let real_hv = self.get_hue_value_point().get_real();
        let real_s = self.get_saturation_point().get_real();
        let h = real_hv.x;
        let s = *real_s;
        let v = 1.0 - real_hv.y;
        Color::from_hsv(h, s, v)
    }

    pub fn is_selecting_color(&self) -> bool {
        self.is_selecting_hue_value || self.is_selecting_saturation
    }

    pub fn render(&self, ctx: &mut RenderCtx) {
        self.render_hue_value_picker(ctx);
        self.render_saturation_picker(ctx);
    }

    pub fn render_hue_value_picker(&self, ctx: &mut RenderCtx) {
        let color_picker_mesh = ctx.square_xyzuv;

        let area = self.get_hue_value_area(ctx.width).unwrap();
        let x = area.min_x;
        let y = area.min_y;
        let dimensions = glm::vec2(area.width(), area.height());
        let hue_value_point_animated = self.hue_value_point.get_animated();

        let identity = glm::identity::<f32, 4>();
        let transformation = glm::translation(&glm::vec3(dimensions.x / 2.0 + x, dimensions.y / 2.0 + y, 0.0));
        let transformation = glm::scale(&transformation, &glm::vec3(dimensions.x / 2.0, dimensions.y / 2.0, 0.0));
        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: reach_inside_mat4(&identity),
            projection: reach_inside_mat4(ctx.projection),
            saturation: self.saturation_point.get_animated(),
            point: [hue_value_point_animated.x, hue_value_point_animated.y],
            dimensions: [dimensions.x, dimensions.y],
        };

        ctx.draw(
            &color_picker_mesh.vertex_buffer,
            &color_picker_mesh.indices,
            ctx.hue_value_picker_shader,
            &uniforms,
            &Default::default(),
        )
        .unwrap();
    }

    pub fn render_saturation_picker(&self, ctx: &mut RenderCtx) {
        let color_picker_mesh = ctx.square_xyzuv;

        let area = self.get_saturation_area(ctx.width).unwrap();
        let x = area.min_x;
        let y = area.min_y;
        let dimensions = glm::vec2(area.width(), area.height());

        let hue_value_point = self.get_hue_value_point().get_animated();
        let hue = hue_value_point.x;
        let value = 1.0 - hue_value_point.y;

        let identity = glm::identity::<f32, 4>();
        let transformation = glm::translation(&glm::vec3(dimensions.x / 2.0 + x, dimensions.y / 2.0 + y, 0.0));
        let transformation = glm::scale(&transformation, &glm::vec3(dimensions.x / 2.0, dimensions.y / 2.0, 0.0));
        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: reach_inside_mat4(&identity),
            projection: reach_inside_mat4(ctx.projection),
            hue: hue,
            value: value,
            point: [self.get_saturation_point().get_animated(), 0.5],
            dimensions: [dimensions.x, dimensions.y],
        };

        ctx.draw(
            &color_picker_mesh.vertex_buffer,
            &color_picker_mesh.indices,
            ctx.saturation_picker_shader,
            &uniforms,
            &Default::default(),
        )
        .unwrap();
    }
}
