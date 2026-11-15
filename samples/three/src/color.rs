//! three.js's `Color` (`src/math/Color.js`) and `ColorManagement`
//! (`src/math/ColorManagement.js`), as far as a hex color in the default
//! working color space (linear sRGB) needs them.

use crate::jsmath;

/// A color in three.js's working color space, linear sRGB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    /// `new Color(hex)`: `setHex(hex, SRGBColorSpace)`, which converts the
    /// sRGB channels to the working color space.
    pub fn from_hex(hex: u32) -> Color {
        let channel = |shift: u32| f64::from((hex >> shift) & 255) / 255.0;
        // `ColorManagement.colorSpaceToWorking(color, SRGBColorSpace)`:
        // sRGB and linear sRGB share primaries, so only the transfer
        // function applies.
        Color {
            r: srgb_to_linear(channel(16)),
            g: srgb_to_linear(channel(8)),
            b: srgb_to_linear(channel(0)),
        }
    }
}

/// `SRGBToLinear(c)`.
pub fn srgb_to_linear(c: f64) -> f64 {
    if c < 0.04045 {
        c * 0.0773993808
    } else {
        jsmath::pow(c * 0.9478672986 + 0.0521327014, 2.4)
    }
}
