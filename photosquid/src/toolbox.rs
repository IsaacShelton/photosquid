use crate::tool::ToolKey;
use crate::{
    color::Color,
    color_picker::ColorPicker,
    matrix_helpers::reach_inside_mat4,
    mesh::MeshXyz,
    render_ctx::RenderCtx,
    tool::{Capture, Interaction, Tool},
    tool_button::ToolButton,
    ColorScheme,
};
use glium::glutin::event::MouseButton;
use glium::Display;
use glium_text_rusttype::{FontTexture, TextSystem};
use interpolation::{Ease, Lerp};
use nalgebra_glm as glm;
use slotmap::SlotMap;
use std::{
    rc::Rc,
    time::{Duration, Instant},
};

pub struct ToolBox {
    buttons: Vec<ToolButton>,
    icon_size: f32,
    padding: f32,
    width: f32,
    full_width: f32,
    selection: SelectionIndicator,

    pub color_picker: ColorPicker,
}

impl ToolBox {
    pub fn new(display: &Display) -> Self {
        ToolBox {
            buttons: vec![],
            icon_size: 48.0,
            padding: 16.0,
            width: 48.0,
            full_width: 256.0,
            selection: SelectionIndicator::new(0.0, display),
            color_picker: ColorPicker::new(),
        }
    }

    pub fn add(&mut self, button: ToolButton) {
        self.buttons.push(button);
    }

    pub fn select(&mut self, index: usize) {
        if index < self.buttons.len() {
            for button in self.buttons.iter_mut() {
                button.animate(false);
            }

            self.buttons[index].animate(true);
            self.selection.select(index);
        }
    }

    pub fn click(&mut self, button: MouseButton, mouse: &glm::Vec2, screen_width: f32, screen_height: f32) -> bool {
        // Tool ribbon
        if button == MouseButton::Left && mouse.x < self.width {
            let index = self.get_index_for_mouse_y(mouse.y, screen_height);

            if let Some(index) = index {
                self.select(index);
            }

            return true;
        }

        // Squid options ribbon
        if button == MouseButton::Left && mouse.x > screen_width - 256.0 {
            let _ = self.color_picker.click(button, mouse, screen_width);
            return true;
        }

        false
    }

    pub fn mouse_release(&mut self, button: MouseButton) {
        self.color_picker.mouse_release(button);
    }

    pub fn drag(&mut self, _button: MouseButton, interaction: &Interaction, screen_width: f32) -> Capture {
        if let Interaction::Drag { start, .. } = *interaction {
            if self.color_picker.is_selecting_color() {
                self.color_picker.drag(interaction, screen_width)?;
            }

            if start.x <= self.full_width || start.x >= screen_width - 256.0 {
                return Capture::AllowDrag;
            }
        }
        Capture::Miss
    }

    pub fn get_index_for_mouse_y(&self, mouse_y: f32, height: f32) -> Option<usize> {
        let beginning = self.calculate_beginning_y(height) - self.icon_size / 2.0 - self.padding / 2.0;
        let mut next_y = beginning;

        for i in 0..self.buttons.len() {
            if mouse_y >= next_y && mouse_y <= next_y + self.icon_size + self.padding {
                return Some(i);
            }
            next_y += self.icon_size + self.padding;
        }

        None
    }

    pub fn update(&mut self, window_height: f32) {
        let mut next_y = self.calculate_beginning_y(window_height);

        for button in self.buttons.iter_mut() {
            button.set_raw_position(self.icon_size / 2.0, next_y);
            next_y += self.icon_size + self.padding;
        }

        let target_selection_y = self.calculate_center_y_for_index(window_height, self.selection.index);
        self.selection.update(target_selection_y);
    }

    pub fn calculate_beginning_y(&self, window_height: f32) -> f32 {
        window_height / 2.0 - self.calculate_stripe_height() / 2.0
    }

    pub fn calculate_center_y_for_index(&self, window_height: f32, index: usize) -> f32 {
        self.calculate_beginning_y(window_height) + (self.icon_size + self.padding) * index as f32
    }

    pub fn calculate_stripe_height(&self) -> f32 {
        let num_buttons = self.buttons.len();
        let stripe_height = (num_buttons as f32) * self.icon_size + (num_buttons as f32 - 1.0).max(0.0) * self.padding;
        stripe_height
    }

    pub fn get_selected(&self) -> Option<ToolKey> {
        Some(self.buttons.get(self.selection.index)?.tool_key)
    }

    pub fn render(
        &self,
        ctx: &mut RenderCtx,
        tools: &mut SlotMap<ToolKey, Box<dyn Tool>>,
        color_scheme: &ColorScheme,
        text_system: &TextSystem,
        font: Rc<FontTexture>,
    ) {
        // Background
        ctx.ribbon_mesh.render(ctx, 0.0, 0.0, self.full_width, ctx.height, &color_scheme.dark_ribbon);

        // Icons
        for button in self.buttons.iter() {
            button.render(ctx, &color_scheme.foreground);
        }

        // Tool Options
        if let Some(tool_key) = self.get_selected() {
            tools[tool_key].render_options(ctx, text_system, font);
        }

        // Selection
        self.selection.render(ctx, &color_scheme.foreground);

        // Options background
        ctx.ribbon_mesh
            .render(ctx, ctx.width - 256.0, 0.0, 256.0, ctx.height, &color_scheme.dark_ribbon);

        // Draw hue/value picker
        self.color_picker.render(ctx);
    }
}

pub struct SelectionIndicator {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub start_y: f32,
    pub instant: Instant,
    pub duration: Duration,
    pub mesh: MeshXyz,
}

impl SelectionIndicator {
    pub fn new(x: f32, display: &Display) -> Self {
        Self {
            index: 0,
            x: x,
            y: 0.0,
            start_y: 0.0,
            instant: Instant::now(),
            duration: Duration::from_millis(100),
            mesh: MeshXyz::new(include_str!("_src_objs/selection_bubble.obj"), display),
        }
    }

    pub fn update(&mut self, target_y: f32) {
        let since_instant = Instant::now() - self.instant;

        let t = if since_instant > self.duration {
            1.0
        } else {
            since_instant.as_secs_f32() / self.duration.as_secs_f32()
        }
        .exponential_out();

        self.y = Lerp::lerp(&self.start_y, &target_y, &t);
    }

    pub fn select(&mut self, index: usize) {
        self.index = index;
        self.start_y = self.y;
        self.instant = Instant::now();
    }

    pub fn render(&self, ctx: &mut RenderCtx, color: &Color) {
        let identity = glm::identity::<f32, 4>();
        let transformation = glm::translation(&glm::vec3(self.x, self.y, 0.0));
        let transformation = glm::scale(&transformation, &glm::vec3(16.0, 16.0, 0.0));
        let transformation = glm::scale(&transformation, &glm::vec3(0.5, 0.5, 0.0)); // (since icons are in 2x2 meters, we have downscale by factor of 2)

        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: reach_inside_mat4(&identity),
            projection: reach_inside_mat4(ctx.projection),
            color: Into::<[f32; 4]>::into(color)
        };

        ctx.draw(&self.mesh.vertex_buffer, &self.mesh.indices, ctx.color_shader, &uniforms, &Default::default())
            .unwrap();
    }
}
