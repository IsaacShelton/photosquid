use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};
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
