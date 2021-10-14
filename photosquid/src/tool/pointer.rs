use super::{Capture, Interaction, Tool};
use crate::{
    app::ApplicationState,
    ocean::{NewSelection, TrySelectResult},
};
use glium::glutin::event::{MouseButton, VirtualKeyCode};
use nalgebra_glm as glm;

pub struct Pointer {}

impl Pointer {
    pub fn new() -> Box<dyn Tool> {
        Box::new(Self {})
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
    fn interact(&self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
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
            _ => (),
        }

        Capture::AllowDrag
    }
}
