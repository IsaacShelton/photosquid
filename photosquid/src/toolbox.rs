use crate::{
    capture::Capture,
    color::Color,
    interaction::Interaction,
    matrix_helpers::reach_inside_mat4,
    mesh::MeshXyz,
    options,
    options::color_picker::ColorPicker,
    render_ctx::RenderCtx,
    smooth::Smooth,
    tool::{Tool, ToolKey},
    tool_button::ToolButton,
    ColorScheme,
};
use glium::{glutin::event::MouseButton, Display};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use slotmap::SlotMap;
use std::{rc::Rc, time::Duration};

pub struct ToolBox {
    buttons: Vec<ToolButton>,
    icon_size: f32,
    padding: f32,
    width: f32,
    full_width: f32,
    selection: SelectionIndicator,
    tab_selection: SelectionIndicator,
    options_tab_region_height: f32,
    options_tab_buttons: Vec<options::TabButton>,

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
            selection: SelectionIndicator::new(glm::zero(), false, display),
            tab_selection: SelectionIndicator::new(glm::vec2(10000000.0, 0.0), true, display),
            color_picker: ColorPicker::new(),
            options_tab_region_height: 64.0,
            options_tab_buttons: vec![],
        }
    }

    pub fn create_standard_tools(&mut self, tools: &mut SlotMap<ToolKey, Box<dyn Tool>>, display: &Display) {
        // Create tools and corresponding tool buttons

        use crate::{press_animation::DeformPressAnimation, tool};

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/pointer.obj"),
            Box::new(DeformPressAnimation {}),
            tools.insert(tool::Pointer::new()),
            &display,
        ));

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/pan.obj"),
            Box::new(DeformPressAnimation {}),
            tools.insert(tool::Pan::new()),
            &display,
        ));

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/rectangle.obj"),
            Box::new(DeformPressAnimation {}),
            tools.insert(tool::Rect::new()),
            &display,
        ));

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/triangle.obj"),
            Box::new(DeformPressAnimation {}),
            tools.insert(tool::Tri::new()),
            &display,
        ));

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/circle.obj"),
            Box::new(DeformPressAnimation {}),
            tools.insert(tool::Circle::new()),
            &display,
        ));

        // Select first tool
        self.select_tool(0);
    }

    pub fn create_standard_options_tabs(&mut self, tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>, display: &Display) {
        use crate::press_animation::DeformPressAnimation;

        self.add_options_tab_button(options::TabButton::new(
            include_str!("_src_objs/object.obj"),
            Box::new(DeformPressAnimation {}),
            tabs.insert(options::tab::Object::new()),
            &display,
        ));

        self.add_options_tab_button(options::TabButton::new(
            include_str!("_src_objs/layers.obj"),
            Box::new(DeformPressAnimation {}),
            tabs.insert(options::tab::Object::new()),
            &display,
        ));

        self.select_tab(0);
    }

    pub fn add_tool_button(&mut self, button: ToolButton) {
        self.buttons.push(button);
    }

    pub fn add_options_tab_button(&mut self, options_tab: options::TabButton) {
        self.options_tab_buttons.push(options_tab);
    }

    fn is_on_object_options(&self) -> bool {
        self.tab_selection.external_index == 0
    }

    pub fn select_tool(&mut self, index: usize) {
        if index < self.buttons.len() {
            for button in self.buttons.iter_mut() {
                button.animate(false);
            }

            self.buttons[index].animate(true);
            self.selection.external_index = index;
        }
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.options_tab_buttons.len() {
            for button in self.options_tab_buttons.iter_mut() {
                button.animate(false);
            }

            self.options_tab_buttons[index].animate(true);
            self.tab_selection.external_index = index;
        }
    }

    pub fn click(&mut self, button: MouseButton, mouse: &glm::Vec2, screen_width: f32, screen_height: f32) -> bool {
        // Tool ribbon
        if button == MouseButton::Left && mouse.x < self.width {
            let index = self.get_index_for_mouse_y(mouse.y, screen_height);

            if let Some(index) = index {
                self.select_tool(index);
            }

            return true;
        }

        // Squid options ribbon
        if button == MouseButton::Left && mouse.x > screen_width - 256.0 {
            let index = self.get_options_tab_index_for_mouse(mouse, screen_width);

            if let Some(index) = index {
                self.select_tab(index);
            } else if self.is_on_object_options() {
                let _ = self.color_picker.click(button, mouse, screen_width);
            }

            return true;
        }

        false
    }

    pub fn mouse_release(&mut self, button: MouseButton) {
        self.color_picker.mouse_release(button);
    }

    pub fn drag(&mut self, _button: MouseButton, interaction: &Interaction, screen_width: f32) -> Capture {
        if let Interaction::Drag { start, .. } = *interaction {
            if self.is_on_object_options() && self.color_picker.is_selecting_color() {
                self.color_picker.drag(interaction, screen_width)?;
            }

            if start.x <= self.full_width || start.x >= screen_width - 256.0 {
                return Capture::AllowDrag;
            }
        }
        Capture::Miss
    }

    fn get_index_for_mouse_y(&self, mouse_y: f32, height: f32) -> Option<usize> {
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

    fn get_options_tab_index_for_mouse(&self, mouse: &glm::Vec2, window_width: f32) -> Option<usize> {
        if mouse.y >= self.options_tab_region_height {
            return None;
        }

        let beginning = self.calculate_beginning_x(window_width) - self.icon_size / 2.0 - self.padding / 2.0;
        let mut next_x = beginning;

        for i in 0..self.buttons.len() {
            if mouse.x >= next_x && mouse.x <= next_x + self.icon_size + self.padding {
                return Some(i);
            }
            next_x += self.icon_size + self.padding;
        }

        None
    }

    pub fn update(&mut self, window_width: f32, window_height: f32) {
        self.update_tool_buttons(window_height);
        self.update_options_tab_buttons(window_width);
    }

    fn update_tool_buttons(&mut self, window_height: f32) {
        let mut next_y = self.calculate_beginning_y(window_height);

        for button in self.buttons.iter_mut() {
            button.set_raw_position(self.icon_size / 2.0, next_y);
            next_y += self.icon_size + self.padding;
        }

        let target_selection_y = self.calculate_center_y_for_index(window_height, self.selection.external_index);
        self.selection.position.set(glm::vec2(self.selection.position.get_real().x, target_selection_y));
    }

    fn update_options_tab_buttons(&mut self, window_width: f32) {
        let mut next_x = self.calculate_beginning_x(window_width);

        for button in self.options_tab_buttons.iter_mut() {
            button.set_raw_position(next_x, 8.0 + self.icon_size / 2.0);
            next_x += self.icon_size + self.padding;
        }

        let target_selection_x = self.calculate_center_x_for_index(window_width, self.tab_selection.external_index);
        self.tab_selection
            .position
            .set(glm::vec2(target_selection_x, self.tab_selection.position.get_real().y));
    }

    fn calculate_beginning_y(&self, window_height: f32) -> f32 {
        // Looks better without "true" center alignment
        window_height / 2.0 - self.calculate_stripe_height() / 2.0 /* + self.icon_size / 2.0*/
    }

    fn calculate_center_y_for_index(&self, window_height: f32, index: usize) -> f32 {
        self.calculate_beginning_y(window_height) + (self.icon_size + self.padding) * index as f32
    }

    fn calculate_stripe_height(&self) -> f32 {
        let num_buttons = self.buttons.len();
        let stripe_height = (num_buttons as f32) * self.icon_size + (num_buttons as f32 - 1.0).max(0.0) * self.padding;
        stripe_height
    }

    fn calculate_beginning_x(&self, window_width: f32) -> f32 {
        window_width - 256.0 / 2.0 - self.calculate_stripe_width() / 2.0 + self.icon_size / 2.0
    }

    fn calculate_center_x_for_index(&self, window_width: f32, index: usize) -> f32 {
        self.calculate_beginning_x(window_width) + (self.icon_size + self.padding) * index as f32
    }

    fn calculate_stripe_width(&self) -> f32 {
        let num_buttons = self.options_tab_buttons.len();
        let stripe_width = (num_buttons as f32) * self.icon_size + (num_buttons as f32 - 1.0).max(0.0) * self.padding;
        stripe_width
    }

    pub fn get_selected(&self) -> Option<ToolKey> {
        Some(self.buttons.get(self.selection.external_index)?.key)
    }

    pub fn render(
        &mut self,
        ctx: &mut RenderCtx,
        tools: &mut SlotMap<ToolKey, Box<dyn Tool>>,
        color_scheme: &ColorScheme,
        text_system: &TextSystem,
        font: Rc<FontTexture>,
    ) {
        // Background
        ctx.ribbon_mesh.render(ctx, 0.0, 0.0, self.full_width, ctx.height, &color_scheme.dark_ribbon);

        // Icons
        for button in self.buttons.iter_mut() {
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

        // Options Tabs
        for (i, button) in self.options_tab_buttons.iter_mut().enumerate() {
            button.render(
                ctx,
                if self.tab_selection.external_index == i {
                    &color_scheme.foreground
                } else {
                    &color_scheme.input
                },
            );
        }

        // Options Tab Selection
        self.tab_selection.render(ctx, &color_scheme.foreground);

        // Draw hue/value picker
        if self.is_on_object_options() {
            self.color_picker.render(ctx);
        }
    }
}

pub struct SelectionIndicator {
    pub external_index: usize,
    pub position: Smooth<glm::Vec2>,
    pub mesh: MeshXyz,
    pub horizontal: bool,
}

impl SelectionIndicator {
    pub fn new(start: glm::Vec2, horizontal: bool, display: &Display) -> Self {
        Self {
            external_index: 0,
            position: Smooth::new(start, Duration::from_millis(100)),
            mesh: MeshXyz::new(include_str!("_src_objs/selection_bubble.obj"), display),
            horizontal,
        }
    }

    pub fn render(&self, ctx: &mut RenderCtx, color: &Color) {
        let position = self.position.get_animated();

        let identity = glm::identity::<f32, 4>();
        let mut transformation = glm::translation(&glm::vec2_to_vec3(&position));

        transformation = glm::scale(&transformation, &glm::vec3(16.0, 16.0, 0.0));
        transformation = glm::scale(&transformation, &glm::vec3(0.5, 0.5, 0.0)); // (since icons are in 2x2 meters, we have downscale by factor of 2)

        if self.horizontal {
            transformation = glm::rotate(&transformation, std::f32::consts::FRAC_PI_2, &glm::vec3(0.0, 0.0, 1.0));
        }

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
