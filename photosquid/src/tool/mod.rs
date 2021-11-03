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

pub fn get_nth_input_area(n: i32) -> AABB {
    TextInput::standard_area(&glm::vec2(64.0, 128.0 + n as f32 * 96.0))
}

pub fn interact_text_inputs(text_inputs: Vec<&mut TextInput>, interaction: Interaction, app: &mut ApplicationState) -> Capture {
    let mut text_inputs = text_inputs;

    match interaction {
        Interaction::Click { button, position } => {
            for (i, text_input) in text_inputs.drain(0..).enumerate() {
                let area = get_nth_input_area(i as i32);
                text_input.click(button, &position, &area)?;
            }
        }
        Interaction::Key { virtual_keycode } => {
            let shift = app.keys_held.contains(&VirtualKeyCode::LShift);

            for text_input in text_inputs.drain(0..) {
                let key_capture = text_input.key_press(virtual_keycode, &app.numeric_mappings, shift);
                if key_capture != KeyCapture::Miss {
                    return Capture::Keyboard(key_capture);
                }
            }
        }
        _ => (),
    }
    Capture::Miss
}

pub fn render_text_inputs(ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, text_inputs: Vec<&mut TextInput>) {
    let mut text_inputs = text_inputs;

    for (i, text_input) in text_inputs.drain(0..).enumerate() {
        let area = get_nth_input_area(i as i32);
        text_input.render(ctx, text_system, font.clone(), &area);
    }
}
