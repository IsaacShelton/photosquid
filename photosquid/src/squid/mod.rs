pub mod rect;

use crate::{
    color::Color,
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    ocean::NewSelection,
    render_ctx::RenderCtx,
    tool::{Capture, Interaction},
};
use glium::Display;
use nalgebra_glm as glm;
use slotmap::new_key_type;
use std::cmp::{Ord, Ordering, PartialOrd};

pub use rect::Rect;

new_key_type! {
    pub struct SquidRef;
    pub struct SquidLimbRef;
}

pub trait Squid {
    // Renders squid in regular state
    fn render(&self, ctx: &mut RenderCtx);

    // Render additional selection indicators and helpers for when
    // the squid is selected
    fn render_selected_indication(&self, ctx: &mut RenderCtx);

    // Called when squid is selected and has opportunity to capture
    // user interaction
    // Returns if and how the interaction was captured
    fn interact(&mut self, interaction: &Interaction, camera: &glm::Vec2) -> Capture;

    // Returns whether a point is over this squid
    fn is_point_over(&self, underneath: &glm::Vec2, camera: &glm::Vec2) -> bool;

    // Moves a squid body
    fn translate(&mut self, delta: &glm::Vec2);

    // Rotates a squid body
    fn rotate(&mut self, delta_theta: f32);

    // Attempts to get a selection for this squid or a selection for a limb of this squid
    // under the point (x, y)
    fn try_select(&self, underneath: &glm::Vec2, camera: &glm::Vec2, self_reference: SquidRef) -> Option<NewSelection>;

    // Attempt to get a context menu for if a quid is underneath a point
    fn try_context_menu(&self, underneath: &glm::Vec2, camera: &glm::Vec2, self_reference: SquidRef, color_scheme: &ColorScheme) -> Option<ContextMenu>;

    // Attempts to set the color of a squid
    fn set_color(&mut self, color: Color);

    // Duplicates a squid
    fn duplicate(&self, offset: &glm::Vec2, display: &Display) -> Box<dyn Squid>;

    fn get_creation_time(&self) -> std::time::Instant;
}

impl PartialOrd for dyn Squid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.get_creation_time().partial_cmp(&other.get_creation_time())
    }
}

impl PartialEq for dyn Squid {
    fn eq(&self, other: &Self) -> bool {
        self.get_creation_time().eq(&other.get_creation_time())
    }
}

impl Eq for dyn Squid {}

impl Ord for dyn Squid {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get_creation_time().cmp(&other.get_creation_time())
    }
}
