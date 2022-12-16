use std::convert::TryInto;

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
use itertools::Itertools;
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
    pub mesh_p: [glm::Vec2; 3],

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
            position, p, rotation, color, ..
        } = self.data.get_animated();

        let p = p.map(|point| point.reveal() + position.reveal());
        let position = self.data.get_animated().position.reveal();

        self.refresh_mesh(ctx.display);

        let (render_position, render_size) = if let Some(preview) = &as_preview {
            let max_distance = p.map(|point| glm::distance(&point, &position)).iter().fold(0.0f32, |a, &b| a.max(b));
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
        let TriData { p, .. } = self.data.get_animated();

        let p = p.map(|point| point.reveal());

        let model_point_mismatch = p.iter().zip(self.mesh_p).any(|(a, b)| glm::distance2(&a, &b) > 1.0);

        if self.mesh.is_none() || model_point_mismatch {
            // Data points are far enough from existing mesh that we will need
            // to re-create it
            self.mesh = Some(MeshXyz::new_shape_triangle(display, p));
        }
    }

    pub fn get_animated_screen_points(&self, camera: &Camera) -> [glm::Vec2; 3] {
        let TriData { p, position, rotation, .. } = self.data.get_animated();

        p.iter()
            .map(|point| camera.apply(&(glm::rotate_vec2(&point.reveal(), -rotation.scalar()) + position.reveal())))
            .collect_vec()
            .try_into()
            .unwrap()
    }

    pub fn get_rotate_handle(&self, camera: &Camera) -> glm::Vec2 {
        let tri_data = self.data.get_animated();

        let rotation = tri_data.rotation + self.virtual_rotation;
        let p = tri_data.p.map(|point| point.reveal());
        let position = tri_data.position.reveal();

        let max_distance = p.iter().map(|point| glm::magnitude(&point)).fold(0.0f32, |a, b| a.max(b));
        let first_try = position + (max_distance + 24.0) * glm::vec2(rotation.cos(), -rotation.sin());

        let screen_points = self.get_animated_screen_points(&Camera::identity(camera.window));
        let true_distance = get_distance_between_point_and_triangle(&first_try, &screen_points);
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
                self.prescale_size = real.p.map(|point| point.reveal());
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
                self.prescale_size = real.p.map(|point| point.reveal());
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

        let tri_data = self.data.get_real();

        let p = tri_data.p.map(|point| point.reveal());
        let position = tri_data.position.reveal();
        let rotation = tri_data.rotation.scalar();

        let world_p = p.map(|point| glm::rotate_vec2(&point, -rotation) + position);
        is_point_inside_triangle(underneath, world_p)
    }

    pub fn build(&self, _document: &svg::Document) {}

    fn reposition_point(&mut self, mouse_position: &glm::Vec2, camera: &Camera) {
        let TriData { p, position, rotation, .. } = self.data.get_real();

        let position = position.reveal();

        let mut p = p.map(|point| glm::rotate_vec2(&point.reveal(), -rotation.scalar()));
        let mouse_world_position = camera.apply_reverse(mouse_position);
        let new_single_p = mouse_world_position - position;

        if let Some(index) = self.moving_point {
            p[index] = new_single_p;
        }

        let delta_center = get_triangle_center(p);
        let new_position = position + delta_center;

        let p = p.map(|point| point - delta_center);

        // Set new data as the new target points, with zero rotation applied

        // Reset underlying rotation
        // The virtual rotation will be adjusted to compensate
        // HACK: Instantly snap rotation back to 0.0
        {
            self.virtual_rotation += *rotation;

            let mut_real = self.data.manual_get_real();
            mut_real.p = p.map(|point| MultiLerp::Linear(point));
            mut_real.position = MultiLerp::Linear(new_position);
            mut_real.rotation = Rad(0.0);

            let mut_previous = self.data.manual_get_previous();
            mut_previous.p = p.map(|point| MultiLerp::Linear(point));
            mut_previous.position = MultiLerp::Linear(new_position);
            mut_previous.rotation = Rad(0.0);
        }
    }
}
