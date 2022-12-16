pub mod behavior;
mod circle;
mod rect;
mod tri;

use self::behavior::TranslateBehavior;
use crate::{
    accumulator::Accumulator,
    algorithm::get_triangle_center,
    approx_instant,
    camera::{Camera, IDENTITY_CAMERA},
    capture::Capture,
    color::Color,
    color_scheme::ColorScheme,
    context_menu::{ContextAction, ContextMenu, ContextMenuOption},
    data::{rect::BorderRadii, CircleData, RectData, TriData},
    interaction::Interaction,
    interaction_options::InteractionOptions,
    render_ctx::RenderCtx,
    selection::{NewSelection, NewSelectionInfo, Selection},
    smooth::{MultiLerp, NoLerp, Smooth},
};
use angular_units::Rad;
use circle::Circle;
use itertools::Itertools;
use lazy_static::lazy_static;
use nalgebra_glm as glm;
use rect::Rect;
use serde::{Deserialize, Serialize};
use slotmap::new_key_type;
use std::{cmp::Ordering, time::Instant};
use tri::Tri;

new_key_type! {
    pub struct SquidRef;
    pub struct SquidLimbRef;
}

#[derive(Serialize, Deserialize)]
enum SquidKind {
    Rect(Rect),
    Circle(Circle),
    Tri(Tri),
}

#[derive(Serialize, Deserialize)]
pub struct Squid {
    name: Option<String>,

    #[serde(with = "approx_instant")]
    created: Instant,

    kind: SquidKind,
}

impl Squid {
    pub fn rect(position: glm::Vec2, size: glm::Vec2, rotation: Rad<f32>, color: Color, radii: f32, is_viewport: bool) -> Self {
        let data = RectData {
            position: MultiLerp::From(position),
            size,
            rotation,
            color: NoLerp(color),
            radii: BorderRadii::new(radii),
            is_viewport,
        };

        Self::rect_from(data)
    }

    pub fn rect_from(data: RectData) -> Self {
        Self {
            name: None,
            created: Instant::now(),
            kind: SquidKind::Rect(Rect {
                mesh: None,
                data: Smooth::new(data, None),
                moving_corner: None,
                opposite_corner_position: None,
                translate_behavior: Default::default(),
                rotating: false,
                rotation_accumulator: Accumulator::new(),
                prescale_size: data.size,
                spread_behavior: Default::default(),
                revolve_behavior: Default::default(),
                dilate_behavior: Default::default(),
            }),
        }
    }

    pub fn circle(position: glm::Vec2, radius: f32, color: Color) -> Self {
        let data = CircleData {
            position: MultiLerp::From(position),
            radius,
            color: NoLerp(color),
            virtual_rotation: Rad(0.0),
        };

        Self::circle_from(data)
    }

    pub fn circle_from(data: CircleData) -> Self {
        Self {
            name: None,
            created: Instant::now(),
            kind: SquidKind::Circle(Circle {
                mesh: None,
                data: Smooth::new(data, None),
                translate_behavior: Default::default(),
                scale_rotating: false,
                rotation_accumulator: Accumulator::new(),
                prescale_size: data.radius,
                spread_behavior: Default::default(),
                revolve_behavior: Default::default(),
                dilate_behavior: Default::default(),
            }),
        }
    }

    pub fn tri(p: [glm::Vec2; 3], rotation: Rad<f32>, color: Color) -> Self {
        let position = get_triangle_center(p);

        let data = TriData {
            p: p.map(|point| MultiLerp::From(point - position)),
            position: MultiLerp::From(position),
            rotation,
            color: NoLerp(color),
        };

        Self::tri_from(data)
    }

    pub fn tri_from(data: TriData) -> Self {
        let p = data.p.map(|point| point.reveal());

        Self {
            name: None,
            created: Instant::now(),
            kind: SquidKind::Tri(Tri {
                mesh: None,
                data: Smooth::new(data, None),
                mesh_p: p,
                moving_point: None,
                translate_behavior: Default::default(),
                rotating: false,
                rotation_accumulator: Accumulator::new(),
                virtual_rotation: Rad(0.0),
                prescale_size: p,
                spread_behavior: Default::default(),
                revolve_behavior: Default::default(),
                dilate_behavior: Default::default(),
            }),
        }
    }

    // Renders squid in regular state
    pub fn render(&mut self, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
        match &mut self.kind {
            SquidKind::Rect(rect) => rect.render(ctx, as_preview),
            SquidKind::Circle(circle) => circle.render(ctx, as_preview),
            SquidKind::Tri(tri) => tri.render(ctx, as_preview),
        }
    }

    // Render additional selection indicators and helpers for when
    // the squid is selected
    pub fn get_selection_points(&self, camera: &Camera, output: &mut Vec<glm::Vec2>) {
        match &self.kind {
            SquidKind::Rect(rect) => {
                let RectData { position, .. } = rect.data.get_animated();
                output.push(camera.apply(&position.reveal()));
                output.push(rect.get_rotate_handle(camera));

                for corner in rect.get_relative_corners() {
                    output.push(camera.apply(&(position.reveal() + corner)));
                }
            }
            SquidKind::Circle(circle) => {
                let CircleData { position, .. } = circle.data.get_animated();
                output.push(camera.apply(&position.reveal()));
                output.push(circle.get_rotate_handle(camera));
            }
            SquidKind::Tri(tri) => {
                let TriData { position, .. } = tri.data.get_animated();

                output.push(camera.apply(&position.reveal()));
                output.push(tri.get_rotate_handle(camera));

                for point in tri.get_animated_screen_points(camera) {
                    output.push(point);
                }
            }
        }
    }

    // Called when squid is selected and has opportunity to capture
    // user interaction
    // Returns if and how the interaction was captured
    pub fn interact(&mut self, interaction: &Interaction, camera: &Camera, _options: &InteractionOptions) -> Capture {
        match &mut self.kind {
            SquidKind::Rect(rect) => rect.interact(interaction, camera),
            SquidKind::Circle(circle) => circle.interact(interaction, camera),
            SquidKind::Tri(tri) => tri.interact(interaction, camera),
        }
    }

    fn translate_behavior(&mut self) -> Option<&mut TranslateBehavior> {
        match &mut self.kind {
            SquidKind::Rect(rect) => Some(&mut rect.translate_behavior),
            SquidKind::Circle(circle) => Some(&mut circle.translate_behavior),
            SquidKind::Tri(tri) => Some(&mut tri.translate_behavior),
        }
    }

    fn reposition_by(&mut self, delta: glm::Vec2) {
        if delta == glm::zero::<glm::Vec2>() {
            return;
        }

        match &mut self.kind {
            SquidKind::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.position = MultiLerp::Linear(new_data.position.reveal() + delta);
                rect.data.set(new_data);
            }
            SquidKind::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.position = MultiLerp::Linear(new_data.position.reveal() + delta);
                circle.data.set(new_data);
            }
            SquidKind::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                new_data.position = MultiLerp::Linear(new_data.position.reveal() + delta);
                tri.data.set(new_data);
            }
        }
    }

    fn rotate_behavior(&mut self) -> Option<&mut Accumulator<Rad<f32>>> {
        match &mut self.kind {
            SquidKind::Rect(rect) => Some(&mut rect.rotation_accumulator),
            SquidKind::Circle(circle) => Some(&mut circle.rotation_accumulator),
            SquidKind::Tri(tri) => Some(&mut tri.rotation_accumulator),
        }
    }

    fn rotate_by(&mut self, delta_theta: Rad<f32>) {
        match &mut self.kind {
            SquidKind::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.rotation += delta_theta;
                rect.data.set(new_data);
            }
            SquidKind::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.virtual_rotation += delta_theta;
                circle.data.set(new_data);
            }
            SquidKind::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                new_data.rotation += delta_theta;
                tri.data.set(new_data);
            }
        }
    }

    // Moves a squid body
    pub fn translate(&mut self, world_delta: &glm::Vec2, options: &InteractionOptions) {
        let delta = if let Some(behavior) = self.translate_behavior() {
            behavior.express(world_delta, options)
        } else {
            glm::zero()
        };

        self.reposition_by(delta);
    }

    // Rotates a squid body
    pub fn rotate(&mut self, mouse_delta_theta: Rad<f32>, options: &InteractionOptions) {
        let delta_theta = self
            .rotate_behavior()
            .and_then(|behavior| behavior.accumulate(&mouse_delta_theta, options.rotation_snapping));

        if let Some(delta_theta) = delta_theta {
            self.rotate_by(delta_theta);
        }
    }

    // Scales a squid body
    pub fn scale(&mut self, total_scale_factor: f32, _options: &InteractionOptions) {
        match &mut self.kind {
            SquidKind::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.size = total_scale_factor * rect.prescale_size;
                rect.data.set(new_data);
                rect.mesh = None;
            }
            SquidKind::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.radius = circle.prescale_size * total_scale_factor;
                circle.data.set(new_data);
            }
            SquidKind::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                new_data.p = tri.prescale_size.map(|axis| MultiLerp::Linear(total_scale_factor * axis));
                tri.data.set(new_data);
            }
        }
    }

    // Spreads a squid body toward/from a point
    pub fn spread(&mut self, current: &glm::Vec2, _options: &InteractionOptions) {
        match &mut self.kind {
            SquidKind::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.position = MultiLerp::Linear(rect.spread_behavior.express(current));
                rect.data.set(new_data);
            }
            SquidKind::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.position = MultiLerp::Linear(circle.spread_behavior.express(current));
                circle.data.set(new_data);
            }
            SquidKind::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                new_data.position = MultiLerp::Linear(tri.spread_behavior.express(current));
                tri.data.set(new_data);
            }
        }
    }

    // Revolves a squid body around point
    pub fn revolve(&mut self, current: &glm::Vec2, options: &InteractionOptions) {
        match &mut self.kind {
            SquidKind::Rect(rect) => {
                if let Some(expression) = rect.revolve_behavior.express(current, options) {
                    let mut new_data = *rect.data.get_real();
                    new_data.position = MultiLerp::Circle(expression.apply_origin_rotation_to_center(), expression.origin);
                    new_data.rotation += expression.delta_object_rotation;
                    rect.data.set(new_data);
                }
            }
            SquidKind::Circle(circle) => {
                if let Some(expression) = circle.revolve_behavior.express(current, options) {
                    let mut new_data = *circle.data.get_real();
                    new_data.position = MultiLerp::Circle(expression.apply_origin_rotation_to_center(), expression.origin);
                    new_data.virtual_rotation += expression.delta_object_rotation;
                    circle.data.set(new_data);
                }
            }
            SquidKind::Tri(tri) => {
                if let Some(expression) = tri.revolve_behavior.express(current, options) {
                    let mut new_data = *tri.data.get_real();
                    new_data.position = MultiLerp::Circle(expression.apply_origin_rotation_to_center(), expression.origin);
                    new_data.rotation += expression.delta_object_rotation;
                    tri.data.set(new_data);
                }
            }
        }
    }

    // Dilates a squid body toward/from a point
    pub fn dilate(&mut self, current: &glm::Vec2, _options: &InteractionOptions) {
        match &mut self.kind {
            SquidKind::Rect(rect) => {
                let expression = rect.dilate_behavior.express(current);
                let mut new_data = *rect.data.get_real();
                new_data.position = MultiLerp::Linear(expression.position);
                new_data.size = expression.total_scale_factor * rect.prescale_size;
                rect.data.set(new_data);
                rect.mesh = None;
            }
            SquidKind::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                let expression = circle.dilate_behavior.express(current);
                new_data.position = MultiLerp::Linear(expression.position);
                new_data.radius = circle.prescale_size * expression.total_scale_factor;
                circle.data.set(new_data);
            }
            SquidKind::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                let expression = tri.dilate_behavior.express(current);
                new_data.position = MultiLerp::Linear(expression.position);
                new_data.p = tri.prescale_size.map(|axis| MultiLerp::Linear(expression.total_scale_factor * axis));
                tri.data.set(new_data);
            }
        }
    }

    // Attempts to get a selection for this squid or a selection for a limb of this squid
    // under the point (x, y)
    pub fn try_select(&self, underneath: glm::Vec2, camera: &Camera, self_reference: SquidRef) -> Option<NewSelection> {
        match &self.kind {
            SquidKind::Rect(rect) => {
                if rect.is_point_over(underneath, camera) {
                    return Some(NewSelection {
                        selection: Selection::new(self_reference, None),
                        info: NewSelectionInfo {
                            color: Some(*rect.data.get_real().color),
                        },
                    });
                }
            }
            SquidKind::Circle(circle) => {
                if circle.is_point_over(underneath, camera) {
                    return Some(NewSelection {
                        selection: Selection::new(self_reference, None),
                        info: NewSelectionInfo {
                            color: Some(*circle.data.get_real().color),
                        },
                    });
                }
            }
            SquidKind::Tri(tri) => {
                if tri.is_point_over(underneath, camera) {
                    return Some(NewSelection {
                        selection: Selection::new(self_reference, None),
                        info: NewSelectionInfo {
                            color: Some(*tri.data.get_real().color),
                        },
                    });
                }
            }
        }

        None
    }

    // Performs selection
    pub fn select(&mut self) {
        if let Some(behavior) = self.translate_behavior() {
            behavior.moving = true;
        }
    }

    pub fn is_point_over(&self, mouse_position: glm::Vec2, camera: &Camera) -> bool {
        match &self.kind {
            SquidKind::Rect(rect) => rect.is_point_over(mouse_position, camera),
            SquidKind::Circle(circle) => circle.is_point_over(mouse_position, camera),
            SquidKind::Tri(tri) => tri.is_point_over(mouse_position, camera),
        }
    }

    pub fn as_viewport(&self) -> Option<RectData> {
        match &self.kind {
            SquidKind::Rect(rect) if rect.data.get_real().is_viewport => {
                return Some(*rect.data.get_real());
            }
            _ => None,
        }
    }

    pub fn build(&self, document: &mut svg::Document) {
        match &self.kind {
            SquidKind::Rect(rect) => rect.build(document),
            SquidKind::Circle(circle) => circle.build(document),
            SquidKind::Tri(tri) => tri.build(document),
        }
    }

    // Attempt to get a context menu for if a quid is underneath a point
    pub fn try_context_menu(&self, underneath: glm::Vec2, camera: &Camera, _self_reference: SquidRef, color_scheme: &ColorScheme) -> Option<ContextMenu> {
        if self.is_point_over(underneath, camera) {
            Some(common_context_menu(underneath, color_scheme))
        } else {
            None
        }
    }

    // Attempts to set the color of a squid
    pub fn set_color(&mut self, color: Color) {
        match &mut self.kind {
            SquidKind::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.color = NoLerp(color);
                rect.data.set(new_data);
            }
            SquidKind::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.color = NoLerp(color);
                circle.data.set(new_data);
            }
            SquidKind::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                new_data.color = NoLerp(color);
                tri.data.set(new_data);
            }
        }
    }

    // Duplicates a squid
    pub fn duplicate(&self, offset: &glm::Vec2) -> Squid {
        match &self.kind {
            SquidKind::Rect(rect) => {
                let mut real = *rect.data.get_real();
                real.position = MultiLerp::From(real.position.reveal() + offset);
                Squid::rect_from(real)
            }
            SquidKind::Circle(circle) => {
                let mut real = *circle.data.get_real();
                real.position = MultiLerp::From(real.position.reveal() + offset);
                Squid::circle_from(real)
            }
            SquidKind::Tri(tri) => {
                let mut real = *tri.data.get_real();
                real.position = MultiLerp::From(real.position.reveal() + offset);
                Squid::tri_from(real)
            }
        }
    }

    // Signals to the squid to initiate a certain user action
    pub fn initiate(&mut self, initiation: Initiation) {
        match &mut self.kind {
            SquidKind::Rect(rect) => rect.initiate(initiation),
            SquidKind::Circle(circle) => circle.initiate(initiation),
            SquidKind::Tri(tri) => tri.initiate(initiation),
        }
    }

    // Gets center of a squid
    pub fn get_center(&self) -> glm::Vec2 {
        use SquidKind::*;

        match &self.kind {
            Rect(rect) => rect.data.get_animated().position.reveal(),
            Circle(circle) => circle.data.get_animated().position.reveal(),
            Tri(tri) => tri.data.get_animated().position.reveal(),
        }
    }

    // Opaque name getter/setter
    pub fn get_name(&self) -> &str {
        self.name.as_deref().unwrap_or_else(|| match &self.kind {
            SquidKind::Rect(rect) => {
                if rect.data.get_animated().is_viewport {
                    "Unnamed Viewport"
                } else {
                    "Unnamed Rect"
                }
            }
            SquidKind::Circle(_) => "Unnamed Circle",
            SquidKind::Tri(_) => "Unnamed Tri",
        })
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    // Returns the world positions of all "opaque" handles (aka handles that will take priority over new selections)
    pub fn get_opaque_handles(&self) -> Vec<glm::Vec2> {
        match &self.kind {
            SquidKind::Rect(rect) => {
                let mut handles = rect.get_relative_corners();
                handles.push(rect.get_rotate_handle(&IDENTITY_CAMERA));
                handles
            }
            SquidKind::Circle(circle) => {
                vec![circle.get_rotate_handle(&IDENTITY_CAMERA)]
            }
            SquidKind::Tri(tri) => {
                let data = tri.data.get_animated();
                let position = data.position.reveal();

                data.p
                    .iter()
                    .map(|point| point.reveal() + position)
                    .chain(std::iter::once(tri.get_rotate_handle(&IDENTITY_CAMERA)))
                    .collect_vec()
            }
        }
    }
}

impl PartialOrd for Squid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.created.partial_cmp(&other.created)
    }
}

impl PartialEq for Squid {
    fn eq(&self, other: &Self) -> bool {
        self.created.eq(&other.created)
    }
}

impl Eq for Squid {}

impl Ord for Squid {
    fn cmp(&self, other: &Self) -> Ordering {
        self.created.cmp(&other.created)
    }
}

impl Clone for Squid {
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
    Dilate { point: glm::Vec2, center: glm::Vec2 },
}

pub const HANDLE_RADIUS: f32 = 8.0;

lazy_static! {
    pub static ref HANDLE_SIZE: glm::Vec2 = glm::vec2(HANDLE_RADIUS, HANDLE_RADIUS);
}

pub fn common_context_menu(underneath: glm::Vec2, color_scheme: &ColorScheme) -> ContextMenu {
    use ContextAction::*;

    ContextMenu::new(
        underneath,
        vec![
            ContextMenuOption::new("Delete", "X", DeleteSelected),
            ContextMenuOption::new("Duplicate", "Shift+D", DuplicateSelected),
            ContextMenuOption::new("Grab", "G", GrabSelected),
            ContextMenuOption::new("Rotate", "R", RotateSelected),
            ContextMenuOption::new("Scale", "S", ScaleSelected),
            ContextMenuOption::new("Collectively", "C", Collectively),
        ],
        color_scheme.dark_ribbon,
    )
}

pub struct PreviewParams {
    pub position: glm::Vec2,
    pub radius: f32,
}
