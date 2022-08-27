use angular_units::Rad;
use nalgebra_glm as glm;

pub trait Lerpable {
    type Scalar;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self;
}

impl Lerpable for f32 {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        interpolation::Lerp::lerp(self, other, &scalar)
    }
}

impl Lerpable for f64 {
    type Scalar = f64;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        interpolation::Lerp::lerp(self, other, &scalar)
    }
}

impl Lerpable for glm::Vec2 {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        glm::vec2(self.x.lerp(&other.x, scalar), self.y.lerp(&other.y, scalar))
    }
}

impl Lerpable for glm::Vec3 {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        glm::vec3(self.x.lerp(&other.x, scalar), self.y.lerp(&other.y, scalar), self.y.lerp(&other.z, scalar))
    }
}

impl Lerpable for Rad<f32> {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        angular_units::Interpolate::interpolate(self, other, scalar)
    }
}

impl Lerpable for crate::color::Color {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: Self::Scalar) -> Self {
        let (hue, saturation, value) = self.to_hsv();
        let (other_hue, other_saturation, other_value) = other.to_hsv();

        crate::color::Color::from_hsv(
            hue.lerp(&other_hue, scalar),
            saturation.lerp(&other_saturation, scalar),
            value.lerp(&other_value, scalar),
        )
    }
}
