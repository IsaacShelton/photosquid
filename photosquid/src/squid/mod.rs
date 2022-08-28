pub mod behavior;
mod circle;
mod rect;
mod tri;

use self::behavior::{DilateBehavior, SpreadBehavior, TranslateBehavior};
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
enum SquidData {
    Rect(Rect),
    Circle(Circle),
    Tri(Tri),
}

#[derive(Serialize, Deserialize)]
pub struct Squid {
    name: Option<String>,

    #[serde(with = "approx_instant")]
    created: Instant,

    data: SquidData,
}

impl Squid {
    pub fn rect(position: glm::Vec2, size: glm::Vec2, rotation: Rad<f32>, color: Color, radii: f32) -> Self {
        let data = RectData {
            position: MultiLerp::From(position),
            size,
            rotation,
            color: NoLerp(color),
            radii: BorderRadii::new(radii),
        };

        Self::rect_from(data)
    }

    pub fn rect_from(data: RectData) -> Self {
        Self {
            name: None,
            created: Instant::now(),
            data: SquidData::Rect(Rect {
                mesh: None,
                data: Smooth::new(data, None),
                moving_corner: None,
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
            data: SquidData::Circle(Circle {
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

    pub fn tri(p1: glm::Vec2, p2: glm::Vec2, p3: glm::Vec2, rotation: Rad<f32>, color: Color) -> Self {
        let position = get_triangle_center(p1, p2, p3);

        let data = TriData {
            p1: MultiLerp::From(p1 - position),
            p2: MultiLerp::From(p2 - position),
            p3: MultiLerp::From(p3 - position),
            position: MultiLerp::From(position),
            rotation,
            color: NoLerp(color),
        };

        Self::tri_from(data)
    }

    pub fn tri_from(data: TriData) -> Self {
        let p1 = data.p1.reveal();
        let p2 = data.p2.reveal();
        let p3 = data.p3.reveal();

        Self {
            name: None,
            created: Instant::now(),
            data: SquidData::Tri(Tri {
                mesh: None,
                data: Smooth::new(data, None),
                mesh_p1: p1,
                mesh_p2: p2,
                mesh_p3: p3,
                moving_point: None,
                translate_behavior: Default::default(),
                rotating: false,
                rotation_accumulator: Accumulator::new(),
                virtual_rotation: Rad(0.0),
                prescale_size: [p1, p2, p3],
                spread_behavior: Default::default(),
                revolve_behavior: Default::default(),
                dilate_behavior: Default::default(),
            }),
        }
    }

    // Renders squid in regular state
    pub fn render(&mut self, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
        match &mut self.data {
            SquidData::Rect(rect) => rect::render(rect, ctx, as_preview),
            SquidData::Circle(circle) => circle::render(circle, ctx, as_preview),
            SquidData::Tri(tri) => tri::render(tri, ctx, as_preview),
        }
    }

    // Render additional selection indicators and helpers for when
    // the squid is selected
    pub fn get_selection_points(&self, camera: &Camera, output: &mut Vec<glm::Vec2>) {
        match &self.data {
            SquidData::Rect(rect) => {
                let RectData { position, .. } = rect.data.get_animated();
                output.push(camera.apply(&position.reveal()));
                output.push(rect::get_rotate_handle(rect, camera));

                for corner in rect::get_relative_corners(rect) {
                    output.push(camera.apply(&(position.reveal() + corner)));
                }
            }
            SquidData::Circle(circle) => {
                let CircleData { position, .. } = circle.data.get_animated();
                output.push(camera.apply(&position.reveal()));
                output.push(circle::get_rotate_handle(circle, camera));
            }
            SquidData::Tri(tri) => {
                let TriData { position, .. } = tri.data.get_animated();

                output.push(camera.apply(&position.reveal()));
                output.push(tri::get_rotate_handle(tri, camera));

                for point in tri::get_animated_screen_points(tri, camera) {
                    output.push(point);
                }
            }
        }
    }

    // Called when squid is selected and has opportunity to capture
    // user interaction
    // Returns if and how the interaction was captured
    pub fn interact(&mut self, interaction: &Interaction, camera: &Camera, _options: &InteractionOptions) -> Capture {
        match &mut self.data {
            SquidData::Rect(rect) => rect::interact(rect, interaction, camera),
            SquidData::Circle(circle) => circle::interact(circle, interaction, camera),
            SquidData::Tri(tri) => tri::interact(tri, interaction, camera),
        }
    }

    fn translate_behavior(&mut self) -> Option<&mut TranslateBehavior> {
        match &mut self.data {
            SquidData::Rect(rect) => Some(&mut rect.translate_behavior),
            SquidData::Circle(circle) => Some(&mut circle.translate_behavior),
            SquidData::Tri(tri) => Some(&mut tri.translate_behavior),
        }
    }

    fn reposition_by(&mut self, delta: glm::Vec2) {
        if delta == glm::zero::<glm::Vec2>() {
            return;
        }

        match &mut self.data {
            SquidData::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.position = MultiLerp::Linear(new_data.position.reveal() + delta);
                rect.data.set(new_data);
            }
            SquidData::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.position = MultiLerp::Linear(new_data.position.reveal() + delta);
                circle.data.set(new_data);
            }
            SquidData::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                new_data.position = MultiLerp::Linear(new_data.position.reveal() + delta);
                tri.data.set(new_data);
            }
        }
    }

    fn rotate_behavior(&mut self) -> Option<&mut Accumulator<Rad<f32>>> {
        match &mut self.data {
            SquidData::Rect(rect) => Some(&mut rect.rotation_accumulator),
            SquidData::Circle(circle) => Some(&mut circle.rotation_accumulator),
            SquidData::Tri(tri) => Some(&mut tri.rotation_accumulator),
        }
    }

    fn rotate_by(&mut self, delta_theta: Rad<f32>) {
        match &mut self.data {
            SquidData::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.rotation += delta_theta;
                rect.data.set(new_data);
            }
            SquidData::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.virtual_rotation += delta_theta;
                circle.data.set(new_data);
            }
            SquidData::Tri(tri) => {
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
            .map(|behavior| behavior.accumulate(&mouse_delta_theta, options.rotation_snapping))
            .flatten();

        if let Some(delta_theta) = delta_theta {
            self.rotate_by(delta_theta);
        }
    }

    // Scales a squid body
    pub fn scale(&mut self, total_scale_factor: f32, _options: &InteractionOptions) {
        match &mut self.data {
            SquidData::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.size = total_scale_factor * rect.prescale_size;
                rect.data.set(new_data);
                rect.mesh = None;
            }
            SquidData::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.radius = circle.prescale_size * total_scale_factor;
                circle.data.set(new_data);
            }
            SquidData::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                new_data.p1 = MultiLerp::Linear(tri.prescale_size[0] * total_scale_factor);
                new_data.p2 = MultiLerp::Linear(tri.prescale_size[1] * total_scale_factor);
                new_data.p3 = MultiLerp::Linear(tri.prescale_size[2] * total_scale_factor);
                tri.data.set(new_data);
            }
        }
    }

    // Spreads a squid body toward/from a point
    pub fn spread(&mut self, current: &glm::Vec2, _options: &InteractionOptions) {
        match &mut self.data {
            SquidData::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.position = MultiLerp::Linear(rect.spread_behavior.express(current));
                rect.data.set(new_data);
            }
            SquidData::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.position = MultiLerp::Linear(circle.spread_behavior.express(current));
                circle.data.set(new_data);
            }
            SquidData::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                new_data.position = MultiLerp::Linear(tri.spread_behavior.express(current));
                tri.data.set(new_data);
            }
        }
    }

    // Revolves a squid body around point
    pub fn revolve(&mut self, current: &glm::Vec2, options: &InteractionOptions) {
        match &mut self.data {
            SquidData::Rect(rect) => {
                if let Some(expression) = rect.revolve_behavior.express(current, options) {
                    let mut new_data = *rect.data.get_real();
                    new_data.position = MultiLerp::Circle(expression.apply_origin_rotation_to_center(), expression.origin);
                    new_data.rotation += expression.delta_object_rotation;
                    rect.data.set(new_data);
                }
            }
            SquidData::Circle(circle) => {
                if let Some(expression) = circle.revolve_behavior.express(current, options) {
                    let mut new_data = *circle.data.get_real();
                    new_data.position = MultiLerp::Circle(expression.apply_origin_rotation_to_center(), expression.origin);
                    new_data.virtual_rotation += expression.delta_object_rotation;
                    circle.data.set(new_data);
                }
            }
            SquidData::Tri(tri) => {
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
        match &mut self.data {
            SquidData::Rect(rect) => {
                let expression = rect.dilate_behavior.express(current);
                let mut new_data = *rect.data.get_real();
                new_data.position = MultiLerp::Linear(expression.position);
                new_data.size = expression.total_scale_factor * rect.prescale_size;
                rect.data.set(new_data);
                rect.mesh = None;
            }
            SquidData::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                let expression = circle.dilate_behavior.express(current);
                new_data.position = MultiLerp::Linear(expression.position);
                new_data.radius = circle.prescale_size * expression.total_scale_factor;
                circle.data.set(new_data);
            }
            SquidData::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                let expression = tri.dilate_behavior.express(current);
                new_data.position = MultiLerp::Linear(expression.position);
                new_data.p1 = MultiLerp::Linear(expression.total_scale_factor * tri.prescale_size[0]);
                new_data.p2 = MultiLerp::Linear(expression.total_scale_factor * tri.prescale_size[1]);
                new_data.p3 = MultiLerp::Linear(expression.total_scale_factor * tri.prescale_size[2]);
                tri.data.set(new_data);
            }
        }
    }

    // Attempts to get a selection for this squid or a selection for a limb of this squid
    // under the point (x, y)
    pub fn try_select(&self, underneath: glm::Vec2, camera: &Camera, self_reference: SquidRef) -> Option<NewSelection> {
        match &self.data {
            SquidData::Rect(rect) => {
                if rect::is_point_over(rect, underneath, camera) {
                    return Some(NewSelection {
                        selection: Selection::new(self_reference, None),
                        info: NewSelectionInfo {
                            color: Some(rect.data.get_real().color.0),
                        },
                    });
                }
            }
            SquidData::Circle(circle) => {
                if circle::is_point_over(circle, underneath, camera) {
                    return Some(NewSelection {
                        selection: Selection::new(self_reference, None),
                        info: NewSelectionInfo {
                            color: Some(circle.data.get_real().color.0),
                        },
                    });
                }
            }
            SquidData::Tri(tri) => {
                if tri::is_point_over(tri, underneath, camera) {
                    return Some(NewSelection {
                        selection: Selection::new(self_reference, None),
                        info: NewSelectionInfo {
                            color: Some(tri.data.get_real().color.0),
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
        match &self.data {
            SquidData::Rect(rect) => rect::is_point_over(rect, mouse_position, camera),
            SquidData::Circle(circle) => circle::is_point_over(circle, mouse_position, camera),
            SquidData::Tri(tri) => tri::is_point_over(tri, mouse_position, camera),
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
        match &mut self.data {
            SquidData::Rect(rect) => {
                let mut new_data = *rect.data.get_real();
                new_data.color = NoLerp(color);
                rect.data.set(new_data);
            }
            SquidData::Circle(circle) => {
                let mut new_data = *circle.data.get_real();
                new_data.color = NoLerp(color);
                circle.data.set(new_data);
            }
            SquidData::Tri(tri) => {
                let mut new_data = *tri.data.get_real();
                new_data.color = NoLerp(color);
                tri.data.set(new_data);
            }
        }
    }

    // Duplicates a squid
    pub fn duplicate(&self, offset: &glm::Vec2) -> Squid {
        match &self.data {
            SquidData::Rect(rect) => {
                let mut real = *rect.data.get_real();
                real.position = MultiLerp::From(real.position.reveal() + offset);
                Squid::rect_from(real)
            }
            SquidData::Circle(circle) => {
                let mut real = *circle.data.get_real();
                real.position = MultiLerp::From(real.position.reveal() + offset);
                Squid::circle_from(real)
            }
            SquidData::Tri(tri) => {
                let mut real = *tri.data.get_real();
                real.position = MultiLerp::From(real.position.reveal() + offset);
                Squid::tri_from(real)
            }
        }
    }

    // Signals to the squid to initiate a certain user action
    pub fn initiate(&mut self, initiation: Initiation) {
        match &mut self.data {
            SquidData::Rect(rect) => match initiation {
                Initiation::Translate => {
                    rect.translate_behavior.moving = true;
                    rect.moving_corner = None;
                }
                Initiation::Rotate => (),
                Initiation::Scale => {
                    let real = rect.data.get_real();
                    rect.prescale_size = real.size;
                }
                Initiation::Spread { point, center } => {
                    rect.spread_behavior = SpreadBehavior {
                        origin: center,
                        start: rect.data.get_real().position.reveal(),
                        point,
                    };
                }
                Initiation::Dilate { point, center } => {
                    let real = rect.data.get_real();
                    rect.prescale_size = real.size;
                    rect.dilate_behavior = DilateBehavior {
                        point,
                        origin: center,
                        start: rect.data.get_real().position.reveal(),
                    };
                }
                Initiation::Revolve { point, center } => rect.revolve_behavior.set(&center, &rect.data.get_real().position.reveal(), &point),
            },
            SquidData::Circle(circle) => match initiation {
                Initiation::Translate => circle.translate_behavior.moving = true,
                Initiation::Rotate => (),
                Initiation::Scale => circle.prescale_size = circle.data.get_real().radius,
                Initiation::Spread { point, center } => {
                    circle.spread_behavior = SpreadBehavior {
                        point,
                        origin: center,
                        start: circle.data.get_real().position.reveal(),
                    };
                }
                Initiation::Revolve { point, center } => circle.revolve_behavior.set(&center, &circle.data.get_real().position.reveal(), &point),
                Initiation::Dilate { point, center } => {
                    circle.prescale_size = circle.data.get_real().radius;
                    circle.dilate_behavior = DilateBehavior {
                        point,
                        origin: center,
                        start: circle.data.get_real().position.reveal(),
                    };
                }
            },
            SquidData::Tri(tri) => match initiation {
                Initiation::Translate => {
                    tri.translate_behavior.moving = true;
                    tri.moving_point = None;
                }
                Initiation::Rotate => (),
                Initiation::Scale => {
                    let real = tri.data.get_real();
                    tri.prescale_size = [real.p1.reveal(), real.p2.reveal(), real.p3.reveal()];
                }
                Initiation::Spread { point, center } => {
                    tri.spread_behavior = SpreadBehavior {
                        point,
                        origin: center,
                        start: tri.data.get_real().position.reveal(),
                    };
                }
                Initiation::Revolve { point, center } => tri.revolve_behavior.set(&center, &tri.data.get_real().position.reveal(), &point),
                Initiation::Dilate { point, center } => {
                    let real = tri.data.get_real();
                    tri.prescale_size = [real.p1.reveal(), real.p2.reveal(), real.p3.reveal()];
                    tri.dilate_behavior = DilateBehavior {
                        point,
                        origin: center,
                        start: tri.data.get_real().position.reveal(),
                    };
                }
            },
        }
    }

    // Gets center of a squid
    pub fn get_center(&self) -> glm::Vec2 {
        match &self.data {
            SquidData::Rect(rect) => rect.data.get_animated().position.reveal(),
            SquidData::Circle(circle) => circle.data.get_animated().position.reveal(),
            SquidData::Tri(tri) => tri.data.get_animated().position.reveal(),
        }
    }

    // Opaque name getter/setter
    pub fn get_name(&self) -> &str {
        self.name.as_deref().unwrap_or_else(|| match self.data {
            SquidData::Rect(_) => "Unnamed Rect",
            SquidData::Circle(_) => "Unnamed Circle",
            SquidData::Tri(_) => "Unnamed Tri",
        })
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    // Returns the world positions of all "opaque" handles (aka handles that will take priority over new selections)
    pub fn get_opaque_handles(&self) -> Vec<glm::Vec2> {
        match &self.data {
            SquidData::Rect(rect) => {
                let mut handles = rect::get_relative_corners(rect);
                handles.push(rect::get_rotate_handle(rect, &IDENTITY_CAMERA));
                handles
            }
            SquidData::Circle(circle) => {
                vec![circle::get_rotate_handle(circle, &IDENTITY_CAMERA)]
            }
            SquidData::Tri(tri) => {
                let data = tri.data.get_animated();
                let position = data.position.reveal();
                vec![
                    data.p1.reveal() + position,
                    data.p2.reveal() + position,
                    data.p3.reveal() + position,
                    tri::get_rotate_handle(tri, &IDENTITY_CAMERA),
                ]
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
    let delete = ContextMenuOption::new("Delete".to_string(), "X".to_string(), ContextAction::DeleteSelected);
    let duplicate = ContextMenuOption::new("Duplicate".to_string(), "Shift+D".to_string(), ContextAction::DuplicateSelected);
    let grab = ContextMenuOption::new("Grab".to_string(), "G".to_string(), ContextAction::GrabSelected);
    let rotate = ContextMenuOption::new("Rotate".to_string(), "R".to_string(), ContextAction::RotateSelected);
    let scale = ContextMenuOption::new("Scale".to_string(), "S".to_string(), ContextAction::ScaleSelected);
    let collectively = ContextMenuOption::new("Collectively".to_string(), "C".to_string(), ContextAction::Collectively);
    ContextMenu::new(underneath, vec![delete, duplicate, grab, rotate, scale, collectively], color_scheme.dark_ribbon)
}

pub struct PreviewParams {
    pub position: glm::Vec2,
    pub radius: f32,
}
