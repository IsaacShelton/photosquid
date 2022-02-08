use super::{Capture, Interaction, Tool};
use crate::{
    app::ApplicationState,
    camera::{Camera, EasySmoothCamera},
    interaction::DragInteraction,
    render_ctx::RenderCtx,
    text_input::TextInput,
    tool,
    user_input::UserInput,
};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Pan {
    x_input: UserInput,
    y_input: UserInput,
}

impl Pan {
    pub const TOOL_NAME: &'static str = "photosquid.pan";

    pub fn new() -> Self {
        Self {
            x_input: UserInput::TextInput(TextInput::new("0".into(), "Camera X".into(), "".into())),
            y_input: UserInput::TextInput(TextInput::new("0".into(), "Camera Y".into(), "".into())),
        }
    }

    fn poll_inputs(&mut self, app: &mut ApplicationState) {
        let existing_position = app.camera.get_real().position;

        // Update options
        if let Some(new_content) = self.x_input.as_text_input_mut().unwrap().poll() {
            let new_x = new_content.parse::<f32>().unwrap_or_default();
            app.camera.set_location(glm::vec2(new_x, existing_position.y));
        }

        if let Some(new_content) = self.y_input.as_text_input_mut().unwrap().poll() {
            let new_y = new_content.parse::<f32>().unwrap_or_default();
            app.camera.set_location(glm::vec2(existing_position.x, new_y));
        }
    }

    fn update_ui_input_values(&mut self, real_camera: &Camera) {
        let x_input = self.x_input.as_text_input_mut().unwrap();
        let y_input = self.y_input.as_text_input_mut().unwrap();

        if !x_input.is_focused() {
            x_input.set(&Self::camera_coord_to_string(real_camera.position.x));
        }

        if !y_input.is_focused() {
            y_input.set(&Self::camera_coord_to_string(real_camera.position.y));
        }
    }

    fn camera_coord_to_string(value: f32) -> String {
        value.round().to_string()
    }
}

impl Tool for Pan {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        match interaction {
            Interaction::Drag(DragInteraction { delta, .. }) => {
                // Get the "real" camera, which is unaffected by animation
                let real_camera = app.camera.get_real();

                // Apply reverse camera transformation to the drag vector in order to
                // bring it into world space and then move the real camera position
                // in the opposite direction (as if it was being physically dragged)
                let new_camera_location = real_camera.position - real_camera.apply_reverse_to_vector(&delta);

                app.camera.set_location(new_camera_location);
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
        // Not the most minimal, but this is definitely the easiest way to
        // consistently have the most up-to-date information when rendering text inputs
        self.update_ui_input_values(ctx.real_camera);

        tool::render_user_inputs(ctx, text_system, font, vec![&mut self.x_input, &mut self.y_input]);
    }

    fn tool_name(&self) -> &'static str {
        Self::TOOL_NAME
    }
}
