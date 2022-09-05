use crate::{
    accumulator::Accumulator,
    algorithm::{get_distance_between_point_and_triangle, get_triangle_center, is_point_inside_triangle},
    as_values::AsValues,
    camera::Camera,
    capture::Capture,
    components,
    data::TriData,
    interaction::{ClickInteraction, DragInteraction, Interaction, MouseReleaseInteraction},
    math::DivOrZero,
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
    Initiation, PreviewParams, HANDLE_RADIUS,
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

impl Tri {
    pub fn render(&mut self, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
        let TriData {
            position,
            p1,
            p2,
            p3,
            rotation,
            color,
            ..
        } = self.data.get_animated();

        let p1 = p1.reveal() + position.reveal();
        let p2 = p2.reveal() + position.reveal();
        let p3 = p3.reveal() + position.reveal();
        let position = self.data.get_animated().position.reveal();

        self.refresh_mesh(ctx.display);

        let (render_position, render_size) = if let Some(preview) = &as_preview {
            let max_distance = glm::distance(&p1, &position).max(glm::distance(&p2, &position).max(glm::distance(&p3, &position)));
            let factor = 1.0.div_or_zero(max_distance);

            (preview.position, factor * preview.radius)
        } else {
            (position, 1.0)
        };

        let transformation = {
            let mut matrix;
            matrix = glm::translation(&glm::vec2_to_vec3(&render_position));
            matrix = glm::rotate(&matrix, rotation.scalar(), &glm::vec3(0.0, 0.0, -1.0));
            matrix = glm::scale(&matrix, &glm::vec3(render_size, render_size, 0.0));
            matrix
        };

        let uniforms = glium::uniform! {
            transformation: transformation.as_values(),
            view: if as_preview.is_some() {
                glm::identity::<f32, 4>().as_values()
            } else {
                ctx.view.as_values()
            },
            projection: ctx.projection.as_values(),
            color: color.as_values()
        };

        let mesh = self.mesh.as_ref().unwrap();
        ctx.draw(&mesh.vertex_buffer, &mesh.indices, ctx.color_shader, &uniforms, &Default::default())
            .unwrap();
    }

    pub fn refresh_mesh(&mut self, display: &Display) {
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

    pub fn get_animated_screen_points(&self, camera: &Camera) -> Vec<glm::Vec2> {
        let TriData {
            p1,
            p2,
            p3,
            position,
            rotation,
            ..
        } = self.data.get_animated();

        [p1.reveal(), p2.reveal(), p3.reveal()]
            .iter()
            .map(|p| camera.apply(&(glm::rotate_vec2(p, -rotation.scalar()) + position.reveal())))
            .collect()
    }

    pub fn get_rotate_handle(&self, camera: &Camera) -> glm::Vec2 {
        let TriData {
            position,
            p1,
            p2,
            p3,
            rotation,
            ..
        } = self.data.get_animated();

        let rotation = rotation + self.virtual_rotation;
        let p1 = p1.reveal();
        let p2 = p2.reveal();
        let p3 = p3.reveal();
        let position = position.reveal();

        let max_distance = glm::magnitude(&p1).max(glm::magnitude(&p2)).max(glm::magnitude(&p3));
        let first_try = position + (max_distance + 24.0) * glm::vec2(rotation.cos(), -rotation.sin());

        let screen_points = self.get_animated_screen_points(&Camera::identity(camera.window));
        assert_eq!(screen_points.len(), 3);

        let r_p1 = &screen_points[0];
        let r_p2 = &screen_points[1];
        let r_p3 = &screen_points[2];
        let true_distance = get_distance_between_point_and_triangle(&first_try, r_p1, r_p2, r_p3);
        let final_distance = (max_distance - true_distance) + 48.0;

        components::get_rotate_handle(position, rotation, final_distance, camera)
    }

    pub fn interact(&mut self, interaction: &Interaction, camera: &Camera) -> Capture {
        match interaction {
            Interaction::PreClick => {
                self.translate_behavior.moving = false;
                self.rotating = false;
                self.moving_point = None;
            }
            Interaction::Click(ClickInteraction {
                button: MouseButton::Left,
                position,
                ..
            }) => {
                for (i, corner) in self.get_animated_screen_points(camera).iter().enumerate() {
                    if glm::distance(position, corner) <= HANDLE_RADIUS * 2.0 {
                        self.moving_point = Some(i);
                        return Capture::AllowDrag;
                    }
                }

                if glm::distance(position, &self.get_rotate_handle(camera)) <= HANDLE_RADIUS * 2.0 {
                    self.rotating = true;
                    return Capture::AllowDrag;
                }

                if self.is_point_over(*position, camera) {
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
                        delta_theta: behavior::get_delta_rotation(
                            &self.data.get_real().position.reveal(),
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

    pub fn initiate(&mut self, initiation: Initiation) {
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
                    start: self.data.get_real().position.reveal(),
                };
            }
            Initiation::Revolve { point, center } => self.revolve_behavior.set(&center, &self.data.get_real().position.reveal(), &point),
            Initiation::Dilate { point, center } => {
                let real = self.data.get_real();
                self.prescale_size = [real.p1.reveal(), real.p2.reveal(), real.p3.reveal()];
                self.dilate_behavior = DilateBehavior {
                    point,
                    origin: center,
                    start: self.data.get_real().position.reveal(),
                };
            }
        }
    }

    pub fn is_point_over(&self, mouse_position: glm::Vec2, camera: &Camera) -> bool {
        let underneath = camera.apply_reverse(&mouse_position);

        let TriData {
            p1,
            p2,
            p3,
            rotation,
            position,
            ..
        } = self.data.get_real();

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

    pub fn build(&self, _builder: &impl lyon::path::builder::PathBuilder) {}

    fn reposition_point(&mut self, mouse_position: &glm::Vec2, camera: &Camera) {
        let TriData {
            p1,
            p2,
            p3,
            position,
            rotation,
            ..
        } = self.data.get_real();

        let position = position.reveal();
        let mut p1 = glm::rotate_vec2(&p1.reveal(), -rotation.scalar());
        let mut p2 = glm::rotate_vec2(&p2.reveal(), -rotation.scalar());
        let mut p3 = glm::rotate_vec2(&p3.reveal(), -rotation.scalar());

        let mouse_world_position = camera.apply_reverse(mouse_position);
        let new_p = mouse_world_position - position;

        match self.moving_point {
            Some(0) => p1 = new_p,
            Some(1) => p2 = new_p,
            Some(2) => p3 = new_p,
            _ => (),
        }

        let delta_center = get_triangle_center(p1, p2, p3);
        let new_position = position + delta_center;

        p1 -= delta_center;
        p2 -= delta_center;
        p3 -= delta_center;

        // Set new data as the new target points, with zero rotation applied

        // Reset underlying rotation
        // The virtual rotation will be adjusted to compensate
        // HACK: Instantly snap rotation back to 0.0
        {
            self.virtual_rotation += *rotation;

            let mut_real = self.data.manual_get_real();
            mut_real.p1 = MultiLerp::Linear(p1);
            mut_real.p2 = MultiLerp::Linear(p2);
            mut_real.p3 = MultiLerp::Linear(p3);
            mut_real.position = MultiLerp::Linear(new_position);
            mut_real.rotation = Rad(0.0);

            let mut_previous = self.data.manual_get_previous();
            mut_previous.p1 = MultiLerp::Linear(p1);
            mut_previous.p2 = MultiLerp::Linear(p2);
            mut_previous.p3 = MultiLerp::Linear(p3);
            mut_previous.position = MultiLerp::Linear(new_position);
            mut_previous.rotation = Rad(0.0);
        }
    }
}
