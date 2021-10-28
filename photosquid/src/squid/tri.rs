use super::{Initiation, Squid, SquidRef};
use crate::{
    accumulator::Accumulator,
    app::InteractionOptions,
    color::Color,
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    matrix_helpers::reach_inside_mat4,
    mesh::MeshXyz,
    ocean::{NewSelection, NewSelectionInfo, Selection},
    render_ctx::RenderCtx,
    smooth::{Lerpable, Smooth},
    squid,
    tool::{Capture, Interaction},
};
use glium::{glutin::event::MouseButton, Display};
use nalgebra_glm as glm;
use std::time::{Duration, Instant};

pub struct Tri {
    data: Smooth<TriData>,
    created: Instant,
    mesh: Option<MeshXyz>,

    // Keep track of which points the mesh is made of,
    // so that we know when we have to re-create it
    mesh_p1: glm::Vec2,
    mesh_p2: glm::Vec2,
    mesh_p3: glm::Vec2,

    // Tweaking parameters
    moving_point: Option<usize>, // (zero indexed)
    moving: bool,
    rotating: bool,
    translation_accumulator: Accumulator<glm::Vec2>,
    rotation_accumulator: Accumulator<f32>,
    virtual_rotation: f32, // Rotation that only applies to the handle
}

#[derive(Copy, Clone)]
struct TriData {
    p1: glm::Vec2,
    p2: glm::Vec2,
    p3: glm::Vec2,
    color: Color,
    rotation: f32,
}

impl Lerpable for TriData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            p1: Lerpable::lerp(&self.p1, &other.p1, scalar),
            p2: Lerpable::lerp(&self.p2, &other.p2, scalar),
            p3: Lerpable::lerp(&self.p3, &other.p3, scalar),
            rotation: interpolation::Lerp::lerp(&self.rotation, &other.rotation, scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
        }
    }
}

impl Tri {
    pub fn new(p1: glm::Vec2, p2: glm::Vec2, p3: glm::Vec2, rotation: f32, color: Color) -> Self {
        let data = TriData { p1, p2, p3, rotation, color };

        Self {
            data: Smooth::new(data, Duration::from_millis(500)),
            created: Instant::now(),
            mesh: None,
            mesh_p1: p1,
            mesh_p2: p2,
            mesh_p3: p3,
            moving_point: None,
            moving: false,
            rotating: false,
            translation_accumulator: Accumulator::new(),
            rotation_accumulator: Accumulator::new(),
            virtual_rotation: 0.0,
        }
    }

    fn refresh_mesh(&mut self, display: &Display) {
        let TriData { p1, p2, p3, .. } = self.data.get_animated();

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

        let d1 = sign(&p, &p1, &p2);
        let d2 = sign(&p, &p2, &p3);
        let d3 = sign(&p, &p3, &p1);

        let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;

        !(has_neg && has_pos)
    }

    fn get_real_center(&self) -> glm::Vec2 {
        let TriData { p1, p2, p3, .. } = self.data.get_real();
        Self::get_center(p1, p2, p3)
    }

    fn get_animated_center(&self) -> glm::Vec2 {
        let TriData { p1, p2, p3, .. } = self.data.get_animated();
        Self::get_center(&p1, &p2, &p3)
    }

    fn get_center(p1: &glm::Vec2, p2: &glm::Vec2, p3: &glm::Vec2) -> glm::Vec2 {
        (p1 + p2 + p3) / 3.0
    }

    fn get_animated_screen_points(&self, camera: &glm::Vec2) -> Vec<glm::Vec2> {
        let TriData { p1, p2, p3, rotation, .. } = self.data.get_animated();
        let center = self.get_animated_center();

        [p1 - center, p2 - center, p3 - center]
            .iter()
            .map(|p| glm::rotate_vec2(p, -rotation) + center + camera)
            .collect()
    }

    fn get_rotate_handle_location(&self, camera: &glm::Vec2) -> glm::Vec2 {
        let center = self.get_animated_center();
        let TriData { p1, p2, p3, rotation, .. } = self.data.get_animated();
        let rotation = rotation + self.virtual_rotation;

        let max_distance = glm::distance(&p1, &center).max(glm::distance(&p2, &center).max(glm::distance(&p3, &center)));

        let first_try = glm::vec2(
            center.x + camera.x + rotation.cos() * (max_distance + 24.0),
            center.y + camera.y - rotation.sin() * (max_distance + 24.0),
        );

        let screen_points = self.get_animated_screen_points(camera);
        assert_eq!(screen_points.len(), 3);

        let r_p1 = &screen_points[0];
        let r_p2 = &screen_points[1];
        let r_p3 = &screen_points[2];
        let true_distance = Self::get_distance_between_point_and_triangle(&first_try, r_p1, r_p2, r_p3);

        let final_distance = (max_distance + 24.0 - true_distance) + 24.0;

        glm::vec2(
            center.x + camera.x + rotation.cos() * final_distance,
            center.y + camera.y - rotation.sin() * final_distance,
        )
    }

    fn get_delta_rotation(&self, mouse_position: &glm::Vec2, camera: &glm::Vec2) -> f32 {
        let real = self.data.get_real();
        let center = self.get_real_center();
        let screen_center = center + camera;

        let old_rotation = real.rotation + self.rotation_accumulator.residue() + self.virtual_rotation;
        let new_rotation = -1.0 * (mouse_position.y - screen_center.y).atan2(mouse_position.x - screen_center.x);
        return squid::angle_difference(old_rotation, new_rotation);
    }

    fn reposition_point(&mut self, mouse_position: &glm::Vec2, camera: &glm::Vec2) {
        let real = self.data.get_real();
        let rotation = self.data.get_real().rotation;
        let center = self.get_real_center();

        let mut p1 = glm::rotate_vec2(&(real.p1 - center), -rotation);
        let mut p2 = glm::rotate_vec2(&(real.p2 - center), -rotation);
        let mut p3 = glm::rotate_vec2(&(real.p3 - center), -rotation);

        let new_p = glm::rotate_vec2(&(mouse_position - camera - center), 0.0);

        match self.moving_point {
            Some(0) => p1 = new_p,
            Some(1) => p2 = new_p,
            Some(2) => p3 = new_p,
            _ => (),
        }

        p1 += center;
        p2 += center;
        p3 += center;

        // Set new data as the new target points, with zero rotation applied

        // HACK: Instantly snap rotation back to 0.0
        if real.rotation != 0.0 {
            self.virtual_rotation += real.rotation;

            {
                let mut_real = self.data.manual_get_real();
                mut_real.p1 = p1;
                mut_real.p2 = p2;
                mut_real.p3 = p3;
                mut_real.rotation = 0.0;
            }

            {
                let mut_previous = self.data.manual_get_previous();
                mut_previous.p1 = p1;
                mut_previous.p2 = p2;
                mut_previous.p3 = p3;
                mut_previous.rotation = 0.0;
            }
        } else {
            let mut new_real = *self.data.get_real();
            new_real.p1 = p1;
            new_real.p2 = p2;
            new_real.p3 = p3;
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

        let ab_width = glm::distance(&a, &b);
        let bc_width = glm::distance(&b, &c);
        let ca_width = glm::distance(&c, &a);

        fn get_distance_to_side(point: &glm::Vec2, p1: &glm::Vec2, p2: &glm::Vec2, side_width: f32) -> f32 {
            ((p2.y - p1.y) * point.x - (p2.x - p1.x) * point.y + p2.x * p1.y - p2.y * p1.x) / side_width
        }

        let ab_distance = get_distance_to_side(point, &a, &b, ab_width);
        let bc_distance = get_distance_to_side(point, &b, &c, bc_width);
        let ca_distance = get_distance_to_side(point, &c, &a, ca_width);
        ab_distance.max(bc_distance).max(ca_distance)
    }
}

impl Squid for Tri {
    // Renders squid in regular state
    fn render(&mut self, ctx: &mut RenderCtx) {
        let TriData { rotation, color, .. } = self.data.get_animated();

        self.refresh_mesh(ctx.display);

        let center = self.get_animated_center();
        let transformation = glm::translation(&glm::vec2_to_vec3(&center));
        let transformation = glm::rotate(&transformation, rotation, &glm::vec3(0.0, 0.0, -1.0));
        let transformation = glm::translate(&transformation, &glm::vec2_to_vec3(&(-center)));

        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: reach_inside_mat4(ctx.view),
            projection: reach_inside_mat4(ctx.projection),
            color: Into::<[f32; 4]>::into(color)
        };

        let mesh = self.mesh.as_ref().unwrap();
        ctx.draw(&mesh.vertex_buffer, &mesh.indices, ctx.color_shader, &uniforms, &Default::default())
            .unwrap();
    }

    // Render additional selection indicators and helpers for when
    // the squid is selected
    fn render_selected_indication(&self, ctx: &mut RenderCtx) {
        let camera = ctx.camera;
        let TriData { .. } = self.data.get_animated();
        let center = self.get_animated_center();

        ctx.ring_mesh.render(
            ctx,
            center.x + camera.x,
            center.y + camera.y,
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

        for point in self.get_animated_screen_points(camera).iter() {
            ctx.ring_mesh
                .render(ctx, point.x, point.y, squid::HANDLE_RADIUS, squid::HANDLE_RADIUS, &ctx.color_scheme.foreground);
        }
    }

    // Called when squid is selected and has opportunity to capture
    // user interaction
    // Returns if and how the interaction was captured
    fn interact(&mut self, interaction: &Interaction, camera: &glm::Vec2, _options: &InteractionOptions) -> Capture {
        match interaction {
            Interaction::Click {
                button: MouseButton::Left,
                position,
            } => {
                self.moving = false;
                self.rotating = false;
                self.moving_point = None;

                for (i, corner) in self.get_animated_screen_points(camera).iter().enumerate() {
                    if glm::distance(position, &corner) <= squid::HANDLE_RADIUS * 2.0 {
                        self.moving_point = Some(i);
                        return Capture::AllowDrag;
                    }
                }

                let rotate_handle_location = self.get_rotate_handle_location(camera);
                if glm::distance(position, &rotate_handle_location) <= squid::HANDLE_RADIUS * 3.0 {
                    self.rotating = true;
                    return Capture::AllowDrag;
                }

                if self.is_point_over(&position, camera) {
                    self.moving = true;
                    return Capture::AllowDrag;
                }
            }
            Interaction::Drag { delta, current, .. } => {
                if self.moving_point.is_some() {
                    self.reposition_point(current, camera);
                } else if self.rotating {
                    return Capture::RotateSelectedSquids {
                        delta_theta: self.get_delta_rotation(current, camera),
                    };
                } else if self.moving {
                    return Capture::MoveSelectedSquids { delta: *delta };
                }
            }
            Interaction::MouseRelease { button: MouseButton::Left, .. } => {
                self.rotating = false;
                self.translation_accumulator.clear();
                self.rotation_accumulator.clear();
            }
            _ => (),
        }
        Capture::Miss
    }

    // Returns whether a point is over this squid
    fn is_point_over(&self, underneath: &glm::Vec2, camera: &glm::Vec2) -> bool {
        let real = self.data.get_real();
        let center = self.get_real_center();
        let p1 = glm::rotate_vec2(&(real.p1 - center), -real.rotation) + center + camera;
        let p2 = glm::rotate_vec2(&(real.p2 - center), -real.rotation) + center + camera;
        let p3 = glm::rotate_vec2(&(real.p3 - center), -real.rotation) + center + camera;
        Self::is_point_inside_triangle(underneath, &p1, &p2, &p3)
    }

    // Moves a squid body
    fn translate(&mut self, raw_delta: &glm::Vec2, options: &InteractionOptions) {
        if let Some(delta) = self.translation_accumulator.accumulate(raw_delta, options.translation_snapping) {
            let mut new_data = *self.data.get_real();
            new_data.p1 += delta;
            new_data.p2 += delta;
            new_data.p3 += delta;
            self.data.set(new_data);
        }
    }

    // Rotates a squid body
    fn rotate(&mut self, raw_delta_theta: f32, options: &InteractionOptions) {
        if let Some(delta_theta) = self.rotation_accumulator.accumulate(&raw_delta_theta, options.rotation_snapping) {
            let mut new_data = *self.data.get_real();
            new_data.rotation += delta_theta;
            self.data.set(new_data);
        }
    }

    // Attempts to get a selection for this squid or a selection for a limb of this squid
    // under the point (x, y)
    fn try_select(&mut self, underneath: &glm::Vec2, camera: &glm::Vec2, self_reference: SquidRef) -> Option<NewSelection> {
        if self.is_point_over(underneath, camera) {
            self.moving = true;

            Some(NewSelection {
                selection: Selection::new(self_reference, None),
                info: NewSelectionInfo {
                    color: Some(self.data.get_real().color),
                },
            })
        } else {
            None
        }
    }

    // Attempt to get a context menu for if a quid is underneath a point
    fn try_context_menu(&self, underneath: &glm::Vec2, camera: &glm::Vec2, _self_reference: SquidRef, color_scheme: &ColorScheme) -> Option<ContextMenu> {
        if self.is_point_over(underneath, camera) {
            Some(squid::common_context_menu(underneath, color_scheme))
        } else {
            None
        }
    }

    // Attempts to set the color of a squid
    fn set_color(&mut self, color: Color) {
        let mut new_data = *self.data.get_real();
        new_data.color = color;
        self.data.set(new_data);
    }

    // Duplicates a squid
    fn duplicate(&self, offset: &glm::Vec2) -> Box<dyn Squid> {
        let real = self.data.get_real();
        Box::new(Self::new(real.p1 + offset, real.p2 + offset, real.p3 + offset, real.rotation, real.color))
    }

    // Gets the creation time of a squid (used for ordering)
    fn get_creation_time(&self) -> Instant {
        self.created
    }

    fn initiate(&mut self, initiation: Initiation) {
        match initiation {
            Initiation::TRANSLATION => self.moving = true,
        }
    }
}
