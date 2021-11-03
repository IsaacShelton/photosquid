#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(hex: &str) -> Self {
        let raster_color = raster::Color::hex(hex).unwrap_or_else(|_| raster::Color::black());
        Self {
            r: raster_color.r as f32 / 255.0,
            g: raster_color.g as f32 / 255.0,
            b: raster_color.b as f32 / 255.0,
            a: raster_color.a as f32 / 255.0,
        }
    }

    // Creates a 'Color' from hue, saturation, and value parameters
    // Where h, s, and v are in the range [0.0, 1.0]
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        use palette::FromColor;
        use std::f32::consts::TAU;
        let hsv = palette::Hsv::new(palette::RgbHue::from_radians(h * TAU), s, v);
        let srgb = palette::Srgb::from_color(hsv);
        Color::new(srgb.red, srgb.green, srgb.blue, 1.0)
    }

    // Converts a 'Color' to hue, saturation, and value parameters
    // Where h, s, and v are in the range [0.0, 1.0]
    pub fn to_hsv(&self) -> (f32, f32, f32) {
        use palette::FromColor;
        use std::f32::consts::TAU;
        let hsv = palette::Hsv::from_color(self.to_palette_srgb());
        let hue = hsv.hue.to_radians().rem_euclid(TAU) / TAU;
        let hue = if hue == 1.0 { 0.0 } else { hue };
        let palette::Hsv { saturation, value, .. } = hsv;
        (hue, saturation, value)
    }

    pub fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    pub fn to_palette_srgb(&self) -> palette::Srgb {
        palette::Srgb::new(self.r, self.g, self.b)
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

impl From<Color> for (f32, f32, f32, f32) {
    fn from(color: Color) -> Self {
        (color.r, color.g, color.b, color.a)
    }
}

impl From<Color> for [u8; 4] {
    fn from(color: Color) -> Self {
        [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ]
    }
}

impl From<&Color> for [f32; 4] {
    fn from(color: &Color) -> Self {
        [color.r, color.g, color.b, color.a]
    }
}

impl From<&Color> for (f32, f32, f32, f32) {
    fn from(color: &Color) -> Self {
        (color.r, color.g, color.b, color.a)
    }
}

impl From<&Color> for [u8; 4] {
    fn from(color: &Color) -> Self {
        [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ]
    }
}
