use super::{Initiation, Squid, SquidRef};
use crate::{
    accumulator::Accumulator,
    capture::Capture,
    color::Color,
    color_scheme::ColorScheme,
    context_menu::ContextMenu,
    interaction::Interaction,
    interaction_options::InteractionOptions,
    math_helpers::angle_difference,
    matrix_helpers::reach_inside_mat4,
    mesh::MeshXyz,
    render_ctx::RenderCtx,
    selection::{NewSelection, NewSelectionInfo, Selection},
    smooth::{Lerpable, MultiLerp, NoLerp, Smooth},
    squid::{
        self,
        behavior::{DilateBehavior, RevolveBehavior, SpreadBehavior, TranslateBehavior},
        PreviewParams,
    },
};
use angular_units::{self, Angle, Rad};
use glium::glutin::event::MouseButton;
use nalgebra_glm as glm;
use std::time::{Duration, Instant};

pub struct Circle {
    name: Option<String>,
    data: Smooth<CircleData>,
    created: Instant,
    mesh: Option<MeshXyz>,

    // --------- Tweaking parameters ---------

    // Translate
    translate_behavior: TranslateBehavior,

    // Virtual Rotate
    rotation_accumulator: Accumulator<Rad<f32>>,

    // Scale
    prescale_size: f32,

    // Scale and Virtual Rotate
    scale_rotating: bool,

    // Spread
    spread_behavior: SpreadBehavior,

    // Revolve
    revolve_behavior: RevolveBehavior,

    // Dilate
    dilate_behavior: DilateBehavior,
}

#[derive(Copy, Clone)]
pub struct CircleData {
    position: MultiLerp<glm::Vec2>,
    radius: f32,
    color: NoLerp<Color>,
    virtual_rotation: Rad<f32>,
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

impl Circle {
    pub fn new(x: f32, y: f32, radius: f32, color: Color) -> Self {
        let data = CircleData {
            position: MultiLerp::From(glm::vec2(x, y)),
            radius,
            color: NoLerp(color),
            virtual_rotation: Rad(0.0),
        };
        Self::from_data(data)
    }

    pub fn from_data(data: CircleData) -> Self {
        Self {
            name: None,
            data: Smooth::new(data, Duration::from_millis(500)),
            created: Instant::now(),
            mesh: None,
            translate_behavior: Default::default(),
            scale_rotating: false,
            rotation_accumulator: Accumulator::new(),
            prescale_size: data.radius,
            spread_behavior: Default::default(),
            revolve_behavior: Default::default(),
            dilate_behavior: Default::default(),
        }
    }

    pub fn get_rotate_handle_location(&self, camera: &glm::Vec2) -> glm::Vec2 {
        let CircleData {
            position,
            radius,
            virtual_rotation,
            ..
        } = self.data.get_animated();

        position.reveal() + camera + radius * glm::vec2(virtual_rotation.cos(), -virtual_rotation.sin())
    }

    fn get_delta_rotation(&self, mouse_position: &glm::Vec2, camera: &glm::Vec2) -> Rad<f32> {
        let real = self.data.get_real();
        let screen_position = real.position.reveal() + camera;

        let old_rotation = real.virtual_rotation + *self.rotation_accumulator.residue();
        let new_rotation = Rad(-1.0 * (mouse_position.y - screen_position.y).atan2(mouse_position.x - screen_position.x));

        angle_difference(old_rotation, new_rotation)
    }

    fn reposition_radius(&mut self, mouse: &glm::Vec2, camera: &glm::Vec2) {
        let real_in_world = self.data.get_real();
        let target_in_world = *mouse - camera;

        let mut new_data = *real_in_world;
        new_data.virtual_rotation += self.get_delta_rotation(mouse, camera);
        new_data.radius = glm::distance(&real_in_world.position.reveal(), &target_in_world);
        self.data.set(new_data);
    }
}

impl Squid for Circle {
    fn render(&mut self, ctx: &mut RenderCtx, as_preview: Option<PreviewParams>) {
        let CircleData { position, radius, color, .. } = self.data.get_animated();

        if self.mesh.is_none() {
            self.mesh = Some(MeshXyz::new_shape_circle(ctx.display));
        }

        let transformation = if let Some(preview) = &as_preview {
            let matrix = glm::translation(&glm::vec2_to_vec3(&preview.position));
            glm::scale(&matrix, &glm::vec3(preview.size * 0.5, preview.size * 0.5, 0.0))
        } else {
            let matrix = glm::translation(&glm::vec2_to_vec3(&position.reveal()));
            glm::scale(&matrix, &glm::vec3(radius, radius, 0.0))
        };

        let view = if as_preview.is_some() {
            reach_inside_mat4(&glm::identity::<f32, 4>())
        } else {
            reach_inside_mat4(ctx.view)
        };

        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: view,
            projection: reach_inside_mat4(ctx.projection),
            color: Into::<[f32; 4]>::into(color.0)
        };

        let mesh = self.mesh.as_ref().unwrap();
        ctx.draw(&mesh.vertex_buffer, &mesh.indices, ctx.color_shader, &uniforms, &Default::default())
            .unwrap();
    }

    fn render_selected_indication(&self, ctx: &mut RenderCtx) {
        let camera = ctx.camera;
        let CircleData { position, .. } = self.data.get_animated();

        ctx.ring_mesh.render(
            ctx,
            position.reveal().x + camera.x,
            position.reveal().y + camera.y,
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
                self.translate_behavior.moving = false;
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

                if self.is_point_over(position, camera) {
                    self.translate_behavior.moving = true;
                    return Capture::AllowDrag;
                }
            }
            Interaction::Drag { current, delta, .. } => {
                if self.scale_rotating {
                    // Since rotating and scaling at same time, it doesn't apply to others
                    self.reposition_radius(current, camera);
                } else if self.translate_behavior.moving {
                    return Capture::MoveSelectedSquids { delta: *delta };
                }
            }
            Interaction::MouseRelease { button: MouseButton::Left, .. } => {
                self.scale_rotating = false;
                self.translate_behavior.accumulator.clear();
                self.rotation_accumulator.clear();
            }
            _ => (),
        }

        Capture::Miss
    }

    fn translate(&mut self, raw_delta: &glm::Vec2, options: &InteractionOptions) {
        let delta = self.translate_behavior.express(raw_delta, options);

        if delta != glm::zero::<glm::Vec2>() {
            let mut new_data = *self.data.get_real();
            new_data.position = MultiLerp::Linear(new_data.position.reveal() + delta);
            self.data.set(new_data);
        }
    }

    fn rotate(&mut self, raw_delta_theta: Rad<f32>, options: &InteractionOptions) {
        if let Some(delta_theta) = self.rotation_accumulator.accumulate(&raw_delta_theta, options.rotation_snapping) {
            let mut new_data = *self.data.get_real();
            new_data.virtual_rotation += delta_theta;
            self.data.set(new_data);
        }
    }

    fn scale(&mut self, total_scale_factor: f32, _options: &InteractionOptions) {
        let mut new_data = *self.data.get_real();
        new_data.radius = self.prescale_size * total_scale_factor;
        self.data.set(new_data);
    }

    fn spread(&mut self, current: &glm::Vec2, _options: &InteractionOptions) {
        let mut new_data = *self.data.get_real();
        new_data.position = MultiLerp::Linear(self.spread_behavior.express(current));
        self.data.set(new_data);
    }

    fn dilate(&mut self, current: &glm::Vec2, _options: &InteractionOptions) {
        let mut new_data = *self.data.get_real();
        let expression = self.dilate_behavior.express(current);
        new_data.position = MultiLerp::Linear(expression.position);
        new_data.radius = self.prescale_size * expression.total_scale_factor;
        self.data.set(new_data);
    }

    fn revolve(&mut self, current: &glm::Vec2, options: &InteractionOptions) {
        if let Some(expression) = self.revolve_behavior.express(current, options) {
            let mut new_data = *self.data.get_real();

            let new_center = expression.apply_origin_rotation_to_center();
            new_data.position = MultiLerp::Circle(new_center, expression.origin);
            new_data.virtual_rotation += expression.delta_object_rotation;
            self.data.set(new_data);
        }
    }

    fn is_point_over(&self, underneath: &glm::Vec2, camera: &glm::Vec2) -> bool {
        let real = self.data.get_real();
        let position = real.position.reveal() + camera;
        glm::distance(&position, underneath) < real.radius
    }

    fn try_select(&self, underneath: &glm::Vec2, camera: &glm::Vec2, self_reference: SquidRef) -> Option<NewSelection> {
        if self.is_point_over(underneath, camera) {
            Some(NewSelection {
                selection: Selection::new(self_reference, None),
                info: NewSelectionInfo {
                    color: Some(self.data.get_real().color.0),
                },
            })
        } else {
            None
        }
    }

    fn select(&mut self) {
        self.translate_behavior.moving = true;
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
        new_data.color = NoLerp(color);
        self.data.set(new_data);
    }

    fn duplicate(&self, offset: &glm::Vec2) -> Box<dyn Squid> {
        let mut real = *self.data.get_real();
        real.position = MultiLerp::From(real.position.reveal() + offset);
        Box::new(Self::from_data(real))
    }

    fn get_creation_time(&self) -> Instant {
        self.created
    }

    fn initiate(&mut self, initiation: Initiation) {
        match initiation {
            Initiation::Translate => self.translate_behavior.moving = true,
            Initiation::Rotate => (),
            Initiation::Scale => self.prescale_size = self.data.get_real().radius,
            Initiation::Spread { point, center } => {
                self.spread_behavior = SpreadBehavior {
                    point,
                    origin: center,
                    start: self.get_center(),
                };
            }
            Initiation::Revolve { point, center } => self.revolve_behavior.set(&center, &self.get_center(), &point),
            Initiation::Dilate { point, center } => {
                self.prescale_size = self.data.get_real().radius;
                self.dilate_behavior = DilateBehavior {
                    point,
                    origin: center,
                    start: self.get_center(),
                };
            }
        }
    }

    fn get_center(&self) -> glm::Vec2 {
        let CircleData { position, .. } = self.data.get_animated();
        position.reveal()
    }

    fn get_name(&self) -> &str {
        self.name.as_deref().unwrap_or("Unnamed Circle")
    }

    fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    fn get_opaque_handles(&self) -> Vec<glm::Vec2> {
        vec![self.get_rotate_handle_location(&glm::zero())]
    }
}
