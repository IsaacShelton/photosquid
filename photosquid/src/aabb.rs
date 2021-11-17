#[allow(clippy::upper_case_acronyms)]
pub struct AABB {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl AABB {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x + w,
            max_y: y + h,
        }
    }

    pub fn intersecting_point(&self, x: f32, y: f32) -> bool {
        x > self.min_x && x < self.max_x && y > self.min_y && y < self.max_y
    }

    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }

    pub fn center_x(&self) -> f32 {
        (self.min_x + self.max_x) / 2.0
    }

    pub fn center_y(&self) -> f32 {
        (self.min_y + self.max_y) / 2.0
    }
}
