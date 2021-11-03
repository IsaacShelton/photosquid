use super::Tab;
use crate::{app::ApplicationState, capture::Capture, interaction::Interaction, render_ctx::RenderCtx};
use glium_text_rusttype::{FontTexture, TextSystem};
use std::rc::Rc;

pub struct Object {}

impl Object {
    pub fn new() -> Box<dyn Tab> {
        Box::new(Self {})
    }
}

impl Tab for Object {
    fn interact(&mut self, _interaction: Interaction, _app: &mut ApplicationState) -> Capture {
        Capture::Miss
    }

    fn render(&mut self, _ctx: &mut RenderCtx, _text_system: &TextSystem, _font: Rc<FontTexture>) {
        println!("render");
    }
}
