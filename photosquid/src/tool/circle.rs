use crate::{
    app::ApplicationState, capture::Capture, interaction::Interaction, render_ctx::RenderCtx, squid::Circle as CircleSquid, text_input::TextInput, tool,
    tool::Tool,
};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextSystem};
use std::rc::Rc;

pub struct Circle {
    radius_input: TextInput,

    // Tool options
    initial_radius: f32,
}

impl Circle {
    pub fn new() -> Box<dyn Tool> {
        Box::new(Self {
            radius_input: TextInput::new("50".into(), "Initial Radius".into(), "".into()),

            initial_radius: 50.0,
        })
    }
}

impl Tool for Circle {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        // Update options
        if let Some(new_content) = self.radius_input.poll() {
            self.initial_radius = new_content.parse::<f32>().unwrap_or_default().max(4.0);
        }

        // Handle interaction
        match interaction {
            Interaction::Click {
                button: MouseButton::Left,
                position,
            } => {
                let world_position = position - app.camera.get_animated();
                let color = app.toolbox.color_picker.calculate_color();
                app.insert(Box::new(CircleSquid::new(world_position.x, world_position.y, self.initial_radius, color)));
                return Capture::AllowDrag;
            }
            _ => (),
        }

        Capture::Miss
    }

    fn interact_options(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        tool::interact_text_inputs(vec![&mut self.radius_input], interaction, app)
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_text_inputs(ctx, text_system, font, vec![&mut self.radius_input])
    }
}
