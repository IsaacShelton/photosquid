use crate::{
    accumulator::Accumulator,
    algorithm::{get_distance_between_point_and_triangle, get_triangle_center, is_point_inside_triangle},
    camera::Camera,
    capture::Capture,
    data::TriData,
    interaction::{ClickInteraction, DragInteraction, Interaction, MouseReleaseInteraction},
    math::DivOrZero,
    matrix,
    mesh::MeshXyz,
    render_ctx::RenderCtx,
    smooth::{MultiLerp, Smooth},
};
use angular_units::{Angle, Rad};
use glium::{glutin::event::MouseButton, Display};
use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

use super::{
    behavior::{self, DilateBehavior, RevolveBehavior, SpreadBehavior, TranslateBehavior},
    PreviewParams, HANDLE_RADIUS,
};

#[derive(Serialize, Deserialize)]
pub struct Tri {
    #[serde(skip)]
    pub mesh: Option<MeshXyz>,

    pub data: Smooth<TriData>,

    // Keep track of which points the mesh is made of,
    // so that we know when we have to re-create it
    #[serde(skip)]
    pub mesh_p1: glm::Vec2,

    #[serde(skip)]
    pub mesh_p2: glm::Vec2,

    #[serde(skip)]
    pub mesh_p3: glm::Vec2,

    // Move point
    #[serde(skip)]
    pub moving_point: Option<usize>, // (zero indexed)

    // Translate
    #[serde(skip)]
    pub translate_behavior: TranslateBehavior,

    // Rotate
    #[serde(skip)]
    pub rotating: bool,

    #[serde(skip)]
    pub rotation_accumulator: Accumulator<Rad<f32>>,

    #[serde(skip)]
    pub virtual_rotation: Rad<f32>, // Rotation that only applies to the handle

    // Scale
    #[serde(skip)]
    pub prescale_size: [glm::Vec2; 3],

    // Spread
    #[serde(skip)]
    pub spread_behavior: SpreadBehavior,

    // Revolve
    #[serde(skip)]
    pub revolve_behavior: RevolveBehavior,

    // Dilate
    #[serde(skip)]
    pub dilate_behavior: DilateBehavior,
}

pub fn render(tri: &mut Tri, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
    let TriData {
        position,
        p1,
        p2,
        p3,
        rotation,
        color,
        ..
    } = tri.data.get_animated();

    let p1 = p1.reveal() + position.reveal();
    let p2 = p2.reveal() + position.reveal();
    let p3 = p3.reveal() + position.reveal();
    let position = tri.data.get_animated().position.reveal();

    refresh_mesh(tri, ctx.display);

    let (render_position, render_size) = if let Some(preview) = &as_preview {
        let max_distance = glm::distance(&p1, &position).max(glm::distance(&p2, &position).max(glm::distance(&p3, &position)));
        let factor = 1.0.div_or_zero(max_distance);

        (preview.position, factor * preview.size)
    } else {
        (position, 1.0)
    };

    let mut transformation = glm::translation(&glm::vec2_to_vec3(&render_position));
    transformation = glm::rotate(&transformation, rotation.scalar(), &glm::vec3(0.0, 0.0, -1.0));
    transformation = glm::scale(&transformation, &glm::vec3(render_size, render_size, 0.0));

    let raw_view = if as_preview.is_some() {
        matrix::reach_inside_mat4(&glm::identity::<f32, 4>())
    } else {
        matrix::reach_inside_mat4(ctx.view)
    };

    let uniforms = glium::uniform! {
        transformation: matrix::reach_inside_mat4(&transformation),
        view: raw_view,
        projection: matrix::reach_inside_mat4(ctx.projection),
        color: Into::<[f32; 4]>::into(color.0)
    };

    let mesh = tri.mesh.as_ref().unwrap();
    ctx.draw(&mesh.vertex_buffer, &mesh.indices, ctx.color_shader, &uniforms, &Default::default())
        .unwrap();
}

pub fn refresh_mesh(tri: &mut Tri, display: &Display) {
    let TriData { p1, p2, p3, .. } = tri.data.get_animated();

    let p1 = p1.reveal();
    let p2 = p2.reveal();
    let p3 = p3.reveal();

    if tri.mesh.is_none() || glm::distance2(&p1, &tri.mesh_p1) > 1.0 || glm::distance2(&p2, &tri.mesh_p2) > 1.0 || glm::distance2(&p3, &tri.mesh_p3) > 1.0 {
        // Data points are far enough from existing mesh that we will need
        // to re-create it
        tri.mesh = Some(MeshXyz::new_shape_triangle(display, p1, p2, p3));
    }
}

pub fn get_animated_screen_points(tri: &Tri, camera: &Camera) -> Vec<glm::Vec2> {
    let TriData {
        p1,
        p2,
        p3,
        position,
        rotation,
        ..
    } = tri.data.get_animated();

    let position = position.reveal();
    let rotation = rotation.scalar();

    [p1.reveal(), p2.reveal(), p3.reveal()]
        .iter()
        .map(|p| camera.apply(&(glm::rotate_vec2(p, -rotation) + position)))
        .collect()
}

pub fn get_rotate_handle(tri: &Tri, camera: &Camera) -> glm::Vec2 {
    let TriData {
        position,
        p1,
        p2,
        p3,
        rotation,
        ..
    } = tri.data.get_animated();

    let rotation = rotation + tri.virtual_rotation;
    let p1 = p1.reveal();
    let p2 = p2.reveal();
    let p3 = p3.reveal();
    let position = position.reveal();

    let max_distance = glm::magnitude(&p1).max(glm::magnitude(&p2)).max(glm::magnitude(&p3));
    let first_try = position + (max_distance + 24.0) * glm::vec2(rotation.cos(), -rotation.sin());

    let screen_points = get_animated_screen_points(tri, &Camera::identity(camera.window));
    assert_eq!(screen_points.len(), 3);

    let r_p1 = &screen_points[0];
    let r_p2 = &screen_points[1];
    let r_p3 = &screen_points[2];
    let true_distance = get_distance_between_point_and_triangle(&first_try, r_p1, r_p2, r_p3);
    let final_distance = (max_distance + 24.0 - true_distance) + 24.0;

    let rotate_handle_world_position = position + final_distance * glm::vec2(rotation.cos(), -rotation.sin());
    camera.apply(&rotate_handle_world_position)
}

pub fn interact(tri: &mut Tri, interaction: &Interaction, camera: &Camera) -> Capture {
    match interaction {
        Interaction::PreClick => {
            tri.translate_behavior.moving = false;
            tri.rotating = false;
            tri.moving_point = None;
        }
        Interaction::Click(ClickInteraction {
            button: MouseButton::Left,
            position,
        }) => {
            for (i, corner) in get_animated_screen_points(tri, camera).iter().enumerate() {
                if glm::distance(position, corner) <= HANDLE_RADIUS * 2.0 {
                    tri.moving_point = Some(i);
                    return Capture::AllowDrag;
                }
            }

            if glm::distance(position, &get_rotate_handle(tri, camera)) <= HANDLE_RADIUS * 2.0 {
                tri.rotating = true;
                return Capture::AllowDrag;
            }

            if is_point_over(tri, *position, camera) {
                tri.translate_behavior.moving = true;
                return Capture::AllowDrag;
            }
        }
        Interaction::Drag(DragInteraction {
            delta,
            current: mouse_position,
            ..
        }) => {
            if tri.moving_point.is_some() {
                reposition_point(tri, mouse_position, camera);
            } else if tri.rotating {
                return Capture::RotateSelectedSquids {
                    delta_theta: behavior::get_delta_rotation(
                        &tri.data.get_real().position.reveal(),
                        tri.data.get_real().rotation + tri.virtual_rotation,
                        mouse_position,
                        &tri.rotation_accumulator,
                        camera,
                    ),
                };
            } else if tri.translate_behavior.moving {
                return Capture::MoveSelectedSquids {
                    delta_in_world: camera.apply_reverse_to_vector(delta),
                };
            }
        }
        Interaction::MouseRelease(MouseReleaseInteraction { button: MouseButton::Left, .. }) => {
            tri.rotating = false;
            tri.moving_point = None;
            tri.translate_behavior.accumulator.clear();
            tri.rotation_accumulator.clear();
        }
        _ => (),
    }
    Capture::Miss
}

pub fn is_point_over(tri: &Tri, mouse_position: glm::Vec2, camera: &Camera) -> bool {
    let underneath = camera.apply_reverse(&mouse_position);

    let TriData {
        p1,
        p2,
        p3,
        rotation,
        position,
        ..
    } = tri.data.get_real();

    let p1 = p1.reveal();
    let p2 = p2.reveal();
    let p3 = p3.reveal();
    let position = position.reveal();
    let rotation = rotation.scalar();

    let p1 = glm::rotate_vec2(&p1, -rotation) + position;
    let p2 = glm::rotate_vec2(&p2, -rotation) + position;
    let p3 = glm::rotate_vec2(&p3, -rotation) + position;

    is_point_inside_triangle(underneath, p1, p2, p3)
}

fn reposition_point(tri: &mut Tri, mouse_position: &glm::Vec2, camera: &Camera) {
    let TriData {
        p1,
        p2,
        p3,
        position,
        rotation,
        ..
    } = tri.data.get_real();

    let p1 = p1.reveal();
    let p2 = p2.reveal();
    let p3 = p3.reveal();
    let position = position.reveal();

    let mut p1 = glm::rotate_vec2(&p1, -rotation.scalar());
    let mut p2 = glm::rotate_vec2(&p2, -rotation.scalar());
    let mut p3 = glm::rotate_vec2(&p3, -rotation.scalar());

    let mouse_world_position = camera.apply_reverse(mouse_position);
    let new_p = mouse_world_position - position;

    match tri.moving_point {
        Some(0) => p1 = new_p,
        Some(1) => p2 = new_p,
        Some(2) => p3 = new_p,
        _ => (),
    }

    let new_position = position + get_triangle_center(p1, p2, p3);
    let delta_center = new_position - position;
    p1 -= delta_center;
    p2 -= delta_center;
    p3 -= delta_center;

    // Set new data as the new target points, with zero rotation applied

    // HACK: Instantly snap rotation back to 0.0
    if rotation.scalar() != 0.0 {
        tri.virtual_rotation += *rotation;

        {
            let mut_real = tri.data.manual_get_real();
            mut_real.p1 = MultiLerp::Linear(p1);
            mut_real.p2 = MultiLerp::Linear(p2);
            mut_real.p3 = MultiLerp::Linear(p3);
            mut_real.position = MultiLerp::Linear(new_position);
            mut_real.rotation = Rad(0.0);
        }

        {
            let mut_previous = tri.data.manual_get_previous();
            mut_previous.p1 = MultiLerp::Linear(p1);
            mut_previous.p2 = MultiLerp::Linear(p2);
            mut_previous.p3 = MultiLerp::Linear(p3);
            mut_previous.position = MultiLerp::Linear(new_position);
            mut_previous.rotation = Rad(0.0);
        }
    } else {
        let mut new_real = *tri.data.get_real();
        new_real.p1 = MultiLerp::Linear(p1);
        new_real.p2 = MultiLerp::Linear(p2);
        new_real.p3 = MultiLerp::Linear(p3);
        new_real.position = MultiLerp::Linear(new_position);
        tri.data.set(new_real);
    }
}
