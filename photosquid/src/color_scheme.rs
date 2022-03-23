use crate::color::Color;

pub struct ColorScheme {
    pub background: Color,
    pub light_ribbon: Color,
    pub dark_ribbon: Color,
    pub foreground: Color,
    pub dark_foreground: Color,
    pub really_dark_foreground: Color,
    pub input: Color,
    pub error: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            background: Color::from_hex("#2C2F33FF"),
            light_ribbon: Color::from_hex("#2f3136"),
            dark_ribbon: Color::from_hex("#23272AFF"),
            foreground: Color::from_hex("#7289DA"),
            dark_foreground: Color::from_hex("#5772D3"),
            really_dark_foreground: Color::from_hex("#3D5CCC"),
            input: Color::from_hex("#40444B"),
            error: Color::from_hex("#ed2326"),
        }
    }
}
