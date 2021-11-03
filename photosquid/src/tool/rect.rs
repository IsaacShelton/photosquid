use crate::{
    app::ApplicationState, capture::Capture, interaction::Interaction, render_ctx::RenderCtx, squid::Rect as RectSquid, text_input::TextInput, tool, tool::Tool,
};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextSystem};
use std::rc::Rc;

pub struct Rect {
    width_input: TextInput,
    height_input: TextInput,
    rotation_input: TextInput,

    // Tool options
    initial_width: f32,
    initial_height: f32,
    initial_rotation: f32,
}

impl Rect {
    pub fn new() -> Box<dyn Tool> {
        Box::new(Self {
            width_input: TextInput::new("100".into(), "Initial Width".into(), "".into()),
            height_input: TextInput::new("100".into(), "Initial Height".into(), "".into()),
            rotation_input: TextInput::new("0".into(), "Initial Rotation".into(), " degrees".into()),

            initial_width: 100.0,
            initial_height: 100.0,
            initial_rotation: 0.0,
        })
    }
}

impl Tool for Rect {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        // Update options
        if let Some(new_content) = self.width_input.poll() {
            self.initial_width = new_content.parse::<f32>().unwrap_or_default().max(4.0);
        }

        if let Some(new_content) = self.height_input.poll() {
            self.initial_height = new_content.parse::<f32>().unwrap_or_default().max(4.0);
        }

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
                app.insert(Box::new(RectSquid::new(
                    world_position.x,
                    world_position.y,
                    self.initial_width,
                    self.initial_height,
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
        tool::interact_text_inputs(vec![&mut self.width_input, &mut self.height_input, &mut self.rotation_input], interaction, app)
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_text_inputs(
            ctx,
            text_system,
            font,
            vec![&mut self.width_input, &mut self.height_input, &mut self.rotation_input],
        )
    }
}
