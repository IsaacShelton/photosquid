use super::{
    behavior::{DilateBehavior, RevolveBehavior, SpreadBehavior, TranslateBehavior},
    Initiation, PreviewParams, HANDLE_RADIUS,
};
use crate::{
    accumulator::Accumulator,
    as_values::AsValues,
    camera::Camera,
    capture::Capture,
    components,
    data::CircleData,
    interaction::{ClickInteraction, DragInteraction, Interaction, MouseReleaseInteraction},
    math::angle_difference,
    mesh::MeshXyz,
    render_ctx::RenderCtx,
    smooth::Smooth,
};
use angular_units::Rad;
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Circle {
    #[serde(skip)]
    pub mesh: Option<MeshXyz>,

    pub data: Smooth<CircleData>,

    // Translate
    #[serde(skip)]
    pub translate_behavior: TranslateBehavior,

    // Virtual Rotate
    #[serde(skip)]
    pub rotation_accumulator: Accumulator<Rad<f32>>,

    // Scale
    #[serde(skip)]
    pub prescale_size: f32,

    // Scale and Virtual Rotate
    #[serde(skip)]
    pub scale_rotating: bool,

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

impl Circle {
    pub fn render(&mut self, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
        let CircleData { position, radius, color, .. } = self.data.get_animated();

        if self.mesh.is_none() {
            self.mesh = Some(MeshXyz::new_shape_circle(ctx.display));
        }

        let (render_position, render_radius) = if let Some(preview) = &as_preview {
            (preview.position, preview.radius * 0.5)
        } else {
            (position.reveal(), radius)
        };

        let mut transformation = glm::translation(&glm::vec2_to_vec3(&render_position));
        transformation = glm::scale(&transformation, &glm::vec3(render_radius, render_radius, 0.0));

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

    pub fn interact(&mut self, interaction: &Interaction, camera: &Camera) -> Capture {
        match interaction {
            Interaction::PreClick => {
                self.translate_behavior.moving = false;
                self.scale_rotating = false;
            }
            Interaction::Click(ClickInteraction {
                button: MouseButton::Left,
                position,
                ..
            }) => {
                let rotate_handle_location = self.get_rotate_handle(camera);

                if glm::distance(position, &rotate_handle_location) <= HANDLE_RADIUS * 2.0 {
                    self.scale_rotating = true;
                    return Capture::AllowDrag;
                }

                if self.is_point_over(*position, camera) {
                    self.translate_behavior.moving = true;
                    return Capture::AllowDrag;
                }
            }
            Interaction::Drag(DragInteraction { current, delta, .. }) => {
                if self.scale_rotating {
                    // Since rotating and scaling at same time, it doesn't apply to others
                    self.reposition_radius(current, camera);
                } else if self.translate_behavior.moving {
                    return Capture::MoveSelectedSquids {
                        delta_in_world: camera.apply_reverse_to_vector(delta),
                    };
                }
            }
            Interaction::MouseRelease(MouseReleaseInteraction { button: MouseButton::Left, .. }) => {
                self.scale_rotating = false;
                self.translate_behavior.accumulator.clear();
                self.rotation_accumulator.clear();
            }
            _ => (),
        }

        Capture::Miss
    }

    pub fn initiate(&mut self, initiation: Initiation) {
        match initiation {
            Initiation::Translate => self.translate_behavior.moving = true,
            Initiation::Rotate => (),
            Initiation::Scale => self.prescale_size = self.data.get_real().radius,
            Initiation::Spread { point, center } => {
                self.spread_behavior = SpreadBehavior {
                    point,
                    origin: center,
                    start: self.data.get_real().position.reveal(),
                };
            }
            Initiation::Revolve { point, center } => self.revolve_behavior.set(&center, &self.data.get_real().position.reveal(), &point),
            Initiation::Dilate { point, center } => {
                self.prescale_size = self.data.get_real().radius;
                self.dilate_behavior = DilateBehavior {
                    point,
                    origin: center,
                    start: self.data.get_real().position.reveal(),
                };
            }
        }
    }

    pub fn get_rotate_handle(&self, camera: &Camera) -> glm::Vec2 {
        let CircleData {
            position,
            radius,
            virtual_rotation,
            ..
        } = self.data.get_animated();

        components::get_rotate_handle(position.reveal(), virtual_rotation, radius, camera)
    }

    pub fn is_point_over(&self, mouse_position: glm::Vec2, camera: &Camera) -> bool {
        let real = self.data.get_real();
        let point = camera.apply_reverse(&mouse_position);
        glm::distance(&real.position.reveal(), &point) < real.radius
    }

    pub fn build(&self, document: &mut svg::Document) {
        use svg::Node;

        let CircleData { position, radius, .. } = self.data.get_real();
        let position = position.reveal();

        let circle = svg::node::element::Circle::new()
            .set("r", *radius)
            .set("cx", position.x)
            .set("cy", position.y)
            .set("color", "rgba(255, 0, 0, 0)");
        document.append(circle);
    }

    fn reposition_radius(&mut self, mouse: &glm::Vec2, camera: &Camera) {
        let real_in_world = self.data.get_real();
        let target_in_world = camera.apply_reverse(mouse);

        let mut new_data = *real_in_world;
        new_data.virtual_rotation += self.get_delta_rotation(mouse, camera);
        new_data.radius = glm::distance(&real_in_world.position.reveal(), &target_in_world);
        self.data.set(new_data);
    }

    fn get_delta_rotation(&self, mouse_position: &glm::Vec2, camera: &Camera) -> Rad<f32> {
        let real = self.data.get_real();
        let screen_position = camera.apply(&real.position.reveal());

        let old_rotation = real.virtual_rotation + *self.rotation_accumulator.residue();
        let new_rotation = Rad(-1.0 * (mouse_position.y - screen_position.y).atan2(mouse_position.x - screen_position.x));

        angle_difference(old_rotation, new_rotation)
    }
}
