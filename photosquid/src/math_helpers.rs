pub trait DivOrZero {
    fn div_or_zero(&self, denominator: Self) -> Self;
}

impl DivOrZero for f32 {
    fn div_or_zero(&self, denominator: f32) -> f32 {
        if denominator == 0.0 {
            0.0
        } else {
            *self / denominator
        }
    }
}

impl DivOrZero for f64 {
    fn div_or_zero(&self, denominator: f64) -> f64 {
        if denominator == 0.0 {
            0.0
        } else {
            *self / denominator
        }
    }
}
