use std::num::ParseIntError;

#[derive(Debug)]
pub enum RasterError {
    InvalidHex,
    HexParse(ParseIntError),
}

#[derive(Debug, Clone)]
pub struct RasterColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RasterColor {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn hex(hex: &str) -> Result<Self, RasterError> {
        if hex.len() == 9 && hex.starts_with('#') {
            // #FFFFFFFF (Red Green Blue Alpha)
            Ok(RasterColor {
                r: hex_dec(&hex[1..3])?,
                g: hex_dec(&hex[3..5])?,
                b: hex_dec(&hex[5..7])?,
                a: hex_dec(&hex[7..9])?,
            })
        } else if hex.len() == 7 && hex.starts_with('#') {
            // #FFFFFF (Red Green Blue)
            Ok(RasterColor {
                r: hex_dec(&hex[1..3])?,
                g: hex_dec(&hex[3..5])?,
                b: hex_dec(&hex[5..7])?,
                a: 255,
            })
        } else {
            Err(RasterError::InvalidHex)
        }
    }
}

fn hex_dec(hex_string: &str) -> Result<u8, RasterError> {
    u8::from_str_radix(hex_string, 16).map(|o| o as u8).map_err(RasterError::HexParse)
}
