use crate::shader_helpers;

pub struct Shaders {
    pub color_shader: glium::Program,
    pub hue_value_picker_shader: glium::Program,
    pub saturation_picker_shader: glium::Program,
    pub rounded_rectangle_shader: glium::Program,
    pub television_shader: glium::Program,
}

impl Shaders {
    pub fn new(display: &glium::Display) -> Self {
        use shader_helpers::shader_from_source_that_outputs_srgb;

        let color_shader = shader_from_source_that_outputs_srgb(
            display,
            include_str!("_src_shaders/color/vertex.glsl"),
            include_str!("_src_shaders/color/fragment.glsl"),
            None,
            true,
        )
        .unwrap();

        let hue_value_picker_shader = shader_from_source_that_outputs_srgb(
            display,
            include_str!("_src_shaders/color_picker/hue_value/vertex.glsl"),
            include_str!("_src_shaders/color_picker/hue_value/fragment.glsl"),
            None,
            true,
        )
        .unwrap();

        let saturation_picker_shader = shader_from_source_that_outputs_srgb(
            display,
            include_str!("_src_shaders/color_picker/saturation/vertex.glsl"),
            include_str!("_src_shaders/color_picker/saturation/fragment.glsl"),
            None,
            true,
        )
        .unwrap();

        let rounded_rectangle_shader = shader_from_source_that_outputs_srgb(
            display,
            include_str!("_src_shaders/rounded_rectangle/vertex.glsl"),
            include_str!("_src_shaders/rounded_rectangle/fragment.glsl"),
            None,
            true,
        )
        .unwrap();

        let television_shader = shader_from_source_that_outputs_srgb(
            display,
            include_str!("_src_shaders/texture/vertex.glsl"),
            include_str!("_src_shaders/texture/fragment.glsl"),
            None,
            false,
        )
        .unwrap();
        Self {
            color_shader,
            hue_value_picker_shader,
            saturation_picker_shader,
            rounded_rectangle_shader,
            television_shader,
        }
    }
}
