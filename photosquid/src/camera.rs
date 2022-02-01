use crate::smooth::{Lerpable, Smooth};
use nalgebra_glm as glm;

#[derive(Copy, Clone)]
pub struct Camera {
    pub location: glm::Vec2,
    pub zoom: f32,
    pub viewport: glm::Vec2,
}

impl Camera {
    pub fn identity(viewport: glm::Vec2) -> Camera {
        Camera {
            location: glm::zero(),
            zoom: 1.0,
            viewport,
        }
    }

    pub fn mat(&self) -> glm::Mat4 {
        let half_screen = glm::vec2_to_vec3(&(0.5 * self.viewport));
        let matrix = glm::translation(&half_screen);
        let matrix = glm::scale(&matrix, &glm::vec3(self.zoom, self.zoom, self.zoom));
        let matrix = glm::translate(&matrix, &glm::vec2_to_vec3(&self.location));
        let matrix = glm::translate(&matrix, &(-half_screen));
        matrix
    }

    pub fn inv_mat(&self) -> glm::Mat4 {
        glm::inverse(&self.mat())
    }

    // Point vectors
    pub fn apply(&self, point: &glm::Vec2) -> glm::Vec2 {
        glm::vec4_to_vec2(&(self.mat() * glm::vec4(point.x, point.y, 0.0, 1.0)))
    }

    // Point vectors
    pub fn apply_reverse(&self, point: &glm::Vec2) -> glm::Vec2 {
        glm::vec4_to_vec2(&(self.inv_mat() * glm::vec4(point.x, point.y, 0.0, 1.0)))
    }

    // Sizes / Distances
    pub fn apply_to_scale(&self, object_scale: f32) -> f32 {
        object_scale * self.zoom
    }

    // Sizes / Distances
    pub fn apply_reverse_to_scale(&self, object_scale: f32) -> f32 {
        use crate::math_helpers::DivOrZero;
        object_scale.div_or_zero(self.zoom)
    }

    // Directional vectors
    pub fn apply_to_vector(&self, point: &glm::Vec2) -> glm::Vec2 {
        glm::vec4_to_vec2(&(self.mat() * glm::vec4(point.x, point.y, 0.0, 0.0)))
    }

    // Directional vectors
    pub fn apply_reverse_to_vector(&self, point: &glm::Vec2) -> glm::Vec2 {
        glm::vec4_to_vec2(&(self.inv_mat() * glm::vec4(point.x, point.y, 0.0, 0.0)))
    }

    pub fn with_location(&self, location: glm::Vec2) -> Camera {
        Camera {
            location: location,
            zoom: self.zoom,
            viewport: self.viewport,
        }
    }

    pub fn with_zoom(&self, zoom: f32) -> Camera {
        Camera {
            location: self.location,
            zoom: zoom,
            viewport: self.viewport,
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            zoom: 1.0,
            location: Default::default(),
            viewport: glm::zero(),
        }
    }
}

impl Lerpable for Camera {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Camera {
            location: Lerpable::lerp(&self.location, &other.location, scalar),
            zoom: interpolation::Lerp::lerp(&self.zoom, &other.zoom, scalar),
            viewport: other.viewport,
        }
    }
}

pub trait EasySmoothCamera {
    fn set_location(&mut self, location: glm::Vec2);

    fn zoom(&mut self, zoom: f32);

    fn increase_zoom(&mut self);

    fn decrease_zoom(&mut self);
}

impl EasySmoothCamera for Smooth<Camera> {
    fn set_location(&mut self, location: glm::Vec2) {
        self.set(self.get_animated().with_location(location));
    }

    fn zoom(&mut self, zoom: f32) {
        let real = self.get_real();

        let previous_zoom = real.zoom;
        let previous_location = real.location;
        let viewport = real.viewport;

        self.set(Camera {
            location: previous_location,
            zoom: previous_zoom * zoom,
            viewport: viewport,
        });
    }

    fn increase_zoom(&mut self) {
        self.zoom(1.2);
    }

    fn decrease_zoom(&mut self) {
        self.zoom(1.0 / 1.2);
    }
}
