use super::{Capture, Interaction, KeyCapture, Tool};
use crate::{
    app::ApplicationState,
    bool_poll,
    operation::Operation,
    render_ctx::RenderCtx,
    selection::{NewSelection, TrySelectResult},
    squid::{self, Initiation},
    text_input::TextInput,
    tool,
    user_input::UserInput,
};
use angular_units::Rad;
use glium::glutin::event::{MouseButton, VirtualKeyCode};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Pointer {
    translation_snapping_input: UserInput,
    rotation_snapping_input: UserInput,
}

impl Pointer {
    pub fn new() -> Self {
        Self {
            translation_snapping_input: UserInput::TextInput(TextInput::new("0".into(), "Translation Snapping".into(), "".into())),
            rotation_snapping_input: UserInput::TextInput(TextInput::new("0".into(), "Rotation Snapping".into(), " degrees".into())),
        }
    }

    #[must_use]
    pub fn try_select(&self, position: &glm::Vec2, app: &mut ApplicationState) -> TrySelectResult {
        app.ocean.try_select(position, &app.camera.get_animated(), &app.selections)
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

    fn handle_hotkey(app: &mut ApplicationState, virtual_keycode: VirtualKeyCode) -> Capture {
        use bool_poll::BoolPoll;

        match virtual_keycode {
            VirtualKeyCode::G => {
                if app.perform_next_operation_collectively.poll() {
                    if let Some(center) = app.get_selection_group_center() {
                        app.initiate(Initiation::Spread {
                            point: app.get_mouse_in_world_space(),
                            center,
                        });
                    }
                } else {
                    app.initiate(Initiation::Translate);
                }
                Capture::Keyboard(KeyCapture::Capture)
            }
            VirtualKeyCode::R => {
                if app.perform_next_operation_collectively.poll() {
                    if let Some(center) = app.get_selection_group_center() {
                        app.initiate(Initiation::Revolve {
                            point: app.get_mouse_in_world_space(),
                            center,
                        });
                    }
                } else {
                    app.initiate(Initiation::Rotate);
                }
                Capture::Keyboard(KeyCapture::Capture)
            }
            VirtualKeyCode::S => {
                if app.perform_next_operation_collectively.poll() {
                    if let Some(center) = app.get_selection_group_center() {
                        app.initiate(Initiation::Dilate {
                            point: app.get_mouse_in_world_space(),
                            center,
                        });
                    }
                } else {
                    app.initiate(Initiation::Scale);
                }
                Capture::Keyboard(KeyCapture::Capture)
            }
            VirtualKeyCode::C => {
                app.perform_next_operation_collectively = !app.perform_next_operation_collectively;
                Capture::Keyboard(KeyCapture::Capture)
            }
            _ => Capture::Miss,
        }
    }

    fn poll_options(&mut self, app: &mut ApplicationState) {
        if let Some(new_content) = self.translation_snapping_input.as_text_input_mut().unwrap().poll() {
            app.interaction_options.translation_snapping = new_content.parse::<f32>().unwrap_or_default().max(1.0);
        }

        if let Some(new_content) = self.rotation_snapping_input.as_text_input_mut().unwrap().poll() {
            app.interaction_options.rotation_snapping = Rad(new_content.parse::<f32>().unwrap_or_default().max(0.0) * std::f32::consts::PI / 180.0);
        }
    }

    fn dispatch_drag(app: &mut ApplicationState, mouse_position: &glm::Vec2) -> Capture {
        match &mut app.operation {
            Some(Operation::Rotate { point, rotation }) => {
                let delta_theta = squid::get_point_delta_rotation(point, mouse_position, *rotation) - Rad::pi_over_2();
                *rotation += delta_theta;
                Capture::RotateSelectedSquids { delta_theta }
            }
            Some(Operation::Scale { origin, point }) => {
                let d0 = glm::distance(origin, point);
                let df = glm::distance(origin, &(mouse_position - app.camera.get_animated()));
                let total_scale_factor = df / d0;
                Capture::ScaleSelectedSquids { total_scale_factor }
            }
            Some(Operation::Spread { .. }) => Capture::SpreadSelectedSquids { current: *mouse_position },
            Some(Operation::Revolve { .. }) => Capture::RevolveSelectedSquids { current: *mouse_position },
            Some(Operation::Dilate { .. }) => Capture::DilateSelectedSquids { current: *mouse_position },
            None => Capture::Miss,
        }
    }

    fn dispatch_key(app: &mut ApplicationState, virtual_keycode: VirtualKeyCode) -> Capture {
        app.try_interact_with_selections(&Interaction::Key { virtual_keycode })?;
        Self::handle_hotkey(app, virtual_keycode)
    }

    fn try_open_context_menu(app: &mut ApplicationState, position: &glm::Vec2) -> Capture {
        app.context_menu = app.ocean.try_context_menu(position, &app.camera.get_animated(), &app.color_scheme);

        if app.context_menu.is_some() {
            Capture::NoDrag
        } else {
            Capture::Miss
        }
    }
}

impl Tool for Pointer {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        self.poll_options(app);

        match interaction {
            Interaction::Click { button, position, .. } => {
                app.preclick();

                let possible_selection = self.try_select(&position, app);

                if !matches!(possible_selection, TrySelectResult::New { .. }) {
                    app.try_interact_with_selections(&interaction)?;
                }

                Self::handle_try_select_result(possible_selection, app);

                if button == MouseButton::Right {
                    Self::try_open_context_menu(app, &position)?;
                }

                Capture::AllowDrag
            }
            Interaction::Drag { current: mouse_position, .. } => {
                Self::dispatch_drag(app, &mouse_position)?;
                app.try_interact_with_selections(&interaction)?;
                Capture::AllowDrag
            }
            Interaction::Key { virtual_keycode } => Self::dispatch_key(app, virtual_keycode),
            _ => {
                app.try_interact_with_selections(&interaction)?;
                Capture::AllowDrag
            }
        }
    }

    fn interact_options(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        use VirtualKeyCode::Escape;

        tool::interact_user_inputs(vec![&mut self.translation_snapping_input, &mut self.rotation_snapping_input], interaction, app)?;

        if let Interaction::Key { virtual_keycode: Escape } = interaction {
            app.selections.clear();
            Capture::Keyboard(KeyCapture::Capture)
        } else {
            Capture::Miss
        }
    }

    fn render_options(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>) {
        tool::render_user_inputs(
            ctx,
            text_system,
            font,
            vec![&mut self.translation_snapping_input, &mut self.rotation_snapping_input],
        );
    }
}
