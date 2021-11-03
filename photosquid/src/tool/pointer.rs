use super::{Capture, Interaction, KeyCapture, Tool};
use crate::{
    app::{ApplicationState, Operation},
    ocean::{NewSelection, TrySelectResult},
    render_ctx::RenderCtx,
    squid::{self, Initiation},
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

    #[must_use]
    pub fn try_select(&self, position: &glm::Vec2, app: &mut ApplicationState) -> TrySelectResult {
        app.ocean.try_select(&position, &app.camera.get_animated(), &app.selections)
    }

    fn handle_try_select_result(result: TrySelectResult, app: &mut ApplicationState) {
        match result {
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

                if let Some(squid) = app.ocean.get_mut(selection.squid_id) {
                    squid.select();
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

        if let Interaction::Drag { current, .. } = interaction {
            match &mut app.operation {
                Some(Operation::Rotation { point, rotation }) => {
                    let delta_theta = squid::get_point_delta_rotation(point, &current, *rotation) - std::f32::consts::FRAC_PI_2;
                    *rotation += delta_theta;
                    return Capture::RotateSelectedSquids { delta_theta };
                }
                Some(Operation::Scale { origin, point }) => {
                    let d0 = glm::distance(origin, point);
                    let df = glm::distance(origin, &(current - app.camera.get_animated()));
                    let total_scale_factor = df / d0;
                    return Capture::ScaleSelectedSquids { total_scale_factor };
                }
                None => (),
            }
        }

        let mouse = if let Some(position) = app.mouse_position {
            glm::vec2(position.x, position.y)
        } else {
            glm::zero()
        };
        let possible_selection = self.try_select(&mouse, app);

        // First off
        // If we can interact with existing selections, prefer that over selecting different objects
        if if let Interaction::Click { .. } = interaction {
            match possible_selection {
                TrySelectResult::New { .. } => false,
                _ => true,
            }
        } else {
            true
        } {
            app.try_interact_with_selections(&interaction)?;
        }

        // Otherwise, If we can't interact with the existing selections, then try to select/de-select if applicable
        match interaction {
            Interaction::Click { button: MouseButton::Left, .. } => {
                // Left Click - Try to select
                Self::handle_try_select_result(possible_selection, app);
            }
            Interaction::Click {
                button: MouseButton::Right,
                position,
            } => {
                // Right Click - Try to open context menu
                Self::handle_try_select_result(self.try_select(&position, app), app);
                app.context_menu = app.ocean.try_context_menu(&position, &app.camera.get_animated(), &app.color_scheme);

                if app.context_menu.is_some() {
                    return Capture::NoDrag;
                }
            }
            Interaction::Key { virtual_keycode } => {
                return match virtual_keycode {
                    VirtualKeyCode::G => {
                        app.initiate(Initiation::Translation);
                        Capture::Keyboard(KeyCapture::Capture)
                    }
                    VirtualKeyCode::R => {
                        app.initiate(Initiation::Rotation);
                        Capture::Keyboard(KeyCapture::Capture)
                    }
                    VirtualKeyCode::S => {
                        app.initiate(Initiation::Scale);
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
