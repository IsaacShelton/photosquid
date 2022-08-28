use crate::{color::Color, matrix::reach_inside_mat4, mesh::MeshXyz, press_animation::PressAnimation, render_ctx::RenderCtx, smooth::Smooth};
use angular_units::Angle;
use glium::Display;
use nalgebra_glm as glm;
use std::time::{Duration, Instant};

pub struct IconButton<T> {
    pub key: T,

    mesh: MeshXyz,
    position: glm::Vec3,
    relative_scale: f32,
    instant: Option<Instant>,
    duration: Duration,
    animation: PressAnimation,
    focused: bool,
    color: Option<Smooth<Color>>,
}

impl<T> IconButton<T> {
    pub fn new(obj_src_code: &str, animation: PressAnimation, key: T, display: &Display) -> Self {
        Self::new_from_mesh(MeshXyz::new(obj_src_code, display), animation, key)
    }

    pub fn new_from_mesh(mesh: MeshXyz, animation: PressAnimation, key: T) -> Self {
        Self {
            mesh,
            position: glm::vec3(0.0, 0.0, 0.0),
            relative_scale: 1.0,
            instant: None,
            duration: Duration::from_millis(1000),
            animation,
            focused: false,
            key,
            color: None,
        }
    }

    pub fn set_raw_position(&mut self, center_x: f32, center_y: f32) {
        self.position.x = center_x;
        self.position.y = center_y;
    }

    pub fn animate(&mut self, focus: bool) {
        if focus != self.focused {
            self.instant = Some(Instant::now());
            self.focused = focus;
        }
    }

    pub fn render(&mut self, ctx: &mut RenderCtx, color: &Color) {
        let animation_moment = if let Some(instant) = self.instant {
            let since_instant = Instant::now() - instant;
            let t = (since_instant.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0);
            self.animation.at_time(self.focused, t)
        } else {
            Default::default()
        };

        if let Some(smooth_color) = self.color.as_mut() {
            smooth_color.set(*color);
        } else {
            self.color = Some(Smooth::new(*color, None));
        }

        let identity = glm::identity::<f32, 4>();
        let real_scale = animation_moment.relative_scale * self.relative_scale * 24.0 * 0.5; // (times 0.5 since icons are 2x2 meters)
        let transformation = glm::translation(&self.position);
        let transformation = glm::rotate(&transformation, animation_moment.backwards_rotation.scalar(), &glm::vec3(1.0, 0.0, 0.0));
        let transformation = glm::rotate(&transformation, animation_moment.rotation.scalar(), &glm::vec3(0.0, 0.0, -1.0));
        let transformation = glm::scale(&transformation, &glm::vec3(real_scale, real_scale, 0.0));

        let uniforms = glium::uniform! {
            transformation: reach_inside_mat4(&transformation),
            view: reach_inside_mat4(&identity),
            projection: reach_inside_mat4(ctx.projection),
            color: Into::<[f32; 4]>::into(color)
        };

        ctx.draw(&self.mesh.vertex_buffer, &self.mesh.indices, ctx.color_shader, &uniforms, &Default::default())
            .unwrap();
    }
}
