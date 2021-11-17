use nalgebra_glm as glm;

// Since we cannot extend interpolation::Lerp, we must make our own trait
pub trait Lerpable {
    type Scalar;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self;
}

impl Lerpable for f32 {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        interpolation::Lerp::lerp(self, other, scalar)
    }
}

impl Lerpable for f64 {
    type Scalar = f64;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        interpolation::Lerp::lerp(self, other, scalar)
    }
}

impl Lerpable for glm::Vec2 {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        glm::vec2(
            interpolation::Lerp::lerp(&self.x, &other.x, scalar),
            interpolation::Lerp::lerp(&self.y, &other.y, scalar),
        )
    }
}

impl Lerpable for glm::Vec3 {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        glm::vec3(
            interpolation::Lerp::lerp(&self.x, &other.x, scalar),
            interpolation::Lerp::lerp(&self.y, &other.y, scalar),
            interpolation::Lerp::lerp(&self.z, &other.z, scalar),
        )
    }
}

impl Lerpable for crate::color::Color {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        let (hue, saturation, value) = self.to_hsv();
        let (other_hue, other_saturation, other_value) = other.to_hsv();
        crate::color::Color::from_hsv(
            interpolation::Lerp::lerp(&hue, &other_hue, scalar),
            interpolation::Lerp::lerp(&saturation, &other_saturation, scalar),
            interpolation::Lerp::lerp(&value, &other_value, scalar),
        )
    }
}
