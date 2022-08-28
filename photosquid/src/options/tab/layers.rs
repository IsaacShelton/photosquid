use super::Tab;
use crate::{
    app::App,
    capture::Capture,
    color::Color,
    interaction::Interaction,
    ocean::Ocean,
    render_ctx::RenderCtx,
    selection::{selection_contains, Selection},
    squid::PreviewParams,
    text_helpers,
};
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

pub struct Layers {}

impl Layers {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tab for Layers {
    fn interact(&mut self, _interaction: Interaction, _app: &mut App) -> Capture {
        Capture::Miss
    }

    fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, ocean: &mut Ocean, selections: &[Selection]) {
        const LAYERS_TAB_WIDTH: f32 = 256.0;
        const LEFT_MARGIN: f32 = 16.0;

        let mut y = 100.0;

        let (squids, layers) = ocean.get_squids_and_layers();
        let left = ctx.width - LAYERS_TAB_WIDTH + LEFT_MARGIN;

        for layer in layers.iter() {
            const SMALL_STRIP_HEIGHT: f32 = 30.0;

            // Draw layer name
            text_helpers::draw_text(
                &mut None,
                text_system,
                font.clone(),
                layer.get_name(),
                &glm::vec2(left, y),
                ctx,
                Color::from_hex("#555555"),
            );

            // Move draw area down
            y += SMALL_STRIP_HEIGHT;

            for squid_ref in layer.squids.iter() {
                if let Some(squid) = squids.get_mut(*squid_ref) {
                    const PREVIEW_PADDING: f32 = 4.0;
                    const PREVIEW_RADIUS: f32 = 8.0;
                    const PREVIEW_SIZE_WITH_PADDING: f32 = 2.0 * PREVIEW_PADDING + 2.0 * PREVIEW_RADIUS;

                    // Draw squid preview
                    squid.render(
                        ctx,
                        Some(PreviewParams {
                            position: glm::vec2(left + PREVIEW_PADDING, y - PREVIEW_PADDING),
                            radius: PREVIEW_RADIUS,
                        }),
                    );

                    // Choose text color
                    let color = if selection_contains(selections, *squid_ref) {
                        ctx.color_scheme.foreground
                    } else {
                        Color::from_hex("#777777")
                    };

                    // Draw squid name
                    text_helpers::draw_text(
                        &mut None,
                        text_system,
                        font.clone(),
                        squid.get_name(),
                        &glm::vec2(left + PREVIEW_SIZE_WITH_PADDING, y),
                        ctx,
                        color,
                    );

                    // Move draw area down
                    y += SMALL_STRIP_HEIGHT;
                }
            }
        }
    }
}
