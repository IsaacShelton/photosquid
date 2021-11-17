use crate::{
    app::ApplicationState, capture::Capture, interaction::Interaction, render_ctx::RenderCtx, squid::Rect as RectSquid, text_input::TextInput, tool,
    tool::Tool, user_input::UserInput,
};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextSystem};
use std::rc::Rc;

pub struct Rect {
    width_input: UserInput,
    height_input: UserInput,
    rotation_input: UserInput,

    // Tool options
    initial_width: f32,
    initial_height: f32,
    initial_rotation: f32,
}

impl Rect {
    pub fn new() -> Self {
        Self {
            width_input: UserInput::TextInput(TextInput::new("100".into(), "Initial Width".into(), "".into())),
            height_input: UserInput::TextInput(TextInput::new("100".into(), "Initial Height".into(), "".into())),
            rotation_input: UserInput::TextInput(TextInput::new("0".into(), "Initial Rotation".into(), " degrees".into())),

            initial_width: 100.0,
            initial_height: 100.0,
            initial_rotation: 0.0,
        }
    }
}

impl Tool for Rect {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        // Update options
        if let Some(new_content) = self.width_input.as_text_input_mut().unwrap().poll() {
            self.initial_width = new_content.parse::<f32>().unwrap_or_default().max(4.0);
        }

        if let Some(new_content) = self.height_input.as_text_input_mut().unwrap().poll() {
            self.initial_height = new_content.parse::<f32>().unwrap_or_default().max(4.0);
        }

        if let Some(new_content) = self.rotation_input.as_text_input_mut().unwrap().poll() {
            self.initial_rotation = new_content.parse::<f32>().unwrap_or_default().max(0.0) * std::f32::consts::PI / 180.0;
        }

        // Handle interaction
        if let Interaction::Click {
            button: MouseButton::Left,
            position,
        } = interaction
        {
            let world_position = position - app.camera.get_animated();
            let color = app.toolbox.color_picker.calculate_color();

            app.insert(Box::new(RectSquid::new(
                world_position.x,
                world_position.y,
                self.initial_width,
                self.initial_height,
                self.initial_rotation,
                color,
            )));

            Capture::AllowDrag
        } else {
            Capture::Miss
        }
    }

    fn interact_options(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        tool::interact_user_inputs(vec![&mut self.width_input, &mut self.height_input, &mut self.rotation_input], interaction, app)
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_user_inputs(
            ctx,
            text_system,
            font,
            vec![&mut self.width_input, &mut self.height_input, &mut self.rotation_input],
        )
    }
}
