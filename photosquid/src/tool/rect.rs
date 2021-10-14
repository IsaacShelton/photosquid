use super::{Capture, Interaction, Tool};
use crate::{app::ApplicationState, squid::Rect as RectSquid};
use glium::glutin::event::MouseButton;

pub struct Rect {}

impl Rect {
    pub fn new() -> Box<dyn Tool> {
        Box::new(Self {})
    }
}

impl Tool for Rect {
    fn interact(&self, interaction: Interaction, app: &mut ApplicationState) -> Capture {
        let ocean = &mut app.ocean;
        let display = &app.display;

        match interaction {
            Interaction::Click {
                button: MouseButton::Left,
                position,
            } => {
                let world_position = position - app.camera.get_animated();
                let color = app.toolbox.color_picker.calculate_color();
                ocean
                    .squids
                    .insert(Box::new(RectSquid::new(world_position.x, world_position.y, 100.0, 100.0, color, display)));
                return Capture::AllowDrag;
            }
            _ => (),
        }

        Capture::Miss
    }
}
