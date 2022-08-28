use crate::smooth::{Lerpable, Smooth};
use lazy_static::lazy_static;
use more_asserts::assert_le;
use nalgebra_glm as glm;

lazy_static! {
    pub static ref IDENTITY_CAMERA: Camera = Camera::identity(glm::zero());
}

#[derive(Copy, Clone)]
pub struct Camera {
    pub position: glm::Vec2,
    pub zoom: f32,
    pub window: glm::Vec2,
}

impl Camera {
    pub fn identity(window: glm::Vec2) -> Camera {
        Camera {
            position: glm::zero(),
            zoom: 1.0,
            window,
        }
    }

    pub fn mat(&self) -> glm::Mat4 {
        let matrix = glm::translation(&glm::vec2_to_vec3(&(-self.position * self.zoom)));
        let matrix = glm::translate(&matrix, &glm::vec2_to_vec3(&(0.5 * self.window)));
        let matrix = glm::scale(&matrix, &glm::vec3(self.zoom, self.zoom, 1.0));
        let matrix = glm::translate(&matrix, &glm::vec2_to_vec3(&(-0.5 * self.window)));
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
        use crate::math::DivOrZero;
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

    pub fn with_position(&self, position: glm::Vec2) -> Camera {
        Camera {
            position,
            zoom: self.zoom,
            window: self.window,
        }
    }

    pub fn with_zoom(&self, zoom: f32) -> Camera {
        Camera {
            position: self.position,
            zoom,
            window: self.window,
        }
    }

    pub fn to_view(&self) -> (glm::Vec2, glm::Vec2) {
        let view_size = self.window / self.zoom;
        (self.position - 0.5 * view_size, self.position + 0.5 * view_size)
    }

    pub fn view_to_components(window: &glm::Vec2, view: (glm::Vec2, glm::Vec2)) -> (glm::Vec2, f32) {
        assert_le!(view.0, view.1);

        let view_size = view.1 - view.0;

        // Determine position by top-right anchor plus half view size
        let position = view.0 + 0.5 * view_size;

        // Determine zoom only by x component
        let zoom = window.x / view_size.x;

        (position, zoom)
    }
}

#[cfg(test)]
mod tests {
    use super::{Camera, EasySmoothCamera};
    use crate::smooth::Smooth;
    use nalgebra_glm as glm;

    #[test]
    fn camera_1() {
        let window = glm::vec2(1000.0, 2000.0);
        let view = (glm::vec2(0.0, 0.0), glm::vec2(100.0, 200.0));
        let components = Camera::view_to_components(&window, view);

        assert_eq!(components.0, glm::vec2(50.0, 100.0));
        assert_eq!(components.1, 10.0);
    }

    #[test]
    fn camera_2() {
        let window = glm::vec2(1000.0, 2000.0);
        let view = (glm::vec2(900.0, 1800.0), glm::vec2(1000.0, 2000.0));
        let components = Camera::view_to_components(&window, view);

        assert_eq!(components.0, glm::vec2(950.0, 1900.0));
        assert_eq!(components.1, 10.0);
    }

    #[test]
    fn camera_3() {
        let window = glm::vec2(1000.0, 2000.0);
        let view = (glm::vec2(900.0, 1800.0), glm::vec2(1000.0, 2000.0));
        let components = Camera::view_to_components(&window, view);

        let camera = Camera {
            position: components.0,
            zoom: components.1,
            window,
        };

        assert_eq!(view, camera.to_view());
    }

    #[test]
    fn camera_4() {
        let window = glm::vec2(1000.0, 2000.0);
        let view = (glm::vec2(0.0, 0.0), glm::vec2(1000.0, 2000.0));
        let components = Camera::view_to_components(&window, view);

        let mut camera = Smooth::new(Camera::identity(glm::zero()), None);

        camera.set(Camera {
            position: components.0,
            zoom: components.1,
            window,
        });

        let view = camera.get_real().to_view();
        assert_eq!(view.0, glm::vec2(0.0, 0.0));
        assert_eq!(view.1, glm::vec2(1000.0, 2000.0));

        let focus = glm::zero();
        camera.zoom_point(2.0, &focus);

        let view = camera.get_real().to_view();
        assert_eq!(view, (glm::vec2(0.0, 0.0), glm::vec2(500.0, 1000.0)));

        let components = Camera::view_to_components(&window, view);
        assert_eq!(components.0, glm::vec2(250.0, 500.0));
        assert_eq!(components.1, 2.0);
    }

    #[test]
    fn camera_5() {
        let window = glm::vec2(1000.0, 2000.0);
        let view = (glm::vec2(0.0, 0.0), glm::vec2(1000.0, 2000.0));
        let components = Camera::view_to_components(&window, view);

        let mut camera = Smooth::new(Camera::identity(glm::zero()), None);

        camera.set(Camera {
            position: components.0,
            zoom: components.1,
            window,
        });

        let view = camera.get_real().to_view();
        assert_eq!(view.0, glm::vec2(0.0, 0.0));
        assert_eq!(view.1, glm::vec2(1000.0, 2000.0));

        let focus = components.0;
        camera.zoom_point(2.0, &focus);

        let view = camera.get_real().to_view();
        assert_eq!(view, (glm::vec2(250.0, 500.0), glm::vec2(750.0, 1500.0)));

        let components = Camera::view_to_components(&window, view);
        assert_eq!(components.0, glm::vec2(500.0, 1000.0));
        assert_eq!(components.1, 2.0);
    }

    #[test]
    fn camera_6() {
        let window = glm::vec2(1000.0, 2000.0);
        let view = (glm::vec2(200.0, 400.0), glm::vec2(400.0, 800.0));
        let components = Camera::view_to_components(&window, view);

        let mut camera = Smooth::new(Camera::identity(glm::zero()), None);

        camera.set(Camera {
            position: components.0,
            zoom: components.1,
            window,
        });

        let view = camera.get_real().to_view();
        assert_eq!(view.0, glm::vec2(200.0, 400.0));
        assert_eq!(view.1, glm::vec2(400.0, 800.0));

        let focus = glm::vec2(200.0, 400.0);
        camera.zoom_point(2.0, &focus);

        let view = camera.get_real().to_view();
        assert_eq!(view, (glm::vec2(200.0, 400.0), glm::vec2(300.0, 600.0)));

        let components = Camera::view_to_components(&window, view);
        assert_eq!(components.0, glm::vec2(250.0, 500.0));
        assert_eq!(components.1, 10.0);
    }
}

impl Lerpable for Camera {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        Camera {
            position: Lerpable::lerp(&self.position, &other.position, scalar),
            zoom: self.zoom.lerp(&other.zoom, scalar),
            window: other.window,
        }
    }
}

pub trait EasySmoothCamera {
    fn set_location(&mut self, location: glm::Vec2);

    fn zoom(&mut self, zoom: f32);

    fn zoom_point(&mut self, zoom_multiplier: f32, point: &glm::Vec2);

    fn increase_zoom(&mut self);

    fn decrease_zoom(&mut self);
}

impl EasySmoothCamera for Smooth<Camera> {
    fn set_location(&mut self, location: glm::Vec2) {
        self.set(self.get_animated().with_position(location));
    }

    fn zoom(&mut self, zoom_multiplier: f32) {
        let center = self.get_real().position;
        self.zoom_point(zoom_multiplier, &center);
    }

    fn zoom_point(&mut self, zoom_multiplier: f32, point_in_world_space: &glm::Vec2) {
        let window = self.get_real().window;
        let original_view = self.get_real().to_view();

        let original_view_size = original_view.1 - original_view.0;
        let ratios = (point_in_world_space - original_view.0).component_div(&original_view_size);

        assert!(ratios.x >= 0.0 && ratios.x <= 1.0);
        assert!(ratios.y >= 0.0 && ratios.y <= 1.0);

        let top_left = point_in_world_space - ratios.component_mul(&original_view_size) / zoom_multiplier;
        let bottom_right = top_left + original_view_size / zoom_multiplier;

        let (position, zoom) = Camera::view_to_components(&window, (top_left, bottom_right));
        self.set(Camera { position, zoom, window });
    }

    fn increase_zoom(&mut self) {
        self.zoom(1.2);
    }

    fn decrease_zoom(&mut self) {
        self.zoom(1.0 / 1.2);
    }
}
