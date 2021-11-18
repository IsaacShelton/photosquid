use super::{Initiation, Squid, SquidRef};
use crate::{
    accumulator::Accumulator,
    algorithm,
    capture::Capture,
    color::Color,
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    interaction::Interaction,
    interaction_options::InteractionOptions,
    math_helpers::DivOrZero,
    matrix_helpers::reach_inside_mat4,
    mesh::MeshXyz,
    render_ctx::RenderCtx,
    selection::{NewSelection, NewSelectionInfo, Selection},
    smooth::{Lerpable, MultiLerp, NoLerp, Smooth},
    squid::{
        self,
        behavior::{RevolveBehavior, SpreadBehavior, TranslateBehavior},
        PreviewParams,
    },
};
use angular_units::{self, Angle, Rad};
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;
use std::time::{Duration, Instant};

pub struct Rect {
    name: Option<String>,
    data: Smooth<RectData>,
    created: Instant,
    mesh: Option<MeshXyz>,

    // --------- Tweaking parameters ---------

    // Move point
    moving_corner: Option<CornerKind>,

    // Translate
    translate_behavior: TranslateBehavior,

    // Rotate
    rotating: bool,
    rotation_accumulator: Accumulator<Rad<f32>>,

    // Scale
    prescale_size: glm::Vec2,

    // Spread
    spread_behavior: SpreadBehavior,

    // Revolve
    revolve_behavior: RevolveBehavior,
}

#[derive(Copy, Clone)]
pub struct RectData {
    position: MultiLerp<glm::Vec2>,
    w: f32,
    h: f32,
    color: NoLerp<Color>,
    rotation: Rad<f32>,
}

impl Lerpable for RectData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            position: self.position.lerp(&other.position, scalar),
            w: interpolation::Lerp::lerp(&self.w, &other.w, scalar),
            h: interpolation::Lerp::lerp(&self.h, &other.h, scalar),
            rotation: angular_units::Interpolate::interpolate(&self.rotation, &other.rotation, *scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
        }
    }
}

#[derive(Copy, Clone)]
enum CornerKind {
    ZeroZero = 3,
    ZeroY = 1,
    XZero = 2,
    XY = 0,
}

#[derive(Copy, Clone, PartialEq)]
enum CornerDependence {
    Neither,
    Both,
    X,
    Y,
}

fn get_corner_dependence(moving: CornerKind, dependent: CornerKind) -> CornerDependence {
    use CornerDependence::*;

    //          dependent x,y  0,y  x,0   0,0
    // moving
    // x,y                xy   y    x     n/a
    // 0,y                y    xy   n/a   x
    // x,0                x    n/a  xy    y
    // 0,0                n/a  x    y     xy
    let table = [[Both, Y, X, Neither], [Y, Both, Neither, X], [X, Neither, Both, Y], [Neither, X, Y, Both]];
    table[moving as usize][dependent as usize]
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32, rotation: Rad<f32>, color: Color) -> Self {
        let data = RectData {
            position: MultiLerp::From(glm::vec2(x, y)),
            w,
            h,
            rotation,
            color: NoLerp(color),
        };

        Self::from_data(data)
    }

    pub fn from_data(data: RectData) -> Self {
        Self {
            name: None,
            data: Smooth::new(data, Duration::from_millis(500)),
            created: Instant::now(),
            mesh: None,
            moving_corner: None,
            translate_behavior: Default::default(),
            rotating: false,
            rotation_accumulator: Accumulator::new(),
            prescale_size: glm::vec2(data.w, data.h),
            spread_behavior: Default::default(),
            revolve_behavior: Default::default(),
        }
    }

    fn get_corner_kind(corner_index: usize) -> CornerKind {
        use CornerKind::*;
        [XY, ZeroY, XZero, ZeroZero][corner_index]
    }

    fn get_relative_corners(&self) -> Vec<glm::Vec2> {
        // Use non-animated version always for now?
        // It seems to look better this way
        let RectData { w, h, .. } = self.data.get_real();
        let RectData { rotation, .. } = self.data.get_animated();

        [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0f32)]
            .iter()
            .map(|&p| glm::vec2(p.0 * w / 2.0, p.1 * h / 2.0))
            .map(|p| glm::rotate_vec2(&p, -1.0 * rotation.scalar()))
            .collect()
    }

    fn get_world_corners(&self) -> Vec<glm::Vec2> {
        let RectData { position, .. } = self.data.get_animated();

        self.get_relative_corners()
            .iter()
            .map(|p| glm::vec2(p.x + position.reveal().x, p.y + position.reveal().y))
            .collect()
    }

    fn get_screen_corners(&self, camera: &glm::Vec2) -> Vec<glm::Vec2> {
        let RectData { position, .. } = self.data.get_animated();

        self.get_relative_corners()
            .iter()
            .map(|p| glm::vec2(p.x + position.reveal().x + camera.x, p.y + position.reveal().y + camera.y))
            .collect()
    }

    fn reposition_corner(&mut self, target_screen_position: &glm::Vec2, camera: &glm::Vec2) {
        use CornerDependence::*;
        use CornerKind::*;

        let real = self.data.get_real();
        let moving_corner = self.moving_corner.unwrap();
        let relative_corners = self.get_relative_corners();
        let target_position = *target_screen_position - camera - glm::vec2(real.position.reveal().x, real.position.reveal().y);

        // Rotate corners back into axis-aligned space
        let rotation = real.rotation.scalar();
        let axis_aligned_relative_corners: Vec<glm::Vec2> = relative_corners.iter().map(|p| glm::rotate_vec2(p, rotation)).collect();
        let axis_aligned_target_position = glm::rotate_vec2(&target_position, rotation);

        let mut result: Vec<glm::Vec2> = vec![];
        for (i, c) in axis_aligned_relative_corners.iter().enumerate() {
            let other_corner = Self::get_corner_kind(i);
            let dependence = get_corner_dependence(moving_corner, other_corner);

            result.push(glm::vec2(
                if dependence == Both || dependence == X {
                    axis_aligned_target_position.x
                } else {
                    c.x
                },
                if dependence == Both || dependence == Y {
                    axis_aligned_target_position.y
                } else {
                    c.y
                },
            ));
        }

        let new_w = result[XZero as usize].x - result[ZeroZero as usize].x;
        let new_h = result[ZeroY as usize].y - result[ZeroZero as usize].y;
        let delta_w = new_w - real.w;
        let delta_h = new_h - real.h;

        let mut new_data = *real;
        new_data.w += delta_w;
        new_data.h += delta_h;
        self.data.set(new_data);
    }

    fn get_rotate_handle_location(&self, camera: &glm::Vec2) -> glm::Vec2 {
        let RectData { position, w, rotation, .. } = self.data.get_animated();

        glm::vec2(
            position.reveal().x + camera.x + rotation.cos() * (w * 0.5 + 24.0 * w.signum()),
            position.reveal().y + camera.y - rotation.sin() * (w * 0.5 + 24.0 * w.signum()),
        )
    }
}

impl Squid for Rect {
    fn render(&mut self, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
        let RectData {
            position,
            w,
            h,
            rotation,
            color,
        } = self.data.get_animated();

        if self.mesh.is_none() {
            self.mesh = Some(MeshXyz::new_shape_square(ctx.display));
        }

        let mut transformation = if let Some(preview) = &as_preview {
            glm::translation(&glm::vec2_to_vec3(&preview.position))
        } else {
            glm::translation(&glm::vec3(position.reveal().x, position.reveal().y, 0.0))
        };

        transformation = glm::rotate(&transformation, rotation.scalar(), &glm::vec3(0.0, 0.0, -1.0));
        transformation = glm::scale(&transformation, &glm::vec3(w, h, 0.0));

        if let Some(preview) = &as_preview {
            let max_size = w.max(h);
            let factor = 1.0.div_or_zero(max_size);
            transformation = glm::scale(&transformation, &glm::vec3(factor * preview.size, factor * preview.size, 0.0));
        }

        let view = if as_preview.is_some() {
            reach_inside_mat4(&glm::identity::<f32, 4>())
        } else {
            reach_inside_mat4(ctx.view)
        };

        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: view,
            projection: reach_inside_mat4(ctx.projection),
            color: Into::<[f32; 4]>::into(color.0)
        };

        let mesh = self.mesh.as_ref().unwrap();
        ctx.draw(&mesh.vertex_buffer, &mesh.indices, ctx.color_shader, &uniforms, &Default::default())
            .unwrap();
    }

    fn render_selected_indication(&self, ctx: &mut RenderCtx) {
        let camera = ctx.camera;
        let RectData { position, .. } = self.data.get_animated();
        let x = position.reveal().x;
        let y = position.reveal().y;

        ctx.ring_mesh.render(
            ctx,
            x + camera.x,
            y + camera.y,
            squid::HANDLE_RADIUS,
            squid::HANDLE_RADIUS,
            &ctx.color_scheme.foreground,
        );

        let rotate_handle = self.get_rotate_handle_location(camera);

        ctx.ring_mesh.render(
            ctx,
            rotate_handle.x,
            rotate_handle.y,
            squid::HANDLE_RADIUS,
            squid::HANDLE_RADIUS,
            &ctx.color_scheme.foreground,
        );

        for corner in &self.get_relative_corners() {
            ctx.ring_mesh.render(
                ctx,
                x + camera.x + corner.x,
                y + camera.y + corner.y,
                squid::HANDLE_RADIUS,
                squid::HANDLE_RADIUS,
                &ctx.color_scheme.foreground,
            );
        }
    }

    fn interact(&mut self, interaction: &Interaction, camera: &glm::Vec2, _options: &InteractionOptions) -> Capture {
        match interaction {
            Interaction::PreClick => {
                self.translate_behavior.moving = false;
                self.rotating = false;
                self.moving_corner = None;
            }
            Interaction::Click {
                button: MouseButton::Left,
                position,
            } => {
                for (i, corner) in self.get_screen_corners(camera).iter().enumerate() {
                    if glm::distance(position, corner) <= squid::HANDLE_RADIUS * 2.0 {
                        self.moving_corner = Some(Self::get_corner_kind(i));
                        return Capture::AllowDrag;
                    }
                }

                let rotate_handle_location = self.get_rotate_handle_location(camera);
                if glm::distance(position, &rotate_handle_location) <= squid::HANDLE_RADIUS * 2.0 {
                    self.rotating = true;
                    return Capture::AllowDrag;
                }

                if self.is_point_over(position, camera) {
                    self.translate_behavior.moving = true;
                    return Capture::AllowDrag;
                }
            }
            Interaction::Drag {
                delta,
                current: mouse_position,
                ..
            } => {
                if self.moving_corner.is_some() {
                    self.reposition_corner(mouse_position, camera);
                } else if self.rotating {
                    return Capture::RotateSelectedSquids {
                        delta_theta: squid::behavior::get_delta_rotation(
                            &self.data.get_real().position.reveal(),
                            self.data.get_real().rotation,
                            mouse_position,
                            &self.rotation_accumulator,
                            camera,
                        ),
                    };
                } else if self.translate_behavior.moving {
                    return Capture::MoveSelectedSquids { delta: *delta };
                }
            }
            Interaction::MouseRelease { button: MouseButton::Left, .. } => {
                self.rotating = false;
                self.moving_corner = None;
                self.translate_behavior.accumulator.clear();
                self.rotation_accumulator.clear();
            }
            _ => (),
        }

        Capture::Miss
    }

    fn translate(&mut self, raw_delta: &glm::Vec2, options: &InteractionOptions) {
        let delta = self.translate_behavior.express(raw_delta, options);

        if delta != glm::zero::<glm::Vec2>() {
            let mut new_data = *self.data.get_real();
            new_data.position = MultiLerp::Linear(new_data.position.reveal() + delta);
            self.data.set(new_data);
        }
    }

    fn rotate(&mut self, raw_delta_theta: Rad<f32>, options: &InteractionOptions) {
        if let Some(delta_theta) = self.rotation_accumulator.accumulate(&raw_delta_theta, options.rotation_snapping) {
            let mut new_data = *self.data.get_real();
            new_data.rotation += delta_theta;
            self.data.set(new_data);
        }
    }

    fn scale(&mut self, total_scale_factor: f32, _options: &InteractionOptions) {
        let mut new_data = *self.data.get_real();
        new_data.w = total_scale_factor * self.prescale_size.x;
        new_data.h = total_scale_factor * self.prescale_size.y;
        self.data.set(new_data);
    }

    fn spread(&mut self, point: &glm::Vec2, _options: &InteractionOptions) {
        let new_position = self.spread_behavior.express(point);

        let mut new_data = *self.data.get_real();
        new_data.position = MultiLerp::Linear(new_position);
        self.data.set(new_data);
    }

    fn revolve(&mut self, current: &glm::Vec2, options: &InteractionOptions) {
        if let Some(expression) = self.revolve_behavior.express(current, options) {
            let mut new_data = *self.data.get_real();

            let new_center = expression.apply_origin_rotation_to_center();
            new_data.position = MultiLerp::Circle(new_center, expression.origin);
            new_data.rotation += expression.delta_object_rotation;
            self.data.set(new_data);
        }
    }

    fn is_point_over(&self, underneath: &glm::Vec2, camera: &glm::Vec2) -> bool {
        let real = self.data.get_real();
        let corners: Vec<glm::Vec2> = self
            .get_relative_corners()
            .iter()
            .map(|&p| glm::vec2(real.position.reveal().x + camera.x + p.x, real.position.reveal().y + camera.y + p.y))
            .collect();

        assert_eq!(corners.len(), 4);
        algorithm::is_point_inside_rectangle(corners[0], corners[1], corners[2], corners[3], *underneath)
    }

    fn try_select(&self, underneath: &glm::Vec2, camera: &glm::Vec2, self_reference: SquidRef) -> Option<NewSelection> {
        if self.is_point_over(underneath, camera) {
            Some(NewSelection {
                selection: Selection::new(self_reference, None),
                info: NewSelectionInfo {
                    color: Some(self.data.get_real().color.0),
                },
            })
        } else {
            None
        }
    }

    fn select(&mut self) {
        self.translate_behavior.moving = true;
    }

    fn try_context_menu(&self, underneath: &glm::Vec2, camera: &glm::Vec2, _self_reference: SquidRef, color_scheme: &ColorScheme) -> Option<ContextMenu> {
        if self.is_point_over(underneath, camera) {
            Some(squid::common_context_menu(underneath, color_scheme))
        } else {
            None
        }
    }

    fn set_color(&mut self, color: Color) {
        let mut new_data = *self.data.get_real();
        new_data.color = NoLerp(color);
        self.data.set(new_data);
    }

    fn duplicate(&self, offset: &glm::Vec2) -> Box<dyn Squid> {
        let mut real = *self.data.get_real();
        real.position = MultiLerp::From(real.position.reveal() + offset);
        Box::new(Self::from_data(real))
    }

    fn get_creation_time(&self) -> Instant {
        self.created
    }

    fn initiate(&mut self, initiation: Initiation) {
        match initiation {
            Initiation::Translate => {
                self.translate_behavior.moving = true;
                self.moving_corner = None;
            }
            Initiation::Rotate => (),
            Initiation::Scale => {
                let real = self.data.get_real();
                self.prescale_size = glm::vec2(real.w, real.h);
            }
            Initiation::Spread { point, center } => {
                self.spread_behavior = SpreadBehavior {
                    point,
                    origin: center,
                    start: self.get_center(),
                };
            }
            Initiation::Revolve { point, center } => self.revolve_behavior.set(&center, &self.get_center(), &point),
        }
    }

    fn get_center(&self) -> glm::Vec2 {
        let RectData { position, .. } = self.data.get_animated();
        position.reveal()
    }

    fn get_name(&self) -> &str {
        self.name.as_deref().unwrap_or("Unnamed Rect")
    }

    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    fn get_opaque_handles(&self) -> Vec<glm::Vec2> {
        let mut all_handles = self.get_world_corners();
        all_handles.push(self.get_rotate_handle_location(&glm::zero()));
        all_handles
    }
}
