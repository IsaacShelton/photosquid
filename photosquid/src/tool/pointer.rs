use super::{Capture, Interaction, KeyCapture, Tool};
use crate::{
    app::ApplicationState,
    ocean::{NewSelection, TrySelectResult},
    render_ctx::RenderCtx,
    squid::Initiation,
    text_input::TextInput,
    tool,
};
use glium::glutin::event::{MouseButton, VirtualKeyCode};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Pointer {
    translation_snapping_input: TextInput,
    rotation_snapping_input: TextInput,
}

impl Pointer {
    pub fn new() -> Box<dyn Tool> {
        Box::new(Self {
            translation_snapping_input: TextInput::new("0".into(), "Translation Snapping".into(), "".into()),
            rotation_snapping_input: TextInput::new("0".into(), "Rotation Snapping".into(), " degrees".into()),
        })
    }

    pub fn try_select(&self, position: &glm::Vec2, app: &mut ApplicationState) {
        match app.ocean.try_select(&position, &app.camera.get_animated(), &app.selections) {
            TrySelectResult::New(NewSelection { selection, info }) => {
                // Clear existing selection unless holding shift
                if !app.keys_held.contains(&VirtualKeyCode::LShift) {
                    app.selections.clear();
                }

                // Add to selection
                app.selections.push(selection);

                // Notify UI of changes
                if let Some(its_color) = info.color {
                    app.toolbox.color_picker.set_selected_color_no_notif(its_color);
                }
            }
            TrySelectResult::Preserve => (),
            TrySelectResult::Discard => app.selections.clear(),
        }
    }
}

impl Tool for Pointer {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        // Update options
        if let Some(new_content) = self.translation_snapping_input.poll() {
            app.interaction_options.translation_snapping = new_content.parse::<f32>().unwrap_or_default().max(1.0);
        }

        if let Some(new_content) = self.rotation_snapping_input.poll() {
            app.interaction_options.rotation_snapping = new_content.parse::<f32>().unwrap_or_default().max(0.0) * std::f32::consts::PI / 180.0;
        }

        // Pre-notify all squids of incoming click if applicable
        // (used for resetting internal states)
        if let Interaction::Click { .. } = interaction {
            app.preclick();
        }

        // First off
        // If we can interact with existing selections, prefer that over selecting different objects
        app.try_interact_with_selections(&interaction)?;

        // Otherwise, If we can't interact with the existing selections, then try to select/de-select if applicable
        match interaction {
            Interaction::Click {
                button: MouseButton::Left,
                position,
            } => {
                // Left Click - Try to select
                self.try_select(&position, app);
            }
            Interaction::Click {
                button: MouseButton::Right,
                position,
            } => {
                // Right Click - Try to open context menu
                self.try_select(&position, app);
                app.context_menu = app.ocean.try_context_menu(&position, &app.camera.get_animated(), &app.color_scheme);

                if app.context_menu.is_some() {
                    return Capture::NoDrag;
                }
            }
            Interaction::Key { virtual_keycode } => {
                return match virtual_keycode {
                    VirtualKeyCode::G => {
                        app.initiate(Initiation::TRANSLATION);
                        Capture::Keyboard(KeyCapture::Capture)
                    }
                    _ => Capture::Miss,
                };
            }
            _ => (),
        }

        Capture::AllowDrag
    }

    fn interact_options(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        tool::interact_text_inputs(vec![&mut self.translation_snapping_input, &mut self.rotation_snapping_input], interaction, app)
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_text_inputs(
            ctx,
            text_system,
            font,
            vec![&mut self.translation_snapping_input, &mut self.rotation_snapping_input],
        )
    }
}
