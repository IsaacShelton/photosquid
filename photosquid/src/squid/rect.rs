use super::{
    behavior::{self, DilateBehavior, RevolveBehavior, SpreadBehavior, TranslateBehavior},
    PreviewParams, HANDLE_RADIUS,
};
use crate::{
    accumulator::Accumulator,
    algorithm,
    camera::Camera,
    capture::Capture,
    color::Color,
    interaction::{ClickInteraction, DragInteraction, Interaction, MouseReleaseInteraction},
    math_helpers::DivOrZero,
    matrix_helpers,
    mesh::MeshXyz,
    render_ctx::RenderCtx,
    smooth::{Lerpable, MultiLerp, NoLerp, Smooth},
};
use angular_units::{Angle, Rad};
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;

pub struct Rect {
    pub mesh: Option<MeshXyz>,
    pub data: Smooth<RectData>,

    // Move point
    pub moving_corner: Option<Corner>,

    // Translate
    pub translate_behavior: TranslateBehavior,

    // Rotate
    pub rotating: bool,
    pub rotation_accumulator: Accumulator<Rad<f32>>,

    // Scale
    pub prescale_size: glm::Vec2,

    // Spread
    pub spread_behavior: SpreadBehavior,

    // Revolve
    pub revolve_behavior: RevolveBehavior,

    // Dilate
    pub dilate_behavior: DilateBehavior,
}

#[derive(Copy, Clone)]
pub struct RectData {
    pub position: MultiLerp<glm::Vec2>,
    pub size: glm::Vec2,
    pub color: NoLerp<Color>,
    pub rotation: Rad<f32>,
    pub radii: f32,
}

impl Lerpable for RectData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            position: self.position.lerp(&other.position, scalar),
            size: Lerpable::lerp(&self.size, &other.size, scalar),
            rotation: angular_units::Interpolate::interpolate(&self.rotation, &other.rotation, *scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
            radii: interpolation::Lerp::lerp(&self.radii, &other.radii, scalar),
        }
    }
}

#[derive(Copy, Clone)]
pub enum Corner {
    ZeroZero = 3,
    ZeroY = 1,
    XZero = 2,
    XY = 0,
}

fn get_corner_kind(corner_index: usize) -> Corner {
    use Corner::*;
    [XY, ZeroY, XZero, ZeroZero][corner_index]
}

pub fn get_rotate_handle(rect: &Rect, camera: &Camera) -> glm::Vec2 {
    let RectData { position, size, rotation, .. } = rect.data.get_animated();
    let w = size.x;

    let world_position = glm::vec2(
        position.reveal().x + rotation.cos() * (w * 0.5 + 24.0 * w.signum()),
        position.reveal().y - rotation.sin() * (w * 0.5 + 24.0 * w.signum()),
    );

    camera.apply(&world_position)
}

pub fn get_relative_corners(rect: &Rect) -> Vec<glm::Vec2> {
    // Use non-animated version always for now?
    // It seems to look better this way
    let RectData { size, .. } = rect.data.get_animated();
    let RectData { rotation, .. } = rect.data.get_animated();

    [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0f32)]
        .iter()
        .map(|&p| glm::vec2(p.0 * size.x / 2.0, p.1 * size.y / 2.0))
        .map(|p| glm::rotate_vec2(&p, -1.0 * rotation.scalar()))
        .collect()
}

pub fn get_world_corners(rect: &Rect) -> Vec<glm::Vec2> {
    let RectData { position, .. } = rect.data.get_animated();

    get_relative_corners(rect).iter().map(|p| p + position.reveal()).collect()
}

fn get_screen_corners(rect: &Rect, camera: &Camera) -> Vec<glm::Vec2> {
    let RectData { position, .. } = rect.data.get_animated();

    get_relative_corners(rect).iter().map(|p| camera.apply(&(p + position.reveal()))).collect()
}

fn reposition_corner(rect: &mut Rect, mouse: &glm::Vec2, camera: &Camera) {
    let real = rect.data.get_real();
    let rotation = real.rotation.scalar();
    let mouse_in_world = camera.apply_reverse(mouse);
    let abs_size = 2.0 * glm::rotate_vec2(&(real.position.reveal() - mouse_in_world), rotation);

    let new_size = abs_size.component_mul(&match rect.moving_corner.unwrap() {
        Corner::ZeroZero => glm::vec2(1.0, 1.0),
        Corner::XZero => glm::vec2(-1.0, 1.0),
        Corner::ZeroY => glm::vec2(1.0, -1.0),
        Corner::XY => glm::vec2(-1.0, -1.0),
    });

    let mut new_data = *real;
    new_data.size = new_size;
    rect.data.set(new_data);
    rect.mesh = None;
}

pub fn is_point_over(rect: &Rect, mouse_position: glm::Vec2, camera: &Camera) -> bool {
    let underneath = camera.apply_reverse(&mouse_position);

    let corners: Vec<glm::Vec2> = get_world_corners(rect);
    assert_eq!(corners.len(), 4);
    algorithm::is_point_inside_rectangle(corners[0], corners[1], corners[2], corners[3], underneath)
}

pub fn interact(rect: &mut Rect, interaction: &Interaction, camera: &Camera) -> Capture {
    match interaction {
        Interaction::PreClick => {
            rect.translate_behavior.moving = false;
            rect.rotating = false;
            rect.moving_corner = None;
        }
        Interaction::Click(ClickInteraction {
            button: MouseButton::Left,
            position,
        }) => {
            for (i, corner) in get_screen_corners(rect, camera).iter().enumerate() {
                if glm::distance(position, corner) <= HANDLE_RADIUS * 2.0 {
                    rect.moving_corner = Some(get_corner_kind(i));
                    return Capture::AllowDrag;
                }
            }

            let rotate_handle_location = get_rotate_handle(rect, camera);
            if glm::distance(position, &rotate_handle_location) <= HANDLE_RADIUS * 2.0 {
                rect.rotating = true;
                return Capture::AllowDrag;
            }

            if is_point_over(rect, *position, camera) {
                rect.translate_behavior.moving = true;
                return Capture::AllowDrag;
            }
        }
        Interaction::Drag(DragInteraction {
            delta,
            current: mouse_position,
            ..
        }) => {
            if rect.moving_corner.is_some() {
                reposition_corner(rect, mouse_position, camera);
            } else if rect.rotating {
                // When the rectangle's width is negative, the rotation handle is PI radians ahead of it's angle
                // compared to the actual rotation of the shape,
                // So we have to compensate for the difference when that's the case.
                // It might be better to integrate this difference into the rotation and not allow negative
                // widths/heights for rectangles, but this is an easier way to do it
                let compensation = Rad(if rect.data.get_animated().size.x < 0.0 { std::f32::consts::PI } else { 0.0 });

                return Capture::RotateSelectedSquids {
                    delta_theta: behavior::get_delta_rotation(
                        &rect.data.get_real().position.reveal(),
                        rect.data.get_real().rotation,
                        mouse_position,
                        &rect.rotation_accumulator,
                        camera,
                    ) + compensation,
                };
            } else if rect.translate_behavior.moving {
                return Capture::MoveSelectedSquids {
                    delta_in_world: camera.apply_reverse_to_vector(delta),
                };
            }
        }
        Interaction::MouseRelease(MouseReleaseInteraction { button: MouseButton::Left, .. }) => {
            rect.rotating = false;
            rect.moving_corner = None;
            rect.translate_behavior.accumulator.clear();
            rect.rotation_accumulator.clear();
        }
        _ => (),
    }

    Capture::Miss
}

pub fn render(rect: &mut Rect, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
    let RectData {
        position,
        size,
        rotation,
        color,
        ..
    } = rect.data.get_animated();

    // Refresh mesh
    {
        let real = rect.data.get_real();
        let animated = rect.data.get_animated();

        // Don't use margin of error
        if rect.mesh.is_none() || real.radii != animated.radii || real.size != animated.size {
            rect.mesh = Some(MeshXyz::new_rect(ctx.display, animated.size, animated.radii));
        }
    }

    let render_position = if let Some(preview) = &as_preview {
        preview.position
    } else {
        position.reveal()
    };

    let mut transformation = glm::translation(&glm::vec2_to_vec3(&render_position));
    transformation = glm::rotate(&transformation, rotation.scalar(), &glm::vec3(0.0, 0.0, -1.0));

    if let Some(preview) = &as_preview {
        let max_size = size.abs().max();
        let factor = 1.0.div_or_zero(max_size);
        transformation = glm::scale(&transformation, &glm::vec3(factor * preview.size, factor * preview.size, 0.0));
    }

    let raw_view = if as_preview.is_some() {
        matrix_helpers::reach_inside_mat4(&glm::identity::<f32, 4>())
    } else {
        matrix_helpers::reach_inside_mat4(ctx.view)
    };

    let uniforms = glium::uniform! {
        transformation: matrix_helpers::reach_inside_mat4(&transformation),
        view: raw_view,
        projection: matrix_helpers::reach_inside_mat4(ctx.projection),
        color: Into::<[f32; 4]>::into(color.0)
    };

    let mesh = rect.mesh.as_ref().unwrap();
    ctx.draw(&mesh.vertex_buffer, &mesh.indices, ctx.color_shader, &uniforms, &Default::default())
        .unwrap();
}
