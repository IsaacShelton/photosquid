pub mod behavior;
pub mod circle;
pub mod rect;
pub mod tri;

use crate::{
    capture::Capture,
    color::Color,
    color_scheme::ColorScheme,
    context_menu::{ContextAction, ContextMenu, ContextMenuOption},
    interaction::Interaction,
    interaction_options::InteractionOptions,
    math_helpers::angle_difference,
    render_ctx::RenderCtx,
    selection::NewSelection,
};
use angular_units::Rad;
use nalgebra_glm as glm;
use slotmap::new_key_type;
use std::{
    cmp::{Ord, Ordering, PartialOrd},
    time::Instant,
};

pub use circle::Circle;
pub use rect::Rect;
pub use tri::Tri;

new_key_type! {
    pub struct SquidRef;
    pub struct SquidLimbRef;
}

pub trait Squid {
    // Renders squid in regular state
    fn render(&mut self, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>);

    // Render additional selection indicators and helpers for when
    // the squid is selected
    fn render_selected_indication(&self, ctx: &mut RenderCtx);

    // Called when squid is selected and has opportunity to capture
    // user interaction
    // Returns if and how the interaction was captured
    fn interact(&mut self, interaction: &Interaction, camera: &glm::Vec2, options: &InteractionOptions) -> Capture;

    // Returns whether a point is over this squid
    fn is_point_over(&self, underneath: &glm::Vec2, camera: &glm::Vec2) -> bool;

    // Moves a squid body
    fn translate(&mut self, delta: &glm::Vec2, options: &InteractionOptions);

    // Rotates a squid body
    fn rotate(&mut self, delta_theta: Rad<f32>, options: &InteractionOptions);

    // Scales a squid body
    fn scale(&mut self, total_scale_factor: f32, options: &InteractionOptions);

    // Spreads a squid body toward/from a point
    fn spread(&mut self, current: &glm::Vec2, options: &InteractionOptions);

    // Revolves a squid body around point
    fn revolve(&mut self, current: &glm::Vec2, options: &InteractionOptions);

    // Attempts to get a selection for this squid or a selection for a limb of this squid
    // under the point (x, y)
    fn try_select(&self, underneath: &glm::Vec2, camera: &glm::Vec2, self_reference: SquidRef) -> Option<NewSelection>;

    // Performs selection
    fn select(&mut self);

    // Attempt to get a context menu for if a quid is underneath a point
    fn try_context_menu(&self, underneath: &glm::Vec2, camera: &glm::Vec2, self_reference: SquidRef, color_scheme: &ColorScheme) -> Option<ContextMenu>;

    // Attempts to set the color of a squid
    fn set_color(&mut self, color: Color);

    // Duplicates a squid
    fn duplicate(&self, offset: &glm::Vec2) -> Box<dyn Squid>;

    // Gets the creation time of a squid (used for ordering)
    fn get_creation_time(&self) -> Instant;

    // Signals to the squid to initiate a certain user action
    fn initiate(&mut self, initiation: Initiation);

    // Gets center of a squid
    fn get_center(&self) -> glm::Vec2;

    // Opaque name getter/setter
    fn get_name(&self) -> &str;
    fn set_name(&mut self, name: String);

    // Returns the world positions of all "opaque" handles (aka handles that will take priority over new selections)
    fn get_opaque_handles(&self) -> Vec<glm::Vec2>;
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

impl Clone for Box<dyn Squid> {
    fn clone(&self) -> Self {
        self.duplicate(&glm::zero())
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Initiation {
    Translate,
    Rotate,
    Scale,
    Spread { point: glm::Vec2, center: glm::Vec2 },
    Revolve { point: glm::Vec2, center: glm::Vec2 },
}

pub const HANDLE_RADIUS: f32 = 8.0;

pub fn common_context_menu(underneath: &glm::Vec2, color_scheme: &ColorScheme) -> ContextMenu {
    let delete = ContextMenuOption::new("Delete".to_string(), "X".to_string(), ContextAction::DeleteSelected);
    let duplicate = ContextMenuOption::new("Duplicate".to_string(), "Shift+D".to_string(), ContextAction::DuplicateSelected);
    let grab = ContextMenuOption::new("Grab".to_string(), "G".to_string(), ContextAction::GrabSelected);
    let rotate = ContextMenuOption::new("Rotate".to_string(), "R".to_string(), ContextAction::RotateSelected);
    let scale = ContextMenuOption::new("Scale".to_string(), "S".to_string(), ContextAction::ScaleSelected);
    let collectively = ContextMenuOption::new("Collectively".to_string(), "C".to_string(), ContextAction::Collectively);
    ContextMenu::new(
        *underneath,
        vec![delete, duplicate, grab, rotate, scale, collectively],
        color_scheme.dark_ribbon,
    )
}

pub fn get_point_delta_rotation(screen_position: &glm::Vec2, mouse_position: &glm::Vec2, old_rotation: Rad<f32>) -> Rad<f32> {
    let new_rotation = Rad(-1.0 * (mouse_position.y - screen_position.y).atan2(mouse_position.x - screen_position.x));
    angle_difference(old_rotation, new_rotation)
}

pub struct PreviewParams {
    pub position: glm::Vec2,
    pub size: f32,
}
