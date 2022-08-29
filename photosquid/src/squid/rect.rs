use super::{
    behavior::{self, DilateBehavior, RevolveBehavior, SpreadBehavior, TranslateBehavior},
    PreviewParams, HANDLE_RADIUS,
};
use crate::{
    accumulator::Accumulator,
    algorithm,
    as_values::AsValues,
    camera::Camera,
    capture::Capture,
    data::RectData,
    interaction::{ClickInteraction, DragInteraction, Interaction, MouseReleaseInteraction},
    math::DivOrZero,
    mesh::MeshXyz,
    render_ctx::RenderCtx,
    smooth::{MultiLerp, Smooth},
};
use angular_units::{Angle, Rad};
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Rect {
    #[serde(skip)]
    pub mesh: Option<MeshXyz>,

    pub data: Smooth<RectData>,

    // Move point
    #[serde(skip)]
    pub moving_corner: Option<Corner>,

    // Moving point (single corner)
    #[serde(skip)]
    pub opposite_corner_position: Option<glm::Vec2>,

    // Translate
    #[serde(skip)]
    pub translate_behavior: TranslateBehavior,

    // Rotate
    #[serde(skip)]
    pub rotating: bool,

    #[serde(skip)]
    pub rotation_accumulator: Accumulator<Rad<f32>>,

    // Scale
    #[serde(skip)]
    pub prescale_size: glm::Vec2,

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

#[derive(Copy, Clone)]
pub enum Corner {
    ZeroZero = 3,
    ZeroY = 1,
    XZero = 2,
    XY = 0,
}

impl Corner {
    fn opposite(self) -> Corner {
        use Corner::*;

        match self {
            XY => ZeroZero,
            ZeroY => XZero,
            XZero => ZeroY,
            ZeroZero => XY,
        }
    }
}

impl From<usize> for Corner {
    fn from(corner_index: usize) -> Self {
        use Corner::*;

        [XY, ZeroY, XZero, ZeroZero][corner_index]
    }
}

impl From<Corner> for usize {
    fn from(corner: Corner) -> Self {
        use Corner::*;

        match corner {
            XY => 0,
            ZeroY => 1,
            XZero => 2,
            ZeroZero => 3,
        }
    }
}

impl Rect {
    pub fn get_rotate_handle(&self, camera: &Camera) -> glm::Vec2 {
        let RectData { position, size, rotation, .. } = self.data.get_animated();
        let w = size.x;

        let world_position = glm::vec2(
            position.reveal().x + rotation.cos() * (w * 0.5 + 24.0 * w.signum()),
            position.reveal().y - rotation.sin() * (w * 0.5 + 24.0 * w.signum()),
        );

        camera.apply(&world_position)
    }

    pub fn get_relative_corners(&self) -> Vec<glm::Vec2> {
        // Use non-animated version always for now?
        // It seems to look better this way
        let RectData { size, rotation, .. } = self.data.get_animated();

        [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0f32)]
            .iter()
            .map(|&p| glm::vec2(p.0 * size.x / 2.0, p.1 * size.y / 2.0))
            .map(|p| glm::rotate_vec2(&p, -1.0 * rotation.scalar()))
            .collect()
    }

    pub fn get_world_corners(&self) -> Vec<glm::Vec2> {
        let RectData { position, .. } = self.data.get_animated();

        self.get_relative_corners().iter().map(|p| p + position.reveal()).collect()
    }

    fn get_screen_corners(&self, camera: &Camera) -> Vec<glm::Vec2> {
        let RectData { position, .. } = self.data.get_animated();

        self.get_relative_corners().iter().map(|p| camera.apply(&(p + position.reveal()))).collect()
    }

    fn reposition_corner(&mut self, from: RectScaleFrom, mouse: &glm::Vec2, camera: &Camera) {
        let real = self.data.get_real();
        let rotation = real.rotation.scalar();
        let mouse_in_world = camera.apply_reverse(mouse);

        match from {
            RectScaleFrom::Corner => {
                let pivot = self.opposite_corner_position.unwrap();
                let frame_vector = glm::rotate_vec2(&(pivot - mouse_in_world), rotation);

                let size = frame_vector.component_mul(&match self.moving_corner.unwrap() {
                    Corner::ZeroZero => glm::vec2(1.0, 1.0),
                    Corner::XZero => glm::vec2(-1.0, 1.0),
                    Corner::ZeroY => glm::vec2(1.0, -1.0),
                    Corner::XY => glm::vec2(-1.0, -1.0),
                });

                let mut new_data = *real;
                new_data.position = MultiLerp::Linear(0.5 * (mouse_in_world + pivot));
                new_data.size = size;
                self.data.set(new_data);
                self.mesh = None;
            }
            RectScaleFrom::Center => {
                let abs_size = 2.0 * glm::rotate_vec2(&(real.position.reveal() - mouse_in_world), rotation);

                let new_size = abs_size.component_mul(&match self.moving_corner.unwrap() {
                    Corner::ZeroZero => glm::vec2(1.0, 1.0),
                    Corner::XZero => glm::vec2(-1.0, 1.0),
                    Corner::ZeroY => glm::vec2(1.0, -1.0),
                    Corner::XY => glm::vec2(-1.0, -1.0),
                });

                let mut new_data = *real;
                new_data.size = new_size;
                self.data.set(new_data);
                self.mesh = None;
            }
        }
    }

    pub fn is_point_over(&self, mouse_position: glm::Vec2, camera: &Camera) -> bool {
        let underneath = camera.apply_reverse(&mouse_position);

        let corners: Vec<glm::Vec2> = self.get_world_corners();
        assert_eq!(corners.len(), 4);
        algorithm::is_point_inside_rectangle(corners[0], corners[1], corners[2], corners[3], underneath)
    }

    pub fn interact(&mut self, interaction: &Interaction, camera: &Camera) -> Capture {
        match interaction {
            Interaction::PreClick => {
                self.translate_behavior.moving = false;
                self.rotating = false;
                self.moving_corner = None;
                self.opposite_corner_position = None;
            }
            Interaction::Click(ClickInteraction {
                button: MouseButton::Left,
                position,
                ..
            }) => {
                for (i, corner) in self.get_screen_corners(camera).iter().enumerate() {
                    if glm::distance(position, corner) <= HANDLE_RADIUS * 2.0 {
                        let world_corners = self.get_world_corners();

                        self.moving_corner = Some(i.into());
                        self.opposite_corner_position = Some(world_corners[usize::from(Corner::from(i).opposite())]);
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
                modifiers,
                ..
            }) => {
                if self.moving_corner.is_some() {
                    let from = if modifiers.alt() { RectScaleFrom::Center } else { RectScaleFrom::Corner };
                    self.reposition_corner(from, mouse_position, camera);
                } else if self.rotating {
                    // When the rectangle's width is negative, the rotation handle is PI radians ahead of it's angle
                    // compared to the actual rotation of the shape,
                    // So we have to compensate for the difference when that's the case.
                    // It might be better to integrate this difference into the rotation and not allow negative
                    // widths/heights for rectangles, but this is an easier way to do it
                    let compensation = Rad(if self.data.get_animated().size.x < 0.0 { std::f32::consts::PI } else { 0.0 });

                    let real = self.data.get_real();

                    return Capture::RotateSelectedSquids {
                        delta_theta: compensation
                            + behavior::get_delta_rotation(&real.position.reveal(), real.rotation, mouse_position, &self.rotation_accumulator, camera),
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

    pub fn render(&mut self, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
        let RectData {
            position,
            size,
            rotation,
            color,
            ..
        } = self.data.get_animated();

        // Refresh mesh
        {
            let real = self.data.get_real();
            let animated = self.data.get_animated();

            // Don't use margin of error
            if self.mesh.is_none() || real.radii != animated.radii || real.size != animated.size {
                self.mesh = Some(MeshXyz::new_rect(ctx.display, animated.size, animated.radii));
            }
        }

        // Translate
        let mut transformation = glm::translation(&glm::vec2_to_vec3(&if let Some(preview) = &as_preview {
            preview.position
        } else {
            position.reveal()
        }));

        // Rotate
        transformation = glm::rotate(&transformation, rotation.scalar(), &glm::vec3(0.0, 0.0, -1.0));

        // Scale
        if let Some(preview) = &as_preview {
            let max_size = glm::comp_max(&size.abs());
            let preview_scale = preview.radius.div_or_zero(max_size);
            transformation = glm::scale(&transformation, &glm::vec3(preview_scale, preview_scale, 0.0));
        }

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
}

#[derive(Copy, Clone, PartialEq)]
enum RectScaleFrom {
    Corner,
    Center,
}
