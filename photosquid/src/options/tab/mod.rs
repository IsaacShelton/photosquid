pub mod layers;
pub mod object;

use crate::{app::App, capture::Capture, interaction::Interaction, ocean::Ocean, render_ctx::RenderCtx, selection::Selection};

use glium_text_rusttype::{FontTexture, TextSystem};
use slotmap::new_key_type;
use std::rc::Rc;

pub use layers::Layers;
pub use object::Object;

new_key_type! { pub struct TabRef; }

pub trait Tab {
    fn interact(&mut self, interaction: Interaction, app: &mut App) -> Capture;

    fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, ocean: &mut Ocean, selections: &[Selection]);
}
