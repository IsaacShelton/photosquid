use super::{Initiation, Squid, SquidRef};
use crate::{
    accumulator::Accumulator,
    algorithm,
    camera::Camera,
    capture::Capture,
    color::Color,
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    interaction::{ClickInteraction, DragInteraction, Interaction, MouseReleaseInteraction},
    interaction_options::InteractionOptions,
    math_helpers::DivOrZero,
    matrix_helpers::reach_inside_mat4,
    mesh::MeshXyz,
    render_ctx::RenderCtx,
    selection::{NewSelection, NewSelectionInfo, Selection},
    smooth::{Lerpable, MultiLerp, NoLerp, Smooth},
    squid::{
        self,
        behavior::{DilateBehavior, RevolveBehavior, SpreadBehavior, TranslateBehavior},
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
    moving_corner: Option<Corner>,

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

    // Dilate
    dilate_behavior: DilateBehavior,
}

#[derive(Copy, Clone)]
pub struct RectData {
    position: MultiLerp<glm::Vec2>,
    w: f32,
    h: f32,
    color: NoLerp<Color>,
    rotation: Rad<f32>,
    radii: f32,
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
            radii: interpolation::Lerp::lerp(&self.radii, &other.radii, scalar),
        }
    }
}

#[derive(Copy, Clone)]
enum Corner {
    ZeroZero = 3,
    ZeroY = 1,
    XZero = 2,
    XY = 0,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32, rotation: Rad<f32>, color: Color, radii: f32) -> Self {
        let data = RectData {
            position: MultiLerp::From(glm::vec2(x, y)),
            w,
            h,
            rotation,
            color: NoLerp(color),
            radii,
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
            dilate_behavior: Default::default(),
        }
    }

    fn get_corner_kind(corner_index: usize) -> Corner {
        use Corner::*;
        [XY, ZeroY, XZero, ZeroZero][corner_index]
    }

    fn get_relative_corners(&self) -> Vec<glm::Vec2> {
        // Use non-animated version always for now?
        // It seems to look better this way
        let RectData { w, h, .. } = self.data.get_animated();
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

    fn get_screen_corners(&self, camera: &Camera) -> Vec<glm::Vec2> {
        let RectData { position, .. } = self.data.get_animated();

        self.get_relative_corners().iter().map(|p| camera.apply(&(p + position.reveal()))).collect()
    }

    fn reposition_corner(&mut self, mouse: &glm::Vec2, camera: &Camera) {
        let real = self.data.get_real();
        let rotation = real.rotation.scalar();
        let mouse_in_world = camera.apply_reverse(mouse);
        let abs_size = 2.0 * glm::rotate_vec2(&(real.position.reveal() - mouse_in_world), rotation);

        let new_size = abs_size.component_mul(&match self.moving_corner.unwrap() {
            Corner::ZeroZero => glm::vec2(1.0, 1.0),
            Corner::XZero => glm::vec2(-1.0, 1.0),
            Corner::ZeroY => glm::vec2(1.0, -1.0),
            Corner::XY => glm::vec2(-1.0, -1.0),
        });

        let mut new_data = *real;
        new_data.w = new_size.x;
        new_data.h = new_size.y;
        self.data.set(new_data);

        self.mesh = None;
    }

    fn get_rotate_handle_location(&self, camera: &Camera) -> glm::Vec2 {
        let RectData { position, w, rotation, .. } = self.data.get_animated();

        let world_position = glm::vec2(
            position.reveal().x + rotation.cos() * (w * 0.5 + 24.0 * w.signum()),
            position.reveal().y - rotation.sin() * (w * 0.5 + 24.0 * w.signum()),
        );

        camera.apply(&world_position)
    }

    fn refresh_mesh(&mut self, ctx: &mut RenderCtx) {
        let real_data = self.data.get_real();
        let animated_data = self.data.get_animated();

        // Don't use margin of error
        let has_radii_animation = real_data.radii != animated_data.radii;
        let has_size_animation = real_data.w != animated_data.w || real_data.h != animated_data.h;

        if self.mesh.is_none() || has_radii_animation || has_size_animation {
            self.mesh = Some(MeshXyz::new_rect(ctx.display, animated_data.w, animated_data.h, animated_data.radii));
        }
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
            ..
        } = self.data.get_animated();

        self.refresh_mesh(ctx);

        let mut transformation = if let Some(preview) = &as_preview {
            glm::translation(&glm::vec2_to_vec3(&preview.position))
        } else {
            glm::translation(&glm::vec3(position.reveal().x, position.reveal().y, 0.0))
        };

        transformation = glm::rotate(&transformation, rotation.scalar(), &glm::vec3(0.0, 0.0, -1.0));

        if let Some(preview) = &as_preview {
            let max_size = w.abs().max(h.abs());
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
        let ring_position = camera.apply(&position.reveal());

        ctx.ring_mesh.render(
            ctx,
            ring_position.x,
            ring_position.y,
            squid::HANDLE_RADIUS,
            squid::HANDLE_RADIUS,
            &ctx.color_scheme.foreground,
        );

        let rotate_handle = self.get_rotate_handle_location(&camera);

        ctx.ring_mesh.render(
            ctx,
            rotate_handle.x,
            rotate_handle.y,
            squid::HANDLE_RADIUS,
            squid::HANDLE_RADIUS,
            &ctx.color_scheme.foreground,
        );

        for corner in &self.get_relative_corners() {
            let world_corner_position = position.reveal() + corner;
            let view_corner_position = camera.apply(&world_corner_position);
            ctx.ring_mesh.render(
                ctx,
                view_corner_position.x,
                view_corner_position.y,
                squid::HANDLE_RADIUS,
                squid::HANDLE_RADIUS,
                &ctx.color_scheme.foreground,
            );
        }
    }

    fn interact(&mut self, interaction: &Interaction, camera: &Camera, _options: &InteractionOptions) -> Capture {
        match interaction {
            Interaction::PreClick => {
                self.translate_behavior.moving = false;
                self.rotating = false;
                self.moving_corner = None;
            }
            Interaction::Click(ClickInteraction {
                button: MouseButton::Left,
                position,
            }) => {
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
            Interaction::Drag(DragInteraction {
                delta,
                current: mouse_position,
                ..
            }) => {
                if self.moving_corner.is_some() {
                    self.reposition_corner(mouse_position, camera);
                } else if self.rotating {
                    // When the rectangle's width is negative, the rotation handle is PI radians ahead of it's angle
                    // compared to the actual rotation of the shape,
                    // So we have to compensate for the difference when that's the case.
                    // It might be better to integrate this difference into the rotation and not allow negative
                    // widths/heights for rectangles, but this is an easier way to do it
                    let compensation = Rad(if self.data.get_animated().w < 0.0 { std::f32::consts::PI } else { 0.0 });

                    return Capture::RotateSelectedSquids {
                        delta_theta: squid::behavior::get_delta_rotation(
                            &self.data.get_real().position.reveal(),
                            self.data.get_real().rotation,
                            mouse_position,
                            &self.rotation_accumulator,
                            camera,
                        ) + compensation,
                    };
                } else if self.translate_behavior.moving {
                    return Capture::MoveSelectedSquids {
                        delta_in_world: camera.apply_reverse_to_vector(delta),
                    };
                }
            }
            Interaction::MouseRelease(MouseReleaseInteraction { button: MouseButton::Left, .. }) => {
                self.rotating = false;
                self.moving_corner = None;
                self.translate_behavior.accumulator.clear();
                self.rotation_accumulator.clear();
            }
            _ => (),
        }

        Capture::Miss
    }

    fn translate(&mut self, raw_delta_in_world: &glm::Vec2, options: &InteractionOptions) {
        let delta = self.translate_behavior.express(raw_delta_in_world, options);

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

        self.mesh = None;
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

    fn dilate(&mut self, point: &glm::Vec2, _options: &InteractionOptions) {
        let expression = self.dilate_behavior.express(point);

        let mut new_data = *self.data.get_real();
        new_data.position = MultiLerp::Linear(expression.position);
        new_data.w = expression.total_scale_factor * self.prescale_size.x;
        new_data.h = expression.total_scale_factor * self.prescale_size.y;
        self.data.set(new_data);

        self.mesh = None;
    }

    fn is_point_over(&self, underneath: &glm::Vec2, camera: &Camera) -> bool {
        let real = self.data.get_real();

        let corners: Vec<glm::Vec2> = self
            .get_relative_corners()
            .iter()
            .map(|&p| camera.apply(&(p + real.position.reveal())))
            .collect();

        assert_eq!(corners.len(), 4);
        algorithm::is_point_inside_rectangle(corners[0], corners[1], corners[2], corners[3], *underneath)
    }

    fn try_select(&self, underneath: &glm::Vec2, camera: &Camera, self_reference: SquidRef) -> Option<NewSelection> {
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

    fn try_context_menu(&self, underneath: &glm::Vec2, camera: &Camera, _self_reference: SquidRef, color_scheme: &ColorScheme) -> Option<ContextMenu> {
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
            Initiation::Dilate { point, center } => {
                let real = self.data.get_real();
                self.prescale_size = glm::vec2(real.w, real.h);
                self.dilate_behavior = DilateBehavior {
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
        all_handles.push(self.get_rotate_handle_location(&Camera::identity(glm::zero())));
        all_handles
    }
}
