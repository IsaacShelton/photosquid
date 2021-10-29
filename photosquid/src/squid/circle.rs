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
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;
use std::time::{Duration, Instant};

pub struct Circle {
    data: Smooth<CircleData>,
    created: Instant,
    mesh: Option<MeshXyz>,

    // Tweaking parameters
    moving: bool,
    scale_rotating: bool,
    translation_accumulator: Accumulator<glm::Vec2>,
    rotation_accumulator: Accumulator<f32>,
}

#[derive(Copy, Clone)]
pub struct CircleData {
    x: f32,
    y: f32,
    radius: f32,
    color: Color,
    virtual_rotation: f32,
}

impl Lerpable for CircleData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            x: interpolation::Lerp::lerp(&self.x, &other.x, scalar),
            y: interpolation::Lerp::lerp(&self.y, &other.y, scalar),
            radius: interpolation::Lerp::lerp(&self.radius, &other.radius, scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
            virtual_rotation: interpolation::Lerp::lerp(&self.virtual_rotation, &other.virtual_rotation, scalar),
        }
    }
}

impl Circle {
    pub fn new(x: f32, y: f32, radius: f32, color: Color) -> Self {
        let data = CircleData {
            x,
            y,
            radius,
            color,
            virtual_rotation: 0.0,
        };
        Self::from_data(data)
    }

    pub fn from_data(data: CircleData) -> Self {
        Self {
            data: Smooth::new(data, Duration::from_millis(500)),
            created: Instant::now(),
            mesh: None,
            moving: false,
            scale_rotating: false,
            translation_accumulator: Accumulator::new(),
            rotation_accumulator: Accumulator::new(),
        }
    }

    pub fn get_rotate_handle_location(&self, camera: &glm::Vec2) -> glm::Vec2 {
        let CircleData {
            x,
            y,
            radius,
            virtual_rotation,
            ..
        } = self.data.get_animated();
        glm::vec2(x + camera.x + virtual_rotation.cos() * radius, y + camera.y - virtual_rotation.sin() * radius)
    }

    fn get_delta_rotation(&self, mouse_position: &glm::Vec2, camera: &glm::Vec2) -> f32 {
        let real = self.data.get_real();
        let screen_x = real.x + camera.x;
        let screen_y = real.y + camera.y;

        let old_rotation = real.virtual_rotation + self.rotation_accumulator.residue();
        let new_rotation = -1.0 * (mouse_position.y - screen_y).atan2(mouse_position.x - screen_x);
        return squid::angle_difference(old_rotation, new_rotation);
    }

    fn reposition_radius(&mut self, mouse: &glm::Vec2, camera: &glm::Vec2) {
        let real_in_world = self.data.get_real();
        let target_in_world = *mouse - camera;

        let mut new_data = *real_in_world;
        new_data.virtual_rotation += self.get_delta_rotation(mouse, camera);
        new_data.radius = glm::distance(&glm::vec2(real_in_world.x, real_in_world.y), &target_in_world);
        self.data.set(new_data);
    }
}

impl Squid for Circle {
    fn render(&mut self, ctx: &mut RenderCtx) {
        let CircleData { x, y, radius, color, .. } = self.data.get_animated();

        if self.mesh.is_none() {
            self.mesh = Some(MeshXyz::new_shape_circle(ctx.display));
        }

        let transformation = glm::translation(&glm::vec3(x, y, 0.0));
        let transformation = glm::scale(&transformation, &glm::vec3(radius, radius, 0.0));

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

    fn render_selected_indication(&self, ctx: &mut RenderCtx) {
        let camera = ctx.camera;
        let CircleData { x, y, .. } = self.data.get_animated();

        ctx.ring_mesh.render(
            ctx,
            x + camera.x,
            y + camera.y,
            squid::HANDLE_RADIUS,
            squid::HANDLE_RADIUS,
            &ctx.color_scheme.foreground,
        );

        let handle_position = self.get_rotate_handle_location(ctx.camera);

        ctx.ring_mesh.render(
            ctx,
            handle_position.x,
            handle_position.y,
            squid::HANDLE_RADIUS,
            squid::HANDLE_RADIUS,
            &ctx.color_scheme.foreground,
        );
    }

    fn interact(&mut self, interaction: &Interaction, camera: &glm::Vec2, _options: &InteractionOptions) -> Capture {
        match interaction {
            Interaction::PreClick => {
                self.moving = false;
                self.scale_rotating = false;
            }
            Interaction::Click {
                button: MouseButton::Left,
                position,
            } => {
                let rotate_handle_location = self.get_rotate_handle_location(camera);
                if glm::distance(position, &rotate_handle_location) <= squid::HANDLE_RADIUS * 3.0 {
                    self.scale_rotating = true;
                    return Capture::AllowDrag;
                }

                if self.is_point_over(&position, camera) {
                    self.moving = true;
                    return Capture::AllowDrag;
                }
            }
            Interaction::Drag { current, delta, .. } => {
                if self.scale_rotating {
                    // Since rotating and scaling at same time, it doesn't apply to others
                    self.reposition_radius(current, camera);
                } else if self.moving {
                    return Capture::MoveSelectedSquids { delta: *delta };
                }
            }
            Interaction::MouseRelease { button: MouseButton::Left, .. } => {
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
            new_data.virtual_rotation += delta_theta;
            self.data.set(new_data);
        }
    }

    fn is_point_over(&self, underneath: &glm::Vec2, camera: &glm::Vec2) -> bool {
        let real = self.data.get_real();
        let position = glm::vec2(real.x, real.y) + camera;
        glm::distance(&position, underneath) < real.radius
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

    fn duplicate(&self, offset: &glm::Vec2) -> Box<dyn Squid> {
        let mut real = *self.data.get_real();
        real.x += offset.x;
        real.y += offset.y;
        Box::new(Self::from_data(real))
    }

    fn get_creation_time(&self) -> Instant {
        self.created
    }

    fn initiate(&mut self, initiation: Initiation) {
        match initiation {
            Initiation::TRANSLATION => self.moving = true,
            Initiation::ROTATION => (),
        }
    }

    fn get_center(&self) -> glm::Vec2 {
        let CircleData { x, y, .. } = self.data.get_animated();
        glm::vec2(x, y)
    }
}
