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
    translation_accumulator: Accumulator<glm::Vec2>,
}

#[derive(Copy, Clone)]
struct CircleData {
    x: f32,
    y: f32,
    radius: f32,
    color: Color,
}

impl Lerpable for CircleData {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            x: interpolation::Lerp::lerp(&self.x, &other.x, scalar),
            y: interpolation::Lerp::lerp(&self.y, &other.y, scalar),
            radius: interpolation::Lerp::lerp(&self.radius, &other.radius, scalar),
            color: Lerpable::lerp(&self.color, &other.color, scalar),
        }
    }
}

impl Circle {
    pub fn new(x: f32, y: f32, radius: f32, color: Color) -> Self {
        let data = CircleData { x, y, radius, color };

        Self {
            data: Smooth::new(data, Duration::from_millis(500)),
            created: Instant::now(),
            mesh: None,
            moving: false,
            translation_accumulator: Accumulator::new(),
        }
    }
}

impl Squid for Circle {
    fn render(&mut self, ctx: &mut RenderCtx) {
        let CircleData { x, y, radius, color } = self.data.get_animated();

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
    }

    fn interact(&mut self, interaction: &Interaction, camera: &glm::Vec2, _options: &InteractionOptions) -> Capture {
        match interaction {
            Interaction::Click {
                button: MouseButton::Left,
                position,
            } => {
                self.moving = false;

                if self.is_point_over(&position, camera) {
                    self.moving = true;
                    return Capture::AllowDrag;
                }
            }
            Interaction::Drag { delta, .. } => {
                if self.moving {
                    return Capture::MoveSelectedSquids { delta: *delta };
                }
            }
            Interaction::MouseRelease { button: MouseButton::Left, .. } => {
                self.translation_accumulator.clear();
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

    fn rotate(&mut self, _delta_theta: f32, _options: &InteractionOptions) {}

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
        let real = self.data.get_real();
        Box::new(Self::new(real.x + offset.x, real.y + offset.y, real.radius, real.color))
    }

    fn get_creation_time(&self) -> Instant {
        self.created
    }

    fn initiate(&mut self, initiation: Initiation) {
        match initiation {
            Initiation::TRANSLATION => self.moving = true,
        }
    }
}
