pub mod layers;
pub mod object;

use crate::{
    app::ApplicationState,
    capture::Capture,
    interaction::Interaction,
    ocean::{Ocean, Selection},
    render_ctx::RenderCtx,
};
use glium_text_rusttype::{FontTexture, TextSystem};
use slotmap::new_key_type;
use std::rc::Rc;

pub use layers::Layers;
pub use object::Object;

new_key_type! { pub struct TabKey; }

pub trait Tab {
    fn interact(&mut self, interaction: Interaction, app: &mut ApplicationState) -> Capture;

    fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, ocean: &mut Ocean, selections: &[Selection]);
}
