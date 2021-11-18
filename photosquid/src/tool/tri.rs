use crate::{
    app::ApplicationState, capture::Capture, interaction::Interaction, render_ctx::RenderCtx, squid::Tri as TriSquid, text_input::TextInput, tool, tool::Tool,
    user_input::UserInput,
};
use angular_units::Rad;
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Tri {
    rotation_input: UserInput,

    // Tool options
    initial_rotation: Rad<f32>,
}

impl Tri {
    pub fn new() -> Tri {
        Self {
            rotation_input: UserInput::TextInput(TextInput::new("0".into(), "Initial Rotation".into(), " degrees".into())),
            initial_rotation: Rad(0.0),
        }
    }
}

impl Tool for Tri {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        // Update options
        if let Some(new_content) = self.rotation_input.as_text_input_mut().unwrap().poll() {
            self.initial_rotation = Rad(new_content.parse::<f32>().unwrap_or_default().max(0.0) * std::f32::consts::PI / 180.0);
        }

        // Handle interaction
        if let Interaction::Click {
            button: MouseButton::Left,
            position,
        } = interaction
        {
            let world_position = position - app.camera.get_animated();
            let color = app.toolbox.color_picker.calculate_color();

            app.insert(Box::new(TriSquid::new(
                world_position + glm::vec2(0.0, -50.0),
                world_position + glm::vec2(50.0, 50.0),
                world_position + glm::vec2(-50.0, 50.0),
                self.initial_rotation,
                color,
            )));

            Capture::AllowDrag
        } else {
            Capture::Miss
        }
    }

    fn interact_options(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        tool::interact_user_inputs(vec![&mut self.rotation_input], interaction, app)
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_user_inputs(ctx, text_system, font, vec![&mut self.rotation_input]);
    }
}
