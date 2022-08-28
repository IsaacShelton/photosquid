use glium::{
    backend::Facade,
    program::{ProgramCreationError, ProgramCreationInput},
    Program,
};

pub fn from_code_that_outputs_srgb<'a, F: ?Sized>(
    facade: &F,
    vertex_shader: &'a str,
    fragment_shader: &'a str,
    geometry_shader: Option<&'a str>,
    outputs_srgb: bool,
) -> Result<Program, ProgramCreationError>
where
    F: Facade,
{
    Program::new(
        facade,
        ProgramCreationInput::SourceCode {
            vertex_shader,
            fragment_shader,
            geometry_shader,
            tessellation_control_shader: None,
            tessellation_evaluation_shader: None,
            transform_feedback_varyings: None,
            outputs_srgb,
            uses_point_size: false,
        },
    )
}
