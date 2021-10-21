use super::{Capture, Interaction, Tool};
use crate::{app::ApplicationState, render_ctx::RenderCtx, text_input::TextInput, tool};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Pan {
    x_input: TextInput,
    y_input: TextInput,
}

impl Pan {
    pub fn new() -> Box<dyn Tool> {
        Box::new(Self {
            x_input: TextInput::new("0".into(), "Camera X".into(), "".into()),
            y_input: TextInput::new("0".into(), "Camera Y".into(), "".into()),
        })
    }

    fn poll_inputs(&mut self, app: &mut ApplicationState) {
        // Update options
        if let Some(new_content) = self.x_input.poll() {
            let real_camera_position = *app.camera.get_real();
            let new_x = -new_content.parse::<f32>().unwrap_or_default();
            app.camera.set(glm::vec2(new_x, real_camera_position.y));
        }

        if let Some(new_content) = self.y_input.poll() {
            let real_camera_position = *app.camera.get_real();
            let new_y = -new_content.parse::<f32>().unwrap_or_default();
            app.camera.set(glm::vec2(real_camera_position.x, new_y));
        }
    }

    fn camera_coord_to_string(value: f32) -> String {
        if value == 0.0 {
            "0".to_string()
        } else {
            (-value).to_string()
        }
    }
}

impl Tool for Pan {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        if let Interaction::Drag { delta, .. } = interaction {
            app.camera.set(glm::round(&(app.camera.get_real() + delta)));

            if !self.x_input.is_focused() {
                self.x_input.set(&Self::camera_coord_to_string(app.camera.get_real().x));
            }

            if !self.y_input.is_focused() {
                self.y_input.set(&Self::camera_coord_to_string(app.camera.get_real().y));
            }
        }
        Capture::AllowDrag
    }

    fn interact_options(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        let capture = tool::interact_text_inputs(vec![&mut self.x_input, &mut self.y_input], interaction, app);
        self.poll_inputs(app);
        capture
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_text_inputs(ctx, text_system, font, vec![&mut self.x_input, &mut self.y_input])
    }
}
