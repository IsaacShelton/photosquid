use crate::{accumulator::Accumulator, camera::Camera, math::angle_difference};
use angular_units::Rad;
use nalgebra_glm as glm;

pub fn get_delta_rotation(
    center: &glm::Vec2,
    existing_rotation: Rad<f32>,
    mouse_position: &glm::Vec2,
    rotation_accumulator: &Accumulator<Rad<f32>>,
    camera: &Camera,
) -> Rad<f32> {
    let screen_center = camera.apply(center);

    let old_rotation = existing_rotation + *rotation_accumulator.residue();
    let new_rotation = Rad(-1.0 * (mouse_position.y - screen_center.y).atan2(mouse_position.x - screen_center.x));

    angle_difference(old_rotation, new_rotation)
}
