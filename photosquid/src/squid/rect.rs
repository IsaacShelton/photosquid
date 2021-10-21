use super::{Squid, SquidRef};
use crate::{
    accumulator::Accumulator,
    algorithm,
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

pub struct Rect {
    data: Smooth<RectData>,
    created: Instant,
    mesh: MeshXyz,

    // Tweaking parameters
    moving_corner: Option<CornerKind>,
    moving: bool,
    rotating: bool,
    translation_accumulator: Accumulator<glm::Vec2>,
    rotation_accumulator: Accumulator<f32>,
}

#[derive(Copy, Clone)]
struct RectData {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    rotation: f32,
}

impl Lerpable for RectData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            x: interpolation::Lerp::lerp(&self.x, &other.x, scalar),
            y: interpolation::Lerp::lerp(&self.y, &other.y, scalar),
            w: interpolation::Lerp::lerp(&self.w, &other.w, scalar),
            h: interpolation::Lerp::lerp(&self.h, &other.h, scalar),
            rotation: interpolation::Lerp::lerp(&self.rotation, &other.rotation, scalar),
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
    pub fn new(x: f32, y: f32, w: f32, h: f32, rotation: f32, color: Color, display: &Display) -> Self {
        let data = RectData {
            x,
            y,
            w,
            h,
            rotation: rotation,
            color,
        };

        Self {
            data: Smooth::new(data, Duration::from_millis(500)),
            created: Instant::now(),
            mesh: MeshXyz::new_shape_square(display),
            moving_corner: None,
            moving: false,
            rotating: false,
            translation_accumulator: Accumulator::new(),
            rotation_accumulator: Accumulator::new(),
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
            .map(|p| glm::rotate_vec2(&p, -1.0 * rotation))
            .collect()
    }

    fn get_screen_corners(&self, camera: &glm::Vec2) -> Vec<glm::Vec2> {
        let RectData { x, y, .. } = self.data.get_animated();

        self.get_relative_corners()
            .iter()
            .map(|p| glm::vec2(p.x + x + camera.x, p.y + y + camera.y))
            .collect()
    }

    fn reposition_corner(&mut self, target_screen_position: &glm::Vec2, camera: &glm::Vec2) {
        use CornerDependence::*;
        use CornerKind::*;

        let real = self.data.get_real();
        let moving_corner = self.moving_corner.unwrap();
        let relative_corners = self.get_relative_corners();
        let target_position = *target_screen_position - camera - glm::vec2(real.x, real.y);

        // Rotate corners back into axis-aligned space
        let rotation = real.rotation;
        let axis_aligned_relative_corners: Vec<glm::Vec2> = relative_corners.iter().map(|p| glm::rotate_vec2(&p, rotation)).collect();
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

    fn get_delta_rotation(&self, mouse_position: &glm::Vec2, camera: &glm::Vec2) -> f32 {
        let real = self.data.get_real();
        let screen_x = real.x + camera.x;
        let screen_y = real.y + camera.y;

        let old_rotation = real.rotation + self.rotation_accumulator.residue();
        let new_rotation = -1.0 * (mouse_position.y - screen_y).atan2(mouse_position.x - screen_x) + if real.w < 0.0 { std::f32::consts::PI } else { 0.0 };
        return squid::angle_difference(old_rotation, new_rotation);
    }

    fn get_rotate_handle_location(&self, camera: &glm::Vec2) -> glm::Vec2 {
        let RectData { x, y, w, rotation, .. } = self.data.get_animated();

        glm::vec2(
            x + camera.x + rotation.cos() * (w * 0.5 + 24.0 * w.signum()),
            y + camera.y - rotation.sin() * (w * 0.5 + 24.0 * w.signum()),
        )
    }
}

impl Squid for Rect {
    fn render(&mut self, ctx: &mut RenderCtx) {
        let _t = (Instant::now() - self.created).as_millis() as f32 / 1000.0;
        let RectData { x, y, w, h, rotation, color } = self.data.get_animated();

        let transformation = glm::translation(&glm::vec3(x, y, 0.0));
        let transformation = glm::rotate(&transformation, rotation, &glm::vec3(0.0, 0.0, -1.0));
        let transformation = glm::scale(&transformation, &glm::vec3(w, h, 0.0));

        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: reach_inside_mat4(ctx.view),
            projection: reach_inside_mat4(ctx.projection),
            color: Into::<[f32; 4]>::into(color)
        };

        ctx.draw(&self.mesh.vertex_buffer, &self.mesh.indices, ctx.color_shader, &uniforms, &Default::default())
            .unwrap();
    }

    fn render_selected_indication(&self, ctx: &mut RenderCtx) {
        let camera = ctx.camera;
        let RectData { x, y, .. } = self.data.get_animated();

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

        for corner in self.get_relative_corners().iter() {
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
            Interaction::Click {
                button: MouseButton::Left,
                position,
            } => {
                self.moving = false;
                self.rotating = false;
                self.moving_corner = None;

                for (i, corner) in self.get_screen_corners(camera).iter().enumerate() {
                    if glm::distance(position, &corner) <= squid::HANDLE_RADIUS * 2.0 {
                        self.moving_corner = Some(Self::get_corner_kind(i));
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
                if self.moving_corner.is_some() {
                    self.reposition_corner(current, camera);
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
                self.moving_corner = None;
                self.translation_accumulator.clear();
                self.rotation_accumulator.clear();
            }
            _ => (),
        }

        Capture::Miss
    }

    fn translate(&mut self, raw_delta: &glm::Vec2, options: &InteractionOptions) {
        if let Some(delta) = self.translation_accumulator.accumulate(raw_delta, options.translation_snapping) {
            let mut new_data = *self.data.get_real();
            new_data.x += delta.x;
            new_data.y += delta.y;
            self.data.set(new_data);
        }
    }

    fn rotate(&mut self, raw_delta_theta: f32, options: &InteractionOptions) {
        if let Some(delta_theta) = self.rotation_accumulator.accumulate(&raw_delta_theta, options.rotation_snapping) {
            let mut new_data = *self.data.get_real();
            new_data.rotation += delta_theta;
            self.data.set(new_data);
        }
    }

    fn is_point_over(&self, underneath: &glm::Vec2, camera: &glm::Vec2) -> bool {
        let real = self.data.get_real();
        let corners: Vec<glm::Vec2> = self
            .get_relative_corners()
            .iter()
            .map(|&p| glm::vec2(real.x + camera.x + p.x, real.y + camera.y + p.y))
            .collect();

        assert_eq!(corners.len(), 4);
        algorithm::is_point_inside_rectangle(corners[0], corners[1], corners[2], corners[3], *underneath)
    }

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

    fn try_context_menu(&self, underneath: &glm::Vec2, camera: &glm::Vec2, _self_reference: SquidRef, color_scheme: &ColorScheme) -> Option<ContextMenu> {
        if self.is_point_over(underneath, camera) {
            Some(squid::common_context_menu(underneath, color_scheme))
        } else {
            None
        }
    }

    fn set_color(&mut self, color: Color) {
        let mut new_data = *self.data.get_real();
        new_data.color = color;
        self.data.set(new_data);
    }

    fn duplicate(&self, offset: &glm::Vec2, display: &Display) -> Box<dyn Squid> {
        let real = self.data.get_real();
        Box::new(Self::new(
            real.x + offset.x,
            real.y + offset.y,
            real.w,
            real.h,
            real.rotation,
            real.color,
            display,
        ))
    }

    fn get_creation_time(&self) -> Instant {
        self.created
    }
}
