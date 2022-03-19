use super::{
    behavior::{DilateBehavior, RevolveBehavior, SpreadBehavior, TranslateBehavior},
    PreviewParams, HANDLE_RADIUS,
};
use crate::{
    accumulator::Accumulator,
    camera::Camera,
    capture::Capture,
    color::Color,
    interaction::{ClickInteraction, DragInteraction, Interaction, MouseReleaseInteraction},
    math_helpers::angle_difference,
    matrix_helpers,
    mesh::MeshXyz,
    render_ctx::RenderCtx,
    smooth::{Lerpable, MultiLerp, NoLerp, Smooth},
};
use angular_units::{Angle, Rad};
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;

pub struct Circle {
    pub mesh: Option<MeshXyz>,
    pub data: Smooth<CircleData>,

    // Translate
    pub translate_behavior: TranslateBehavior,

    // Virtual Rotate
    pub rotation_accumulator: Accumulator<Rad<f32>>,

    // Scale
    pub prescale_size: f32,

    // Scale and Virtual Rotate
    pub scale_rotating: bool,

    // Spread
    pub spread_behavior: SpreadBehavior,

    // Revolve
    pub revolve_behavior: RevolveBehavior,

    // Dilate
    pub dilate_behavior: DilateBehavior,
}

#[derive(Copy, Clone)]
pub struct CircleData {
    pub position: MultiLerp<glm::Vec2>,
    pub radius: f32,
    pub color: NoLerp<Color>,
    pub virtual_rotation: Rad<f32>,
}

impl Lerpable for CircleData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            position: Lerpable::lerp(&self.position, &other.position, scalar),
            radius: interpolation::Lerp::lerp(&self.radius, &other.radius, scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
            virtual_rotation: angular_units::Interpolate::interpolate(&self.virtual_rotation, &other.virtual_rotation, *scalar),
        }
    }
}

pub fn render(circle: &mut Circle, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
    let CircleData { position, radius, color, .. } = circle.data.get_animated();

    if circle.mesh.is_none() {
        circle.mesh = Some(MeshXyz::new_shape_circle(ctx.display));
    }

    let (render_position, render_radius) = if let Some(preview) = &as_preview {
        (preview.position, preview.size * 0.5)
    } else {
        (position.reveal(), radius)
    };

    let mut transformation = glm::translation(&glm::vec2_to_vec3(&render_position));
    transformation = glm::scale(&transformation, &glm::vec3(render_radius, render_radius, 0.0));

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

    let mesh = circle.mesh.as_ref().unwrap();
    ctx.draw(&mesh.vertex_buffer, &mesh.indices, ctx.color_shader, &uniforms, &Default::default())
        .unwrap();
}

pub fn interact(circle: &mut Circle, interaction: &Interaction, camera: &Camera) -> Capture {
    match interaction {
        Interaction::PreClick => {
            circle.translate_behavior.moving = false;
            circle.scale_rotating = false;
        }
        Interaction::Click(ClickInteraction {
            button: MouseButton::Left,
            position,
        }) => {
            let rotate_handle_location = get_rotate_handle(circle, camera);
            if glm::distance(position, &rotate_handle_location) <= HANDLE_RADIUS * 2.0 {
                circle.scale_rotating = true;
                return Capture::AllowDrag;
            }

            if is_point_over(circle, *position, camera) {
                circle.translate_behavior.moving = true;
                return Capture::AllowDrag;
            }
        }
        Interaction::Drag(DragInteraction { current, delta, .. }) => {
            if circle.scale_rotating {
                // Since rotating and scaling at same time, it doesn't apply to others
                reposition_radius(circle, current, camera);
            } else if circle.translate_behavior.moving {
                return Capture::MoveSelectedSquids {
                    delta_in_world: camera.apply_reverse_to_vector(delta),
                };
            }
        }
        Interaction::MouseRelease(MouseReleaseInteraction { button: MouseButton::Left, .. }) => {
            circle.scale_rotating = false;
            circle.translate_behavior.accumulator.clear();
            circle.rotation_accumulator.clear();
        }
        _ => (),
    }

    Capture::Miss
}

pub fn get_rotate_handle(circle: &Circle, camera: &Camera) -> glm::Vec2 {
    let CircleData {
        position,
        radius,
        virtual_rotation,
        ..
    } = circle.data.get_animated();

    let position = position.reveal() + radius * glm::vec2(virtual_rotation.cos(), -virtual_rotation.sin());
    camera.apply(&position)
}

pub fn is_point_over(circle: &Circle, mouse_position: glm::Vec2, camera: &Camera) -> bool {
    let real = circle.data.get_real();
    let point = camera.apply_reverse(&mouse_position);
    glm::distance(&real.position.reveal(), &point) < real.radius
}

fn reposition_radius(circle: &mut Circle, mouse: &glm::Vec2, camera: &Camera) {
    let real_in_world = circle.data.get_real();
    let target_in_world = camera.apply_reverse(mouse);

    let mut new_data = *real_in_world;
    new_data.virtual_rotation += get_delta_rotation(circle, mouse, camera);
    new_data.radius = glm::distance(&real_in_world.position.reveal(), &target_in_world);
    circle.data.set(new_data);
}

fn get_delta_rotation(circle: &Circle, mouse_position: &glm::Vec2, camera: &Camera) -> Rad<f32> {
    let real = circle.data.get_real();
    let screen_position = camera.apply(&real.position.reveal());

    let old_rotation = real.virtual_rotation + *circle.rotation_accumulator.residue();
    let new_rotation = Rad(-1.0 * (mouse_position.y - screen_position.y).atan2(mouse_position.x - screen_position.x));

    angle_difference(old_rotation, new_rotation)
}
