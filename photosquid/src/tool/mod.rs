mod circle;
mod pan;
mod pointer;
mod rect;
mod tri;

use crate::{
    aabb::AABB,
    app::ApplicationState,
    capture::{Capture, KeyCapture},
    interaction::Interaction,
    render_ctx::RenderCtx,
    text_input::TextInput,
    user_input::UserInput,
};
use glium::glutin::event::VirtualKeyCode;
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use slotmap::new_key_type;
use std::rc::Rc;

pub use circle::Circle;
pub use pan::Pan;
pub use pointer::Pointer;
pub use rect::Rect;
pub use tri::Tri;

new_key_type! { pub struct ToolKey; }

pub trait Tool {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture;

    fn interact_options(&mut self, _interaction: Interaction, _app: &mut ApplicationState) -> Capture {
        Capture::Miss
    }

    fn render_options(&mut self, _ctx: &mut RenderCtx, _text_system: &TextSystem, _font: Rc<FontTexture>) {}
}

pub fn get_nth_input_area(n: usize) -> AABB {
    TextInput::standard_area(&glm::vec2(64.0, 128.0 + n as f32 * 80.0))
}

fn take_focus_from_user_inputs_except(user_inputs: &mut Vec<&mut UserInput>, except_i: usize) {
    for (i, user_input) in user_inputs.iter_mut().enumerate() {
        if i != except_i {
            user_input.unfocus();
        }
    }
}

pub fn interact_user_inputs(user_inputs: Vec<&mut UserInput>, interaction: Interaction, app: &mut ApplicationState) -> Capture {
    let mut user_inputs = user_inputs;

    match interaction {
        Interaction::Click { button, position } => {
            let mut capture: Option<Capture> = None;
            let mut from_i = 0;

            for (i, user_input) in user_inputs.iter_mut().enumerate() {
                let area = get_nth_input_area(i);
                let click_capture = user_input.click(button, &position, &area);

                if click_capture == Capture::TakeFocus {
                    from_i = i;
                    capture = Some(click_capture);
                }

                if capture.is_some() {
                    break;
                }
            }

            if let Some(capture) = capture {
                if let Capture::TakeFocus = capture {
                    take_focus_from_user_inputs_except(&mut user_inputs, from_i);
                }
                capture?;
            }
        }
        Interaction::Key { virtual_keycode } => {
            let shift = app.keys_held.contains(&VirtualKeyCode::LShift);

            for user_input in user_inputs.drain(0..) {
                let key_capture = user_input.key_press(virtual_keycode, shift);
                if key_capture != KeyCapture::Miss {
                    return Capture::Keyboard(key_capture);
                }
            }
        }
        _ => (),
    }
    Capture::Miss
}

pub fn render_user_inputs(ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, user_inputs: Vec<&mut UserInput>) {
    let mut user_inputs = user_inputs;

    for (i, user_input) in user_inputs.drain(0..).enumerate() {
        let area = get_nth_input_area(i);
        user_input.render(ctx, text_system, font.clone(), &area);
    }
}
