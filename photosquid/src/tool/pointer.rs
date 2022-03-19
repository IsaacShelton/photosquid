use crate::{
    app::App,
    bool_poll::BoolPoll,
    capture::{Capture, KeyCapture},
    interaction::{ClickInteraction, DragInteraction, Interaction, KeyInteraction},
    math_helpers::get_point_delta_rotation,
    operation::Operation,
    selection::{NewSelection, TrySelectResult},
    squid::Initiation,
    user_input::UserInput,
};
use angular_units::Rad;
use glium::glutin::event::{MouseButton, VirtualKeyCode};
use nalgebra_glm as glm;

pub fn interact(user_inputs: &mut Vec<UserInput>, interaction: Interaction, app: &mut App) -> Capture {
    poll_to_set_program_wide_options(user_inputs, app);

    match interaction {
        Interaction::Click(ClickInteraction { button, position, .. }) => {
            app.preclick();

            let result = app.ocean.try_select(position, &app.camera.get_animated(), &app.selections);

            // If we wouldn't be selecting anything new, prefer to interact
            // with existing selection over re-selecting/un-selecting
            if !matches!(result, TrySelectResult::New { .. }) {
                app.try_interact_with_selections(&interaction)?;
            }

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

            if button == MouseButton::Right {
                app.context_menu = app.ocean.try_context_menu(position, &app.camera.get_animated(), &app.color_scheme);

                if app.context_menu.is_some() {
                    return Capture::NoDrag;
                }
            }

            Capture::AllowDrag
        }
        Interaction::Drag(DragInteraction { current: mouse_position, .. }) => match &mut app.operation {
            Some(Operation::Rotate { point, rotation }) => {
                let delta_theta = get_point_delta_rotation(point, &mouse_position, *rotation) - Rad::pi_over_2();
                *rotation += delta_theta;
                Capture::RotateSelectedSquids { delta_theta }
            }
            Some(Operation::Scale { origin, point }) => {
                let d0 = glm::distance(origin, point);
                let world_position = app.camera.get_animated().apply_reverse(&mouse_position);
                let df = glm::distance(origin, &world_position);
                let total_scale_factor = df / d0;
                Capture::ScaleSelectedSquids { total_scale_factor }
            }
            Some(Operation::Spread { .. }) => Capture::SpreadSelectedSquids {
                current: app.camera.get_animated().apply_reverse(&mouse_position),
            },
            Some(Operation::Revolve { .. }) => Capture::RevolveSelectedSquids {
                current: app.camera.get_animated().apply_reverse(&mouse_position),
            },
            Some(Operation::Dilate { .. }) => Capture::DilateSelectedSquids {
                current: app.camera.get_animated().apply_reverse(&mouse_position),
            },
            None => {
                app.try_interact_with_selections(&interaction)?;
                Capture::AllowDrag
            }
        },
        Interaction::Key(KeyInteraction { virtual_keycode }) => {
            app.try_interact_with_selections(&interaction)?;
            pointer_handle_hotkey(app, virtual_keycode)
        }
        _ => {
            app.try_interact_with_selections(&interaction)?;
            Capture::AllowDrag
        }
    }
}

fn pointer_handle_hotkey(app: &mut App, virtual_keycode: VirtualKeyCode) -> Capture {
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

fn poll_to_set_program_wide_options(user_inputs: &mut Vec<UserInput>, app: &mut App) {
    if let Some(new_content) = user_inputs[0].as_text_input_mut().unwrap().poll() {
        app.interaction_options.translation_snapping = new_content.parse::<f32>().unwrap_or_default().max(1.0);
    }

    if let Some(new_content) = user_inputs[1].as_text_input_mut().unwrap().poll() {
        app.interaction_options.rotation_snapping = Rad(new_content.parse::<f32>().unwrap_or_default().max(0.0) * std::f32::consts::PI / 180.0);
    }
}
