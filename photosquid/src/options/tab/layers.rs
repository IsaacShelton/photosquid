use super::Tab;
use crate::{
    app::{selection_contains, ApplicationState},
    capture::Capture,
    color::Color,
    interaction::Interaction,
    ocean::{Ocean, Selection},
    render_ctx::RenderCtx,
    squid::PreviewParams,
    text_helpers,
};
use glium_text_rusttype::{FontTexture, TextDisplay, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Layers {}

impl Layers {
    pub fn new() -> Self {
        Self {}
    }

    #[allow(clippy::too_many_arguments)]
    fn render_layer(
        &mut self,
        ctx: &mut RenderCtx,
        text_system: &TextSystem,
        font: Rc<FontTexture>,
        y: &mut f32,
        ocean: &mut Ocean,
        selections: &[Selection],
        layer_index: usize,
    ) {
        let start_x = ctx.width - 256.0 + 16.0;
        let mut text_display: Option<TextDisplay<Rc<FontTexture>>> = None;

        // Use '#[allow(deprecated)]' to silence warning when manually accessing
        // internal fields of 'Ocean' struct
        #[allow(deprecated)]
        let layer = &ocean.layers[layer_index];

        // Draw layer name
        text_helpers::draw_text(
            &mut text_display,
            text_system,
            font.clone(),
            layer.get_name(),
            &glm::vec2(start_x, *y),
            ctx,
            Color::from_hex("#555555"),
        );

        *y += 30.0;

        // This should be valid without direct access, due to the implementation
        // of Ocean::get_mut(), but Rust isn't having it,
        // so we'll have reach into the actual fields of 'ocean' in order for it
        // to be allowed
        for squid_reference in &layer.squids {
            // Use '#[allow(deprecated)]' to silence warning when manually accessing
            // internal fields of 'Ocean' struct
            #[allow(deprecated)]
            let maybe_squid = ocean.squids.get_mut(*squid_reference);

            if let Some(squid) = maybe_squid {
                let preview = Some(PreviewParams {
                    position: glm::vec2(start_x + 4.0, *y - 4.0),
                    size: 8.0,
                });
                squid.render(ctx, preview);

                let color = if selection_contains(selections, *squid_reference) {
                    ctx.color_scheme.foreground
                } else {
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
    }
}

impl Tab for Layers {
    fn interact(&mut self, _interaction: Interaction, _app: &mut ApplicationState) -> Capture {
        Capture::Miss
    }

    fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, ocean: &mut Ocean, selections: &[Selection]) {
        let mut y = 100.0;

        for i in 0..ocean.get_layers().len() {
            self.render_layer(ctx, text_system, font.clone(), &mut y, ocean, selections, i);
        }
    }
}
