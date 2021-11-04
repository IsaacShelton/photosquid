use super::Tab;
use crate::{
    app::{selection_contains, ApplicationState},
    capture::Capture,
    color::Color,
    interaction::Interaction,
    ocean::{Ocean, Selection},
    render_ctx::RenderCtx,
    text_helpers,
};
use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Layers {}

impl Layers {
    pub fn new() -> Box<dyn Tab> {
        Box::new(Self {})
    }

    fn render_layer(
        &mut self,
        ctx: &mut RenderCtx,
        text_system: &TextSystem,
        font: Rc<FontTexture>,
        start_x: f32,
        y: &mut f32,
        ocean: &mut Ocean,
        selections: &Vec<Selection>,
        layer_index: usize,
    ) {
        let mut text_display: Option<TextDisplay<Rc<FontTexture>>>;

        // Use '#[allow(deprecated)]' to silence warning when manually accessing
        // internal fields of 'Ocean' struct
        #[allow(deprecated)]
        let layer = &ocean.layers[layer_index];

        let layer_label_y = *y;
        let mut all_selected = layer.squids.len() != 0;
        *y += 30.0;

        // This should be valid without direct access, due to the implementation
        // of Ocean::get_mut(), but Rust isn't having it,
        // so we'll have reach into the actual fields of 'ocean' in order for it
        // to be allowed
        for squid_reference in layer.squids.iter() {
            // Use '#[allow(deprecated)]' to silence warning when manually accessing
            // internal fields of 'Ocean' struct
            #[allow(deprecated)]
            let maybe_squid = ocean.squids.get_mut(*squid_reference);

            if let Some(squid) = maybe_squid {
                let color = if selection_contains(selections, *squid_reference) {
                    ctx.color_scheme.foreground
                } else {
                    all_selected = false;
                    Color::from_hex("#777777")
                };

                text_display = None;
                text_helpers::draw_text(
                    &mut text_display,
                    text_system,
                    font.clone(),
                    squid.get_name(),
                    &glm::vec2(start_x + 24.0, *y),
                    ctx,
                    color,
                );
                *y += 30.0;
            }
        }

        text_display = None;
        text_helpers::draw_text(
            &mut text_display,
            text_system,
            font.clone(),
            layer.get_name(),
            &glm::vec2(start_x, layer_label_y),
            ctx,
            if all_selected {
                ctx.color_scheme.foreground
            } else {
                Color::from_hex("#555555")
            },
        );
    }
}

impl Tab for Layers {
    fn interact(&mut self, _interaction: Interaction, _app: &mut ApplicationState) -> Capture {
        Capture::Miss
    }

    fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, ocean: &mut Ocean, selections: &Vec<Selection>) {
        let start_x = ctx.width - 256.0 + 16.0;
        let mut y = 100.0;

        for i in 0..ocean.get_layers().len() {
            self.render_layer(ctx, text_system, font.clone(), start_x, &mut y, ocean, selections, i);
        }
    }
}
