use crate::{
    aabb::AABB, capture::Capture, color::Color, icon_button::IconButton, matrix::reach_inside_mat4, press_animation::PressAnimation, render_ctx::RenderCtx,
    smooth::Smooth, text_helpers,
};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};
use nalgebra_glm as glm;
use std::{rc::Rc, time::Duration};

#[allow(dead_code)]
pub struct Checkbox {
    label: String,
    label_display: Option<TextDisplay<Rc<FontTexture>>>,
    checked: bool,
    color: Option<Smooth<Color>>,
    checkmark: Option<IconButton<()>>,
    has_new_content: bool,
}

#[allow(dead_code)]
impl Checkbox {
    pub fn new(default_label: String, checked: bool) -> Self {
        Self {
            label: default_label,
            label_display: None,
            checked,
            color: None,
            checkmark: None,
            has_new_content: false,
        }
    }

    pub fn click(&mut self, _button: MouseButton, position: &glm::Vec2, area: &AABB) -> Capture {
        if area.intersecting_point(position.x, position.y) {
            self.toggle();
            return Capture::TakeFocus;
        }
        Capture::Miss
    }

    pub fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, area: &AABB) {
        self.update_checkmark(ctx);
        self.render_label(ctx, text_system, font, area);
        self.render_box(ctx, area);
        self.render_check(ctx, area);
    }

    pub fn poll(&mut self) -> Option<bool> {
        if self.has_new_content {
            self.has_new_content = false;
            Some(self.checked)
        } else {
            None
        }
    }

    pub fn toggle(&mut self) {
        self.checked = !self.checked;
        self.has_new_content = true;
    }

    fn update_checkmark(&mut self, ctx: &mut RenderCtx) {
        if self.color.is_none() {
            self.color = Some(Smooth::new(ctx.color_scheme.light_ribbon, Some(Duration::from_millis(200))));
        }

        let checkmark = self
            .checkmark
            .get_or_insert_with(|| IconButton::new(include_str!("../_src_objs/check.obj"), PressAnimation::HalfCycle, (), ctx.display));

        let checked_color = Color::from_hex("#999999");
        let unchecked_color = ctx.color_scheme.light_ribbon;

        let color = self.color.as_mut().unwrap();
        let target_color = *color.get_real();

        if self.checked && target_color != checked_color {
            color.set(checked_color);
            checkmark.animate(true);
        } else if !self.checked && target_color != unchecked_color {
            color.set(unchecked_color);
            checkmark.animate(false);
        }
    }

    fn render_label(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, input_area: &AABB) {
        let input_area_center = glm::vec2(input_area.min_x + input_area.width() / 2.0, input_area.min_y + input_area.height() / 2.0);
        let relative_position = glm::vec2(0.0, -28.0);

        text_helpers::draw_text_centered(
            &mut self.label_display,
            text_system,
            font,
            &self.label,
            &(input_area_center + relative_position),
            ctx,
            Color::from_hex("#777777"),
        );
    }

    fn render_box(&self, ctx: &mut RenderCtx, area: &AABB) {
        let mesh = ctx.square_xyzuv;
        let identity = glm::identity::<f32, 4>();
        let quad_dimensions = glm::vec2(area.height() + 28.0 + 20.0, area.height() + 28.0);
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

    fn render_check(&mut self, ctx: &mut RenderCtx, aabb: &AABB) {
        let checkmark = self.checkmark.as_mut().unwrap();
        let color = self.color.as_ref().unwrap();
        checkmark.set_raw_position(aabb.center_x(), aabb.center_y() + 1.0);
        checkmark.render(ctx, &color.get_animated());
    }
}
