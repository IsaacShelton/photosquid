use crate::{
    app::ApplicationState, capture::Capture, interaction::Interaction, render_ctx::RenderCtx, squid::Tri as TriSquid, text_input::TextInput, tool, tool::Tool,
};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Tri {
    rotation_input: TextInput,

    // Tool options
    initial_rotation: f32,
}

impl Tri {
    pub fn new() -> Box<dyn Tool> {
        Box::new(Self {
            rotation_input: TextInput::new("0".into(), "Initial Rotation".into(), " degrees".into()),

            initial_rotation: 0.0,
        })
    }
}

impl Tool for Tri {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        // Update options
        if let Some(new_content) = self.rotation_input.poll() {
            self.initial_rotation = new_content.parse::<f32>().unwrap_or_default().max(0.0) * std::f32::consts::PI / 180.0;
        }

        // Handle interaction
        match interaction {
            Interaction::Click {
                button: MouseButton::Left,
                position,
            } => {
                let world_position = position - app.camera.get_animated();
                let color = app.toolbox.color_picker.calculate_color();
                app.insert(Box::new(TriSquid::new(
                    world_position + glm::vec2(0.0, -50.0),
                    world_position + glm::vec2(50.0, 50.0),
                    world_position + glm::vec2(-50.0, 50.0),
                    self.initial_rotation,
                    color,
                )));
                return Capture::AllowDrag;
            }
            _ => (),
        }

        Capture::Miss
    }

    fn interact_options(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        tool::interact_text_inputs(vec![&mut self.rotation_input], interaction, app)
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_text_inputs(ctx, text_system, font, vec![&mut self.rotation_input])
    }
}
