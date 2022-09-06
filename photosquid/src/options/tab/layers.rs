use super::Tab;
use crate::{
    app::App,
    capture::Capture,
    color::Color,
    draw_text::draw_text,
    interaction::{ClickInteraction, Interaction},
    layer::Layer,
    ocean::Ocean,
    render_ctx::RenderCtx,
    selection::{selection_contains, Selection},
    squid::{PreviewParams, SquidRef},
};
use glium::glutin::event::MouseButton;
use glium_text_rusttype::{FontTexture, TextSystem};
use nalgebra_glm as glm;
use std::rc::Rc;

enum Entry {
    LayerName(LayerName),
    Child(Child),
}

struct LayerName {
    name: String,
    y: f32,
}

struct Child {
    squid: SquidRef,
    y: f32,
}

pub struct Layers {
    entries: Vec<Entry>,
}

impl Layers {
    const SMALL_STRIP_HEIGHT: f32 = 30.0;
    const TAB_WIDTH: f32 = 256.0;

    pub fn new() -> Self {
        Self { entries: vec![] }
    }

    fn update(&mut self, layers: &[Layer]) {
        let mut entries: Vec<Entry> = Vec::new();
        let mut y = 100.0;

        for layer in layers.iter() {
            entries.push(Entry::LayerName(LayerName {
                name: layer.get_name().into(),
                y,
            }));

            y += Self::SMALL_STRIP_HEIGHT;

            for squid_ref in &layer.squids {
                entries.push(Entry::Child(Child { squid: *squid_ref, y }));
                y += Self::SMALL_STRIP_HEIGHT;
            }
        }

        self.entries = entries;
    }

    fn get_clicked_entry(&self, mouse: &glm::Vec2, window_dimensions: &glm::Vec2) -> Option<&Entry> {
        if mouse.x < window_dimensions.x - Layers::TAB_WIDTH {
            return None;
        }

        for entry in &self.entries {
            match entry {
                Entry::Child(Child { y, .. }) | Entry::LayerName(LayerName { y, .. }) => {
                    if mouse.y >= *y - 0.5 * Self::SMALL_STRIP_HEIGHT && mouse.y < y - 0.5 * Self::SMALL_STRIP_HEIGHT + Self::SMALL_STRIP_HEIGHT {
                        return Some(entry);
                    }
                }
            }
        }

        None
    }
}

impl Tab for Layers {
    fn interact(&mut self, interaction: Interaction, app: &mut App) -> Capture {
        match interaction {
            Interaction::Click(ClickInteraction {
                button: MouseButton::Left,
                modifiers,
                position,
                ..
            }) => {
                if position.x >= app.dimensions.x - Layers::TAB_WIDTH {
                    let clicked: Option<&Entry> = self.get_clicked_entry(&position, &app.dimensions);

                    match clicked {
                        Some(Entry::Child(Child { squid, .. })) => {
                            if !modifiers.shift() {
                                app.selections.clear();
                            }

                            app.selections.push(Selection {
                                squid_id: *squid,
                                limb_id: None,
                            });
                        }
                        Some(Entry::LayerName(_)) => (),
                        None => (),
                    }
                }
            }
            _ => (),
        }

        Capture::Miss
    }

    fn render(&mut self, ctx: &mut RenderCtx, text_system: &TextSystem, font: Rc<FontTexture>, ocean: &mut Ocean, selections: &[Selection]) {
        self.update(ocean.get_layers());

        const LEFT_MARGIN: f32 = 16.0;

        let left = ctx.width - Self::TAB_WIDTH + LEFT_MARGIN;

        for entry in &self.entries {
            match entry {
                Entry::LayerName(layer_name) => {
                    // Draw layer name
                    draw_text(
                        &mut None,
                        text_system,
                        font.clone(),
                        &layer_name.name,
                        &glm::vec2(left, layer_name.y),
                        ctx,
                        Color::from_hex("#555555"),
                    );
                }
                Entry::Child(child) => {
                    if let Some(squid) = ocean.get_mut(child.squid) {
                        const PREVIEW_PADDING: f32 = 4.0;
                        const PREVIEW_RADIUS: f32 = 8.0;
                        const PREVIEW_SIZE_WITH_PADDING: f32 = 2.0 * PREVIEW_PADDING + 2.0 * PREVIEW_RADIUS;

                        // Draw squid preview
                        squid.render(
                            ctx,
                            Some(PreviewParams {
                                position: glm::vec2(left + PREVIEW_PADDING, child.y - PREVIEW_PADDING),
                                radius: PREVIEW_RADIUS,
                            }),
                        );

                        // Choose text color
                        let color = if selection_contains(selections, child.squid) {
                            ctx.color_scheme.foreground
                        } else {
                            Color::from_hex("#777777")
                        };

                        // Draw squid name
                        draw_text(
                            &mut None,
                            text_system,
                            font.clone(),
                            squid.get_name(),
                            &glm::vec2(left + PREVIEW_SIZE_WITH_PADDING, child.y),
                            ctx,
                            color,
                        );
                    }
                }
            }
        }
    }
}
