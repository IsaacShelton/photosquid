use crate::{
    app::ApplicationState, capture::Capture, interaction::Interaction, render_ctx::RenderCtx, squid::Circle as CircleSquid, text_input::TextInput, tool,
    tool::Tool, user_input::UserInput,
};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextSystem};
use std::rc::Rc;

pub struct Circle {
    radius_input: UserInput,

    // Tool options
    initial_radius: f32,
}

impl Circle {
    pub fn new() -> Self {
        Self {
            radius_input: UserInput::TextInput(TextInput::new("50".into(), "Initial Radius".into(), "".into())),
            initial_radius: 50.0,
        }
    }
}

impl Tool for Circle {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        // Update options
        if let Some(new_content) = self.radius_input.as_text_input_mut().unwrap().poll() {
            self.initial_radius = new_content.parse::<f32>().unwrap_or_default().max(4.0);
        }

        // Handle interaction
        if let Interaction::Click {
            button: MouseButton::Left,
            position,
        } = interaction
        {
            let world_position = position - app.camera.get_animated();
            let color = app.toolbox.color_picker.calculate_color();

            app.insert(Box::new(CircleSquid::new(world_position.x, world_position.y, self.initial_radius, color)));

            Capture::AllowDrag
        } else {
            Capture::Miss
        }
    }

    fn interact_options(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        tool::interact_user_inputs(vec![&mut self.radius_input], interaction, app)
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_user_inputs(ctx, text_system, font, vec![&mut self.radius_input])
    }
}
