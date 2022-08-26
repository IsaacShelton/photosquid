use crate::{
    capture::Capture,
    color::Color,
    interaction::{ClickInteraction, DragInteraction, Interaction},
    matrix_helpers::reach_inside_mat4,
    mesh::MeshXyz,
    ocean::Ocean,
    options,
    options::color_picker::ColorPicker,
    press_animation::PressAnimation,
    render_ctx::RenderCtx,
    selection::Selection,
    smooth::Smooth,
    tool::{Tool, ToolKey, ToolKind},
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
            tab_selection: SelectionIndicator::new(glm::vec2(10_000_000.0, 0.0), true, display),
            color_picker: Default::default(),
            options_tab_region_height: 64.0,
            options_tab_buttons: vec![],
        }
    }

    pub fn create_standard_tools(&mut self, tools: &mut SlotMap<ToolKey, Tool>, display: &Display) {
        // Create tools and corresponding tool buttons

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/hamburger.obj"),
            PressAnimation::Scale,
            tools.insert(Tool::main_menu()),
            display,
        ));

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/pointer.obj"),
            PressAnimation::Deform,
            tools.insert(Tool::pointer()),
            display,
        ));

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/pan.obj"),
            PressAnimation::Deform,
            tools.insert(Tool::pan()),
            display,
        ));

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/rectangle.obj"),
            PressAnimation::Deform,
            tools.insert(Tool::rect()),
            display,
        ));

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/triangle.obj"),
            PressAnimation::Deform,
            tools.insert(Tool::tri()),
            display,
        ));

        self.add_tool_button(ToolButton::new(
            include_str!("_src_objs/circle.obj"),
            PressAnimation::Deform,
            tools.insert(Tool::circle()),
            display,
        ));

        // Select first non-menu tool
        self.select_tool(1);
    }

    pub fn create_standard_options_tabs(&mut self, tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>, display: &Display) {
        self.add_options_tab_button(options::TabButton::new(
            include_str!("_src_objs/object.obj"),
            PressAnimation::Deform,
            tabs.insert(Box::new(options::tab::Object::new())),
            display,
        ));

        self.add_options_tab_button(options::TabButton::new(
            include_str!("_src_objs/layers.obj"),
            PressAnimation::Deform,
            tabs.insert(Box::new(options::tab::Layers::new())),
            display,
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
            for button in &mut self.buttons {
                button.animate(false);
            }

            self.buttons[index].animate(true);
            self.selection.external_index = index;
        }
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.options_tab_buttons.len() {
            for button in &mut self.options_tab_buttons {
                button.animate(false);
            }

            self.options_tab_buttons[index].animate(true);
            self.tab_selection.external_index = index;
        }
    }

    pub fn click(&mut self, interaction: Interaction, screen_width: f32, screen_height: f32) -> Capture {
        let ClickInteraction { button, position: mouse } = interaction.as_click().unwrap();

        // Tool ribbon
        if *button == MouseButton::Left && mouse.x < self.width {
            let index = self.get_index_for_mouse_y(mouse.y, screen_height);

            if let Some(index) = index {
                self.select_tool(index);
            }

            return Capture::AllowDrag;
        }

        // Options tab picker and color picker
        if *button == MouseButton::Left && mouse.x > screen_width - 256.0 {
            if let Some(index) = self.get_options_tab_index_for_mouse(*mouse, screen_width) {
                // Change options tab if another options tab was selected
                self.select_tab(index);
                return Capture::AllowDrag;
            }

            if self.is_on_object_options() && self.color_picker.click(*button, *mouse, screen_width) {
                // Do color picker if applicable
                return Capture::AllowDrag;
            }
        }

        Capture::Miss
    }

    pub fn mouse_release(&mut self, button: MouseButton) {
        self.color_picker.mouse_release(button);
    }

    pub fn drag(&mut self, _button: MouseButton, interaction: &Interaction, screen_width: f32) -> Capture {
        if let Interaction::Drag(DragInteraction { start, .. }) = *interaction {
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

    fn get_options_tab_index_for_mouse(&self, mouse: glm::Vec2, window_width: f32) -> Option<usize> {
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

    pub fn get_current_options_tab_key(&self) -> options::tab::TabKey {
        self.options_tab_buttons[self.tab_selection.external_index].key
    }

    pub fn update(&mut self, window_width: f32, window_height: f32) {
        self.update_tool_buttons(window_height);
        self.update_options_tab_buttons(window_width);
    }

    fn update_tool_buttons(&mut self, window_height: f32) {
        let mut next_y = self.calculate_beginning_y(window_height);

        for button in &mut self.buttons {
            button.set_raw_position(self.icon_size / 2.0, next_y);
            next_y += self.icon_size + self.padding;
        }

        let target_selection_y = self.calculate_center_y_for_index(window_height, self.selection.external_index);
        self.selection.position.set(glm::vec2(self.selection.position.get_real().x, target_selection_y));
    }

    fn update_options_tab_buttons(&mut self, window_width: f32) {
        let mut next_x = self.calculate_beginning_x(window_width);

        for button in &mut self.options_tab_buttons {
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
        (num_buttons as f32) * self.icon_size + (num_buttons as f32 - 1.0).max(0.0) * self.padding
    }

    fn calculate_beginning_x(&self, window_width: f32) -> f32 {
        window_width - 256.0 / 2.0 - self.calculate_stripe_width() / 2.0 + self.icon_size / 2.0
    }

    fn calculate_center_x_for_index(&self, window_width: f32, index: usize) -> f32 {
        self.calculate_beginning_x(window_width) + (self.icon_size + self.padding) * index as f32
    }

    fn calculate_stripe_width(&self) -> f32 {
        let num_buttons = self.options_tab_buttons.len();
        (num_buttons as f32) * self.icon_size + (num_buttons as f32 - 1.0).max(0.0) * self.padding
    }

    pub fn get_selected(&self) -> Option<ToolKey> {
        Some(self.buttons.get(self.selection.external_index)?.key)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &mut RenderCtx,
        tools: &mut SlotMap<ToolKey, Tool>,
        options_tabs: &mut SlotMap<options::tab::TabKey, Box<dyn options::tab::Tab>>,
        color_scheme: &ColorScheme,
        text_system: &TextSystem,
        font: Rc<FontTexture>,
        ocean: &mut Ocean,
        selections: &[Selection],
    ) {
        // Background
        ctx.ribbon_mesh
            .render(ctx, glm::zero(), glm::vec2(self.full_width, ctx.height), &color_scheme.dark_ribbon);

        // Icons
        for button in &mut self.buttons {
            button.render(ctx, &color_scheme.foreground);
        }

        // Tool Options
        if let Some(tool_key) = self.get_selected() {
            tools[tool_key].render_options(ctx, text_system, font.clone());
        }

        // Selection
        self.selection.render(ctx, &color_scheme.foreground);

        // Options background
        ctx.ribbon_mesh
            .render(ctx, glm::vec2(ctx.width - 256.0, 0.0), glm::vec2(256.0, ctx.height), &color_scheme.dark_ribbon);

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

        // Draw panel for tab of options menu
        let options_tab_key = self.options_tab_buttons[self.tab_selection.external_index].key;
        if let Some(tab) = options_tabs.get_mut(options_tab_key) {
            tab.render(ctx, text_system, font, ocean, selections);
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
            position: Smooth::new(start, Some(Duration::from_millis(100))),
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

pub fn find_tool(tools: &mut SlotMap<ToolKey, Tool>, kind: ToolKind) -> Option<&mut Tool> {
    for (tool_key, tool) in tools.iter() {
        if tool.kind() == kind {
            return Some(tools.get_mut(tool_key).unwrap());
        }
    }

    None
}
