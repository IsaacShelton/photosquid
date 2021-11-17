use nalgebra_glm as glm;

pub struct InteractionOptions {
    pub translation_snapping: f32,
    pub rotation_snapping: f32,
    pub duplication_offset: glm::Vec2,
    pub treat_selection_as_group: bool,
}

impl Default for InteractionOptions {
    fn default() -> Self {
        Self {
            translation_snapping: 1.0,
            rotation_snapping: 0.0,
            duplication_offset: glm::zero(),
            treat_selection_as_group: false,
        }
    }
}
