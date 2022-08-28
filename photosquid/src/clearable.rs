use crate::color::Color;
use glium::{framebuffer::SimpleFrameBuffer, Frame};

pub trait Clearable {
    fn clear_with(&self, frame: &mut Frame);
    fn clear_framebuffer_with(&self, framebuffer: &mut SimpleFrameBuffer);
}

impl Clearable for Color {
    fn clear_with(&self, frame: &mut Frame) {
        use glium::Surface;
        let values: [f32; 4] = self.into();
        frame.clear_color_srgb(values[0], values[1], values[2], values[3]);
    }

    fn clear_framebuffer_with(&self, framebuffer: &mut SimpleFrameBuffer) {
        use glium::Surface;
        let values: [f32; 4] = self.into();
        framebuffer.clear_color_srgb(values[0], values[1], values[2], values[3]);
    }
}
