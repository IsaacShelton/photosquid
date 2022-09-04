use crate::camera::Camera;
use angular_units::{Angle, Rad};
use nalgebra_glm as glm;

pub trait Position {
    fn position(&self) -> glm::Vec2;
}

pub trait Rotation {
    fn rotation(&self) -> Rad<f32>;
}

pub trait Scale {
    fn scale(&self) -> f32;
}

pub fn get_rotate_handle(position: glm::Vec2, rotation: Rad<f32>, distance: f32, camera: &Camera) -> glm::Vec2 {
    let position = position + distance * glm::vec2(rotation.cos(), -rotation.sin());
    camera.apply(&position)
}
