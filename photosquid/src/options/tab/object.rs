use super::Tab;
use crate::{app::App, capture::Capture, interaction::Interaction, ocean::Ocean, render_ctx::RenderCtx, selection::Selection};
use glium_text_rusttype::{FontTexture, TextSystem};
use std::rc::Rc;

pub struct Object {}

impl Object {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tab for Object {
    fn interact(&mut self, _interaction: Interaction, _app: &mut App) -> Capture {
        Capture::Miss
    }

    fn render(&mut self, _ctx: &mut RenderCtx, _text_system: &TextSystem, _font: Rc<FontTexture>, _ocean: &mut Ocean, _selections: &[Selection]) {}
}
