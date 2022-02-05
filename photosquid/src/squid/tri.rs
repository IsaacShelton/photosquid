use super::{Initiation, Squid, SquidRef};
use crate::{
    accumulator::Accumulator,
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
use angular_units::{Angle, Rad};
use glium::{glutin::event::MouseButton, Display};
use nalgebra_glm as glm;
use std::time::{Duration, Instant};

pub struct Tri {
    name: Option<String>,
    data: Smooth<TriData>,
    created: Instant,
    mesh: Option<MeshXyz>,

    // Keep track of which points the mesh is made of,
    // so that we know when we have to re-create it
    mesh_p1: glm::Vec2,
    mesh_p2: glm::Vec2,
    mesh_p3: glm::Vec2,

    // --------- Tweaking parameters ---------

    // Move point
    moving_point: Option<usize>, // (zero indexed)

    // Translate
    translate_behavior: TranslateBehavior,

    // Rotate
    rotating: bool,
    rotation_accumulator: Accumulator<Rad<f32>>,
    virtual_rotation: Rad<f32>, // Rotation that only applies to the handle

    // Scale
    prescale_size: [glm::Vec2; 3],

    // Spread
    spread_behavior: SpreadBehavior,

    // Revolve
    revolve_behavior: RevolveBehavior,

    // Dilate
    dilate_behavior: DilateBehavior,
}

#[derive(Copy, Clone)]
pub struct TriData {
    p1: MultiLerp<glm::Vec2>,
    p2: MultiLerp<glm::Vec2>,
    p3: MultiLerp<glm::Vec2>,
    center: MultiLerp<glm::Vec2>,
    color: NoLerp<Color>,
    rotation: Rad<f32>,
}

impl Lerpable for TriData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            p1: Lerpable::lerp(&self.p1, &other.p1, scalar),
            p2: Lerpable::lerp(&self.p2, &other.p2, scalar),
            p3: Lerpable::lerp(&self.p3, &other.p3, scalar),
            center: self.center.lerp(&other.center, scalar),
            rotation: angular_units::Interpolate::interpolate(&self.rotation, &other.rotation, *scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
        }
    }
}

impl Tri {
    pub fn new(p1: glm::Vec2, p2: glm::Vec2, p3: glm::Vec2, rotation: Rad<f32>, color: Color) -> Self {
        let center = Tri::get_center(&p1, &p2, &p3);

        let data = TriData {
            p1: MultiLerp::From(p1 - center),
            p2: MultiLerp::From(p2 - center),
            p3: MultiLerp::From(p3 - center),
            center: MultiLerp::From(center),
            rotation,
            color: NoLerp(color),
        };
        Self::from_data(data)
    }

    pub fn from_data(data: TriData) -> Self {
        let p1 = &data.p1.reveal();
        let p2 = &data.p2.reveal();
        let p3 = &data.p3.reveal();
        let center = &data.center.reveal();

        Self {
            name: None,
            data: Smooth::new(data, Duration::from_millis(500)),
            created: Instant::now(),
            mesh: None,
            mesh_p1: *p1,
            mesh_p2: *p2,
            mesh_p3: *p3,
            moving_point: None,
            translate_behavior: Default::default(),
            rotating: false,
            rotation_accumulator: Accumulator::new(),
            virtual_rotation: Rad(0.0),
            prescale_size: [p1 - center, p2 - center, p3 - center],
            spread_behavior: Default::default(),
            revolve_behavior: Default::default(),
            dilate_behavior: Default::default(),
        }
    }

    fn refresh_mesh(&mut self, display: &Display) {
        let TriData { p1, p2, p3, .. } = self.data.get_animated();

        let p1 = p1.reveal();
        let p2 = p2.reveal();
        let p3 = p3.reveal();

        if self.mesh.is_none()
            || glm::distance2(&p1, &self.mesh_p1) > 1.0
            || glm::distance2(&p2, &self.mesh_p2) > 1.0
            || glm::distance2(&p3, &self.mesh_p3) > 1.0
        {
            // Data points are far enough from existing mesh that we will need
            // to re-create it
            self.mesh = Some(MeshXyz::new_shape_triangle(display, p1, p2, p3));
        }
    }

    fn is_point_inside_triangle(p: &glm::Vec2, p1: &glm::Vec2, p2: &glm::Vec2, p3: &glm::Vec2) -> bool {
        fn sign(p1: &glm::Vec2, p2: &glm::Vec2, p3: &glm::Vec2) -> f32 {
            (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
        }

        let d1 = sign(p, p1, p2);
        let d2 = sign(p, p2, p3);
        let d3 = sign(p, p3, p1);

        let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;

        !(has_neg && has_pos)
    }

    fn get_real_center(&self) -> glm::Vec2 {
        let TriData { center, .. } = self.data.get_real();
        center.reveal()
    }

    fn get_animated_center(&self) -> glm::Vec2 {
        let TriData { center, .. } = self.data.get_animated();
        center.reveal()
    }

    fn get_center(p1: &glm::Vec2, p2: &glm::Vec2, p3: &glm::Vec2) -> glm::Vec2 {
        (p1 + p2 + p3) / 3.0
    }

    fn get_animated_screen_points(&self, camera: &Camera) -> Vec<glm::Vec2> {
        let TriData { p1, p2, p3, rotation, .. } = self.data.get_animated();
        let center = self.get_animated_center();

        [p1.reveal(), p2.reveal(), p3.reveal()]
            .iter()
            .map(|p| camera.apply(&(glm::rotate_vec2(p, -rotation.scalar()) + center)))
            .collect()
    }

    fn get_rotate_handle_location(&self, camera: &Camera) -> glm::Vec2 {
        let TriData {
            center, p1, p2, p3, rotation, ..
        } = self.data.get_animated();

        let rotation = rotation + self.virtual_rotation;
        let p1 = p1.reveal();
        let p2 = p2.reveal();
        let p3 = p3.reveal();
        let center = center.reveal();

        let max_distance = glm::magnitude(&p1).max(glm::magnitude(&p2)).max(glm::magnitude(&p3));
        let first_try = center + (max_distance + 24.0) * glm::vec2(rotation.cos(), -rotation.sin());

        let screen_points = self.get_animated_screen_points(&Camera::identity(camera.window));
        assert_eq!(screen_points.len(), 3);

        let r_p1 = &screen_points[0];
        let r_p2 = &screen_points[1];
        let r_p3 = &screen_points[2];
        let true_distance = Self::get_distance_between_point_and_triangle(&first_try, r_p1, r_p2, r_p3);
        let final_distance = (max_distance + 24.0 - true_distance) + 24.0;

        let rotate_handle_world_position = center + final_distance * glm::vec2(rotation.cos(), -rotation.sin());
        camera.apply(&rotate_handle_world_position)
    }

    fn reposition_point(&mut self, mouse_position: &glm::Vec2, camera: &Camera) {
        let real = self.data.get_real();
        let rotation = self.data.get_real().rotation;
        let center = self.get_real_center();

        let mut p1 = glm::rotate_vec2(&real.p1.reveal(), -rotation.scalar());
        let mut p2 = glm::rotate_vec2(&real.p2.reveal(), -rotation.scalar());
        let mut p3 = glm::rotate_vec2(&real.p3.reveal(), -rotation.scalar());

        let mouse_world_position = camera.apply_reverse(&mouse_position);
        let new_p = mouse_world_position - center;

        match self.moving_point {
            Some(0) => p1 = new_p,
            Some(1) => p2 = new_p,
            Some(2) => p3 = new_p,
            _ => (),
        }

        let new_center = center + Self::get_center(&p1, &p2, &p3);
        let delta_center = new_center - center;
        p1 -= delta_center;
        p2 -= delta_center;
        p3 -= delta_center;

        // Set new data as the new target points, with zero rotation applied

        // HACK: Instantly snap rotation back to 0.0
        if real.rotation.scalar() != 0.0 {
            self.virtual_rotation += real.rotation;

            {
                let mut_real = self.data.manual_get_real();
                mut_real.p1 = MultiLerp::Linear(p1);
                mut_real.p2 = MultiLerp::Linear(p2);
                mut_real.p3 = MultiLerp::Linear(p3);
                mut_real.center = MultiLerp::Linear(new_center);
                mut_real.rotation = Rad(0.0);
            }

            {
                let mut_previous = self.data.manual_get_previous();
                mut_previous.p1 = MultiLerp::Linear(p1);
                mut_previous.p2 = MultiLerp::Linear(p2);
                mut_previous.p3 = MultiLerp::Linear(p3);
                mut_previous.center = MultiLerp::Linear(new_center);
                mut_previous.rotation = Rad(0.0);
            }
        } else {
            let mut new_real = *self.data.get_real();
            new_real.p1 = MultiLerp::Linear(p1);
            new_real.p2 = MultiLerp::Linear(p2);
            new_real.p3 = MultiLerp::Linear(p3);
            new_real.center = MultiLerp::Linear(new_center);
            self.data.set(new_real);
        }
    }

    fn ensure_counter_clockwise<'a>(a: &mut &'a glm::Vec2, b: &mut &'a glm::Vec2, c: &mut &'a glm::Vec2) {
        use std::cmp::Ordering;
        let mut array: [&glm::Vec2; 3] = [a, b, c];
        let center = Self::get_center(a, b, c);
        array.sort_by(|u, v| {
            if Self::is_point_less_in_clockwise(&center, u, v) {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });
        *a = array[0];
        *b = array[1];
        *c = array[2];
    }

    fn is_point_less_in_clockwise(center: &glm::Vec2, a: &glm::Vec2, b: &glm::Vec2) -> bool {
        if a.x - center.x >= 0.0 && b.x - center.x < 0.0 {
            return true;
        }

        if a.x - center.x < 0.0 && b.x - center.x >= 0.0 {
            return false;
        }

        if a.x - center.x == 0.0 && b.x - center.x == 0.0 {
            if a.y - center.y >= 0.0 || b.y - center.y >= 0.0 {
                return a.y > b.y;
            }
            return b.y > a.y;
        }

        let det: i32 = ((a.x - center.x) * (b.y - center.y) - (b.x - center.x) * (a.y - center.y)) as i32;

        if det != 0 {
            return det < 0;
        }

        let d1: i32 = ((a.x - center.x) * (a.x - center.x) + (a.y - center.y) * (a.y - center.y)) as i32;
        let d2: i32 = ((b.x - center.x) * (b.x - center.x) + (b.y - center.y) * (b.y - center.y)) as i32;
        d1 > d2
    }

    fn get_distance_between_point_and_triangle(point: &glm::Vec2, a: &glm::Vec2, b: &glm::Vec2, c: &glm::Vec2) -> f32 {
        let mut a = a;
        let mut b = b;
        let mut c = c;

        Self::ensure_counter_clockwise(&mut a, &mut b, &mut c);

        let ab_width = glm::distance(a, b);
        let bc_width = glm::distance(b, c);
        let ca_width = glm::distance(c, a);

        fn get_distance_to_side(point: &glm::Vec2, p1: &glm::Vec2, p2: &glm::Vec2, side_width: f32) -> f32 {
            ((p2.y - p1.y) * point.x - (p2.x - p1.x) * point.y + p2.x * p1.y - p2.y * p1.x) / side_width
        }

        let ab_distance = get_distance_to_side(point, a, b, ab_width);
        let bc_distance = get_distance_to_side(point, b, c, bc_width);
        let ca_distance = get_distance_to_side(point, c, a, ca_width);
        ab_distance.max(bc_distance).max(ca_distance)
    }
}

impl Squid for Tri {
    fn render(&mut self, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
        let TriData {
            center,
            p1,
            p2,
            p3,
            rotation,
            color,
            ..
        } = self.data.get_animated();

        let p1 = p1.reveal() + center.reveal();
        let p2 = p2.reveal() + center.reveal();
        let p3 = p3.reveal() + center.reveal();

        self.refresh_mesh(ctx.display);

        let center = self.get_animated_center();
        let mut transformation = glm::identity::<f32, 4>();

        if let Some(preview) = &as_preview {
            let max_distance = glm::distance(&p1, &center).max(glm::distance(&p2, &center).max(glm::distance(&p3, &center)));
            let factor = 1.0.div_or_zero(max_distance);

            transformation = glm::translate(&transformation, &glm::vec2_to_vec3(&preview.position));
            transformation = glm::scale(&transformation, &glm::vec3(factor * preview.size, factor * preview.size, 0.0));
        } else {
            transformation = glm::translation(&glm::vec2_to_vec3(&center));
        }

        transformation = glm::rotate(&transformation, rotation.scalar(), &glm::vec3(0.0, 0.0, -1.0));

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
        let TriData { .. } = self.data.get_animated();
        let center = self.get_animated_center();

        let center_in_world = camera.apply(&center);

        ctx.ring_mesh.render(
            ctx,
            center_in_world.x,
            center_in_world.y,
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

        for point in &self.get_animated_screen_points(camera) {
            ctx.ring_mesh
                .render(ctx, point.x, point.y, squid::HANDLE_RADIUS, squid::HANDLE_RADIUS, &ctx.color_scheme.foreground);
        }
    }

    fn interact(&mut self, interaction: &Interaction, camera: &Camera, _options: &InteractionOptions) -> Capture {
        match interaction {
            Interaction::PreClick => {
                self.translate_behavior.moving = false;
                self.rotating = false;
                self.moving_point = None;
            }
            Interaction::Click(ClickInteraction {
                button: MouseButton::Left,
                position,
            }) => {
                for (i, corner) in self.get_animated_screen_points(camera).iter().enumerate() {
                    if glm::distance(position, corner) <= squid::HANDLE_RADIUS * 2.0 {
                        self.moving_point = Some(i);
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
                if self.moving_point.is_some() {
                    self.reposition_point(mouse_position, camera);
                } else if self.rotating {
                    return Capture::RotateSelectedSquids {
                        delta_theta: squid::behavior::get_delta_rotation(
                            &self.get_real_center(),
                            self.data.get_real().rotation + self.virtual_rotation,
                            mouse_position,
                            &self.rotation_accumulator,
                            camera,
                        ),
                    };
                } else if self.translate_behavior.moving {
                    return Capture::MoveSelectedSquids {
                        delta_in_world: camera.apply_reverse_to_vector(delta),
                    };
                }
            }
            Interaction::MouseRelease(MouseReleaseInteraction { button: MouseButton::Left, .. }) => {
                self.rotating = false;
                self.moving_point = None;
                self.translate_behavior.accumulator.clear();
                self.rotation_accumulator.clear();
            }
            _ => (),
        }
        Capture::Miss
    }

    fn is_point_over(&self, underneath: &glm::Vec2, camera: &Camera) -> bool {
        let real = self.data.get_real();
        let center = self.get_real_center();
        let p1 = glm::rotate_vec2(&(real.p1.reveal()), -real.rotation.scalar()) + center;
        let p2 = glm::rotate_vec2(&(real.p2.reveal()), -real.rotation.scalar()) + center;
        let p3 = glm::rotate_vec2(&(real.p3.reveal()), -real.rotation.scalar()) + center;
        let p1 = camera.apply(&p1);
        let p2 = camera.apply(&p2);
        let p3 = camera.apply(&p3);
        Self::is_point_inside_triangle(underneath, &p1, &p2, &p3)
    }

    fn translate(&mut self, raw_delta_in_world: &glm::Vec2, options: &InteractionOptions) {
        let delta = self.translate_behavior.express(raw_delta_in_world, options);

        if delta != glm::zero::<glm::Vec2>() {
            let mut new_data = *self.data.get_real();
            new_data.center = MultiLerp::Linear(new_data.center.reveal() + delta);
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
        new_data.p1 = MultiLerp::Linear(self.prescale_size[0] * total_scale_factor);
        new_data.p2 = MultiLerp::Linear(self.prescale_size[1] * total_scale_factor);
        new_data.p3 = MultiLerp::Linear(self.prescale_size[2] * total_scale_factor);
        self.data.set(new_data);
    }

    fn spread(&mut self, point: &glm::Vec2, _options: &InteractionOptions) {
        let new_position = self.spread_behavior.express(point);
        let delta = new_position - self.get_real_center();

        let mut new_data = *self.data.get_real();
        new_data.center = MultiLerp::Linear(new_data.center.reveal() + delta);
        self.data.set(new_data);
    }

    fn revolve(&mut self, current: &glm::Vec2, options: &InteractionOptions) {
        if let Some(expression) = self.revolve_behavior.express(current, options) {
            let center = self.get_animated_center();
            let new_center = expression.apply_origin_rotation_to_center();
            let delta_position = new_center - center;
            let animated_data = self.data.get_animated();
            let mut new_data = *self.data.get_real();
            new_data.center = MultiLerp::Circle(animated_data.center.reveal() + delta_position, expression.origin);
            new_data.rotation += expression.delta_object_rotation;
            self.data.set(new_data);
        }
    }

    fn dilate(&mut self, point: &glm::Vec2, _options: &InteractionOptions) {
        let expression = self.dilate_behavior.express(point);
        let delta = expression.position - self.get_real_center();

        let mut new_data = *self.data.get_real();
        new_data.center = MultiLerp::Linear(new_data.center.reveal() + delta);
        new_data.p1 = MultiLerp::Linear(expression.total_scale_factor * self.prescale_size[0]);
        new_data.p2 = MultiLerp::Linear(expression.total_scale_factor * self.prescale_size[1]);
        new_data.p3 = MultiLerp::Linear(expression.total_scale_factor * self.prescale_size[2]);
        self.data.set(new_data);
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
        real.center = MultiLerp::From(real.center.reveal() + offset);
        Box::new(Self::from_data(real))
    }

    fn get_creation_time(&self) -> Instant {
        self.created
    }

    fn initiate(&mut self, initiation: Initiation) {
        match initiation {
            Initiation::Translate => {
                self.translate_behavior.moving = true;
                self.moving_point = None;
            }
            Initiation::Rotate => (),
            Initiation::Scale => {
                let real = self.data.get_real();
                self.prescale_size = [real.p1.reveal(), real.p2.reveal(), real.p3.reveal()];
            }
            Initiation::Spread { point, center } => {
                self.spread_behavior = SpreadBehavior {
                    point,
                    origin: center,
                    start: self.get_center(),
                };
            }
            Initiation::Revolve { point, center } => self.revolve_behavior.set(&center, &self.get_center(), &point),
            Initiation::Dilate { point, center } => {
                let real = self.data.get_real();
                self.prescale_size = [real.p1.reveal(), real.p2.reveal(), real.p3.reveal()];
                self.dilate_behavior = DilateBehavior {
                    point,
                    origin: center,
                    start: self.get_center(),
                };
            }
        }
    }

    fn get_center(&self) -> glm::Vec2 {
        self.get_animated_center()
    }

    fn get_name(&self) -> &str {
        self.name.as_deref().unwrap_or("Unnamed Tri")
    }

    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    fn get_opaque_handles(&self) -> Vec<glm::Vec2> {
        let data = self.data.get_animated();
        let center = data.center.reveal();
        vec![
            data.p1.reveal() + center,
            data.p2.reveal() + center,
            data.p3.reveal() + center,
            self.get_rotate_handle_location(&Camera::identity(glm::zero())),
        ]
    }
}
