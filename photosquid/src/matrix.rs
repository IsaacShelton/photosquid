use crate::glm;

pub fn reach_inside_mat4(matrix: &glm::Mat4) -> [[f32; 4]; 4] {
    return *matrix.as_ref();
}
