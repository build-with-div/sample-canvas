//! Konva's `SceneCanvas` and `HitCanvas` (`Canvas.ts`): a canvas element,
//! its pixel ratio and size, and the context Konva draws through.

use crate::context::{Arg, CallRecorder, Context2d};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasKind {
    /// What the user sees.
    Scene,
    /// Konva's hidden canvas, where each shape is drawn in its color key.
    Hit,
}

#[derive(Debug)]
pub struct Canvas {
    pub kind: CanvasKind,
    pub pixel_ratio: f64,
    /// The bitmap size, in device pixels.
    pub width: f64,
    pub height: f64,
    logical_width: f64,
    logical_height: f64,
    /// div-gap: W1 HTMLCanvasElement and W2 CanvasRenderingContext2D. The
    /// canvas has no element or bitmap; its context records calls.
    pub context: CallRecorder,
}

impl Canvas {
    /// `new SceneCanvas()` or `new HitCanvas({pixelRatio: 1})`, 0×0.
    ///
    /// div-gap: W12 devicePixelRatio. Konva takes a scene canvas's pixel
    /// ratio from `window.devicePixelRatio`; Div exposes none, so it is 1,
    /// as in Konva without a window.
    pub fn new(kind: CanvasKind) -> Canvas {
        let mut canvas = Canvas {
            kind,
            pixel_ratio: 1.0,
            width: 0.0,
            height: 0.0,
            logical_width: 0.0,
            logical_height: 0.0,
            context: CallRecorder::default(),
        };
        if kind == CanvasKind::Hit {
            canvas
                .context
                .set_property("imageSmoothingEnabled", Arg::Bool(false));
        }
        canvas.set_size(0.0, 0.0);
        canvas
    }

    /// `_bitmapSize(size)`.
    fn bitmap_size(&self, size: f64) -> f64 {
        let size = if crate::js::truthy(size) { size } else { 0.0 };
        (size * self.pixel_ratio).floor()
    }

    /// `setSize(width, height)`. Resizing a canvas resets its context, so
    /// Konva carries `imageSmoothingEnabled` across and scales by the
    /// pixel ratio again.
    pub fn set_size(&mut self, width: f64, height: f64) {
        let (mut width, mut height) = (width, height);
        if !width.is_finite() || !height.is_finite() {
            width = 0.0;
            height = 0.0;
        }
        let width = if crate::js::truthy(width) { width } else { 0.0 };
        let height = if crate::js::truthy(height) {
            height
        } else {
            0.0
        };
        let image_smoothing = self.context.image_smoothing_enabled();
        self.logical_width = width;
        self.logical_height = height;
        self.width = self.bitmap_size(width);
        self.height = self.bitmap_size(height);
        self.context.scale(self.pixel_ratio, self.pixel_ratio);
        self.context
            .set_property("imageSmoothingEnabled", image_smoothing);
    }

    /// The size in CSS pixels, as `setSize` was last given it: the canvas
    /// element's `style.width` and `style.height`.
    pub fn logical_size(&self) -> (f64, f64) {
        (self.logical_width, self.logical_height)
    }

    /// `setSizeIfChanged(width, height)`.
    pub fn set_size_if_changed(&mut self, width: f64, height: f64) {
        if self.width != self.bitmap_size(width) || self.height != self.bitmap_size(height) {
            self.set_size(width, height);
        }
    }

    /// `context.clear()`: clears the whole canvas.
    pub fn clear(&mut self) {
        let (w, h) = (
            self.width / self.pixel_ratio,
            self.height / self.pixel_ratio,
        );
        self.context.clear_rect(0.0, 0.0, w, h);
    }
}
