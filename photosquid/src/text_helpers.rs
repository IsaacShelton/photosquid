use crate::{color::Color, render_ctx::RenderCtx};
use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub fn get_or_make_display<'a>(
    persistent: &'a mut Option<TextDisplay<Rc<FontTexture>>>,
    text_system: &TextSystem,
    font: Rc<FontTexture>,
    text: &str,
) -> &'a TextDisplay<Rc<FontTexture>> {
    if persistent.is_none() {
        let text_display = TextDisplay::new(text_system, font, text);
        *persistent = Some(text_display);
    }
    return persistent.as_ref().unwrap();
}

pub fn draw_text<'a>(
    persistent: &'a mut Option<TextDisplay<Rc<FontTexture>>>,
    text_system: &TextSystem,
    font: Rc<FontTexture>,
    text: &str,
    location: &glm::Vec2,
    ctx: &mut RenderCtx,
    color: Color,
) {
    get_or_make_display(persistent, text_system, font, text);

    let text_display = persistent.as_ref().unwrap();
    let transformation = glm::translation(&glm::vec3(location.x, location.y, 0.0));
    let transformation = glm::scale(&transformation, &glm::vec3(16.0, -16.0, 0.0));
    let matrix = ctx.projection * transformation;
    ctx.draw_text(&text_display, text_system, matrix, color.into()).unwrap();
}

pub fn draw_text_centered<'a>(
    persistent: &'a mut Option<TextDisplay<Rc<FontTexture>>>,
    text_system: &TextSystem,
    font: Rc<FontTexture>,
    text: &str,
    location: &glm::Vec2,
    ctx: &mut RenderCtx,
    color: Color,
) {
    get_or_make_display(persistent, text_system, font, text);

    let text_display = persistent.as_ref().unwrap();
    let transformation = glm::translation(&glm::vec3(location.x - 0.5 * text_display.get_width() * 16.0, location.y, 0.0));
    let transformation = glm::scale(&transformation, &glm::vec3(16.0, -16.0, 0.0));
    let matrix = ctx.projection * transformation;
    ctx.draw_text(&text_display, text_system, matrix, color.into()).unwrap();
}
