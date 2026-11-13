//! Measurement scaffolding for the canvas probes: an app-owned Skia raster
//! surface shown in a Div element through the only public pixel path Div
//! has today.
//!
//! div-gap: W1 (`HTMLCanvasElement` bitmap) and W2 (`CanvasRenderingContext2D`).
//! This is not a canvas implementation and samples must not use it as one;
//! see docs/design.md §9. It draws with the app's own `skia-safe`, reads the
//! pixels back, wraps them in Div's undocumented `image/x-openui-rgba8`
//! transport and sets the result as the element's image resource. Each frame
//! needs a new SHA-256 string, because Div caches decoded images by that
//! string, and Div never frees a registered resource.

use div::{Document, Element, Error};
use skia_safe::{surfaces, AlphaType, ColorSpace, ColorType, ImageInfo, Surface};

/// Div's raw image transport (`div-paint/src/image_resource.rs`): `OUIR`,
/// version 1, three zero bytes, width and height as little-endian u32, then
/// tightly packed unpremultiplied sRGB RGBA8.
pub const RGBA8_MIME: &str = "image/x-openui-rgba8";

/// An app-owned raster surface of `width` by `height` device pixels.
pub fn raster_surface(width: i32, height: i32) -> Surface {
    surfaces::raster_n32_premul((width, height)).expect("a raster surface")
}

/// Read `surface` back as Div's RGBA8 transport bytes.
pub fn rgba8_resource(surface: &mut Surface) -> Vec<u8> {
    let width = surface.width();
    let height = surface.height();
    let info = ImageInfo::new(
        (width, height),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        Some(ColorSpace::new_srgb()),
    );
    let stride = width as usize * 4;
    let mut bytes = Vec::with_capacity(16 + stride * height as usize);
    bytes.extend_from_slice(b"OUIR");
    bytes.extend_from_slice(&[1, 0, 0, 0]);
    bytes.extend_from_slice(&(width as u32).to_le_bytes());
    bytes.extend_from_slice(&(height as u32).to_le_bytes());
    bytes.resize(16 + stride * height as usize, 0);
    let ok = surface.image_snapshot().read_pixels(
        &info,
        &mut bytes[16..],
        stride,
        (0, 0),
        skia_safe::image::CachingHint::Disallow,
    );
    assert!(ok, "read back the app surface");
    bytes
}

/// Pushes frames of an app-owned surface into one Div element.
pub struct SurfaceBridge {
    document: Document,
    element: Element,
    /// CSS size the element shows the surface at.
    css_size: (f32, f32),
    frame: u64,
    tag: String,
}

impl SurfaceBridge {
    pub fn new(document: &Document, element: &Element, css_size: (f32, f32), tag: &str) -> Self {
        Self {
            document: document.clone(),
            element: element.clone(),
            css_size,
            frame: 0,
            tag: tag.to_owned(),
        }
    }

    /// Show the surface's current pixels. Returns the frame number.
    pub fn present(&mut self, surface: &mut Surface) -> Result<u64, Error> {
        self.frame += 1;
        // div-gap: the "sha256" is only a cache key here; a real hash of
        // each frame would cost more than the frame. It must be unique.
        let key = format!("{}-frame-{}", self.tag, self.frame);
        let resource = self.document.register_image_resource(
            format!("app-surface://{key}"),
            RGBA8_MIME,
            key,
            rgba8_resource(surface),
        )?;
        self.element
            .set_image_resource(resource, Some(self.css_size))?;
        Ok(self.frame)
    }

    pub fn frames(&self) -> u64 {
        self.frame
    }
}
