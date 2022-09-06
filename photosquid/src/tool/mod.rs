mod circle;
mod pan;
mod pointer;
mod rect;
mod tri;

use crate::{
    aabb::AABB,
    app::{
        App,
        SaveMethod::{Save, SaveAs},
    },
    camera::EasySmoothCamera,
    capture::{Capture, KeyCapture},
    interaction::{ClickInteraction, Interaction, KeyInteraction},
    render_ctx::RenderCtx,
    user_input::{Button, Checkbox, TextInput, UserInput},
};
use glium::glutin::event::VirtualKeyCode;
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use slotmap::new_key_type;
use std::rc::Rc;
use VirtualKeyCode::Escape;

new_key_type! { pub struct ToolKey; }

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ToolKind {
    MainMenu,
    Circle,
    Pan,
    Pointer,
    Rect,
    Tri,
}

pub struct Tool {
    kind: ToolKind,
    user_inputs: Vec<UserInput>,
}

impl Tool {
    pub fn main_menu() -> Self {
        Self {
            kind: ToolKind::MainMenu,
            user_inputs: vec![
                UserInput::Button(Button::new("Open".to_string(), Box::new(|app| app.load()))),
                UserInput::Button(Button::new("Save".to_string(), Box::new(|app| app.save(Save)))),
                UserInput::Button(Button::new("Save As".to_string(), Box::new(|app| app.save(SaveAs)))),
                UserInput::Button(Button::new("Export".to_string(), Box::new(|app| app.export()))),
                UserInput::Button(Button::new("About".to_string(), Box::new(|app| app.about()))),
            ],
        }
    }

    pub fn circle() -> Self {
        Self {
            kind: ToolKind::Circle,
            user_inputs: vec![UserInput::TextInput(TextInput::new("50".into(), "Initial Radius".into(), "".into()))],
        }
    }

    pub fn pan() -> Self {
        Self {
            kind: ToolKind::Pan,
            user_inputs: vec![
                UserInput::TextInput(TextInput::new("0".into(), "Camera X".into(), "".into())),
                UserInput::TextInput(TextInput::new("0".into(), "Camera Y".into(), "".into())),
            ],
        }
    }

    pub fn pointer() -> Self {
        Self {
            kind: ToolKind::Pointer,
            user_inputs: vec![
                UserInput::TextInput(TextInput::new("0".into(), "Translation Snapping".into(), "".into())),
                UserInput::TextInput(TextInput::new("0".into(), "Rotation Snapping".into(), " degrees".into())),
            ],
        }
    }

    pub fn rect() -> Self {
        Self {
            kind: ToolKind::Rect,
            user_inputs: vec![
                UserInput::TextInput(TextInput::new("100".into(), "Initial Width".into(), "".into())),
                UserInput::TextInput(TextInput::new("100".into(), "Initial Height".into(), "".into())),
                UserInput::TextInput(TextInput::new("0".into(), "Initial Rotation".into(), " degrees".into())),
                UserInput::TextInput(TextInput::new("0".into(), "Initial Corner Radii".into(), "".into())),
                UserInput::Checkbox(Checkbox::new("Create Viewport".into(), false)),
            ],
        }
    }

    pub fn tri() -> Self {
        Self {
            kind: ToolKind::Tri,
            user_inputs: vec![UserInput::TextInput(TextInput::new("0".into(), "Initial Rotation".into(), " degrees".into()))],
        }
    }

    pub fn interact(&mut self, interaction: Interaction, app: &mut App) -> Capture {
        match self.kind {
            ToolKind::MainMenu => Capture::Miss,
            ToolKind::Circle => circle::interact(&mut self.user_inputs, interaction, app),
            ToolKind::Pan => pan::interact(&mut self.user_inputs, interaction, app),
            ToolKind::Pointer => pointer::interact(&mut self.user_inputs, interaction, app),
            ToolKind::Rect => rect::interact(&mut self.user_inputs, interaction, app),
            ToolKind::Tri => tri::interact(&mut self.user_inputs, interaction, app),
        }
    }

    pub fn interact_options(&mut self, interaction: Interaction, app: &mut App) -> Capture {
        // Do interaction
        let capture = self.interact_options_impl(interaction, app);

        // Post interaction
        if self.kind == ToolKind::Pan {
            let existing_position = app.camera.get_real().position;

            // Update options
            if let Some(new_content) = self.user_inputs[0].as_text_input_mut().unwrap().poll() {
                let new_x = new_content.parse::<f32>().unwrap_or_default();
                app.camera.set_location(glm::vec2(new_x, existing_position.y));
            }

            if let Some(new_content) = self.user_inputs[1].as_text_input_mut().unwrap().poll() {
                let new_y = new_content.parse::<f32>().unwrap_or_default();
                app.camera.set_location(glm::vec2(existing_position.x, new_y));
            }
        }

        capture
    }

    fn interact_options_impl(&mut self, interaction: Interaction, app: &mut App) -> Capture {
        match interaction {
            Interaction::Click(ClickInteraction { button, position, .. }) => {
                let index_took_focus = self.user_inputs.iter_mut().enumerate().find_map(|(i, user_input)| {
                    if user_input.click(button, &position, &get_nth_input_area(i), app) == Capture::TakeFocus {
                        Some(i)
                    } else {
                        None
                    }
                });

                if let Some(index_took_focus) = index_took_focus {
                    for (i, user_input) in self.user_inputs.iter_mut().enumerate() {
                        if i != index_took_focus {
                            user_input.unfocus();
                        }
                    }
                    return Capture::TakeFocus;
                }
            }
            Interaction::Key(KeyInteraction { virtual_keycode }) => {
                let shift = app.keys_held.contains(&VirtualKeyCode::LShift);

                if let Some(key_capture) = self
                    .user_inputs
                    .iter_mut()
                    .find_map(|user_input| user_input.key_press(virtual_keycode, shift).to_option())
                {
                    return Capture::Keyboard(key_capture);
                }

                if self.kind == ToolKind::Pointer && virtual_keycode == Escape {
                    app.selections.clear();
                    return Capture::Keyboard(KeyCapture::Capture);
                }
            }
            _ => (),
        }

        Capture::Miss
    }

    pub fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        // Pre-render
        if self.kind == ToolKind::Pan {
            let x_input = self.user_inputs[0].as_text_input_mut().unwrap();
            if !x_input.is_focused() {
                x_input.set(&ctx.real_camera.position.x.round().to_string());
            }

            let y_input = self.user_inputs[1].as_text_input_mut().unwrap();
            if !y_input.is_focused() {
                y_input.set(&ctx.real_camera.position.y.round().to_string());
            }
        }

        // Render
        for i in 0..self.user_inputs.len() {
            self.user_inputs[i].render(ctx, text_system, font.clone(), &get_nth_input_area(i));
        }
    }

    pub fn kind(&self) -> ToolKind {
        self.kind
    }
}

fn get_nth_input_area(n: usize) -> AABB {
    TextInput::standard_area(&glm::vec2(64.0, 128.0 + n as f32 * 80.0))
}
