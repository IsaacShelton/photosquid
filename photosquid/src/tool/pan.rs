use super::{Capture, Interaction, Tool};
use crate::{app::ApplicationState, render_ctx::RenderCtx, text_input::TextInput, tool, user_input::UserInput};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Pan {
    x_input: UserInput,
    y_input: UserInput,
}

impl Pan {
    pub fn new() -> Self {
        Self {
            x_input: UserInput::TextInput(TextInput::new("0".into(), "Camera X".into(), "".into())),
            y_input: UserInput::TextInput(TextInput::new("0".into(), "Camera Y".into(), "".into())),
        }
    }

    fn poll_inputs(&mut self, app: &mut ApplicationState) {
        // Update options
        if let Some(new_content) = self.x_input.as_text_input_mut().unwrap().poll() {
            let real_camera_position = *app.camera.get_real();
            let new_x = -new_content.parse::<f32>().unwrap_or_default();
            app.camera.set(glm::vec2(new_x, real_camera_position.y));
        }

        if let Some(new_content) = self.y_input.as_text_input_mut().unwrap().poll() {
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
        match interaction {
            Interaction::Drag { delta, .. } => {
                app.camera.set(glm::round(&(app.camera.get_real() + delta)));

                let x_input = self.x_input.as_text_input_mut().unwrap();
                let y_input = self.y_input.as_text_input_mut().unwrap();

                if !x_input.is_focused() {
                    x_input.set(&Self::camera_coord_to_string(app.camera.get_real().x));
                }

                if !y_input.is_focused() {
                    y_input.set(&Self::camera_coord_to_string(app.camera.get_real().y));
                }

                Capture::AllowDrag
            }
            Interaction::Click { .. } => Capture::AllowDrag,
            _ => Capture::Miss,
        }
    }

    fn interact_options(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        let capture = tool::interact_user_inputs(vec![&mut self.x_input, &mut self.y_input], interaction, app);
        self.poll_inputs(app);
        capture
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_user_inputs(ctx, text_system, font, vec![&mut self.x_input, &mut self.y_input])
    }
}
