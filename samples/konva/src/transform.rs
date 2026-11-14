//! A port of Konva's `Transform` (`Util.ts`): a 2D affine matrix
//! `[a, b, c, d, e, f]`, as `CanvasRenderingContext2D.setTransform` takes
//! it.

use crate::js;

/// `{x, y}`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector2d {
    pub x: f64,
    pub y: f64,
}

/// `{x, y, width, height}`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// What `decompose()` returns. Angles are in degrees (`Konva.angleDeg`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decomposed {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub skew_x: f64,
    pub skew_y: f64,
}

/// `Math.PI / 180`, as Konva's `PI_OVER_180`.
pub const PI_OVER_180: f64 = std::f64::consts::PI / 180.0;
/// `180 / Math.PI`, as Konva's `DEG180_OVER_PI`.
pub const DEG180_OVER_PI: f64 = 180.0 / std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub m: [f64; 6],
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

impl Transform {
    pub fn new() -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        }
    }

    pub fn from_matrix(m: [f64; 6]) -> Self {
        Self { m }
    }

    pub fn reset(&mut self) {
        self.m = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    }

    pub fn point(&self, p: Vector2d) -> Vector2d {
        let m = &self.m;
        Vector2d {
            x: m[0] * p.x + m[2] * p.y + m[4],
            y: m[1] * p.x + m[3] * p.y + m[5],
        }
    }

    /// `_getTransformedRect`: the axis-aligned bounds of a rectangle after
    /// this transform.
    pub fn transformed_rect(&self, rect: IRect) -> IRect {
        let [a, b, c, d, e, f] = self.m;
        let (x1, y1, x2, y2) = (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height);
        let xs = [
            a * x1 + c * y1 + e,
            a * x2 + c * y1 + e,
            a * x2 + c * y2 + e,
            a * x1 + c * y2 + e,
        ];
        let ys = [
            b * x1 + d * y1 + f,
            b * x2 + d * y1 + f,
            b * x2 + d * y2 + f,
            b * x1 + d * y2 + f,
        ];
        let min_x = js::min(&xs);
        let min_y = js::min(&ys);
        IRect {
            x: min_x,
            y: min_y,
            width: js::max(&xs) - min_x,
            height: js::max(&ys) - min_y,
        }
    }

    pub fn translate(&mut self, x: f64, y: f64) -> &mut Self {
        self.m[4] += self.m[0] * x + self.m[2] * y;
        self.m[5] += self.m[1] * x + self.m[3] * y;
        self
    }

    pub fn scale(&mut self, sx: f64, sy: f64) -> &mut Self {
        self.m[0] *= sx;
        self.m[1] *= sx;
        self.m[2] *= sy;
        self.m[3] *= sy;
        self
    }

    /// `rotate(rad)`.
    pub fn rotate(&mut self, rad: f64) -> &mut Self {
        let c = libm::cos(rad);
        let s = libm::sin(rad);
        let m11 = self.m[0] * c + self.m[2] * s;
        let m12 = self.m[1] * c + self.m[3] * s;
        let m21 = self.m[0] * -s + self.m[2] * c;
        let m22 = self.m[1] * -s + self.m[3] * c;
        self.m[0] = m11;
        self.m[1] = m12;
        self.m[2] = m21;
        self.m[3] = m22;
        self
    }

    pub fn translation(&self) -> Vector2d {
        Vector2d {
            x: self.m[4],
            y: self.m[5],
        }
    }

    pub fn skew(&mut self, sx: f64, sy: f64) -> &mut Self {
        let m11 = self.m[0] + self.m[2] * sy;
        let m12 = self.m[1] + self.m[3] * sy;
        let m21 = self.m[2] + self.m[0] * sx;
        let m22 = self.m[3] + self.m[1] * sx;
        self.m[0] = m11;
        self.m[1] = m12;
        self.m[2] = m21;
        self.m[3] = m22;
        self
    }

    pub fn multiply(&mut self, matrix: &Transform) -> &mut Self {
        let n = &matrix.m;
        let m11 = self.m[0] * n[0] + self.m[2] * n[1];
        let m12 = self.m[1] * n[0] + self.m[3] * n[1];
        let m21 = self.m[0] * n[2] + self.m[2] * n[3];
        let m22 = self.m[1] * n[2] + self.m[3] * n[3];
        let dx = self.m[0] * n[4] + self.m[2] * n[5] + self.m[4];
        let dy = self.m[1] * n[4] + self.m[3] * n[5] + self.m[5];
        self.m = [m11, m12, m21, m22, dx, dy];
        self
    }

    pub fn is_invertible(&self) -> bool {
        self.m[0] * self.m[3] - self.m[1] * self.m[2] != 0.0
    }

    pub fn invert(&mut self) -> &mut Self {
        let m = self.m;
        let d = 1.0 / (m[0] * m[3] - m[1] * m[2]);
        self.m = [
            m[3] * d,
            -m[1] * d,
            -m[2] * d,
            m[0] * d,
            d * (m[2] * m[5] - m[3] * m[4]),
            d * (m[1] * m[4] - m[0] * m[5]),
        ];
        self
    }

    pub fn decompose(&self) -> Decomposed {
        let [a, b, c, d, e, f] = self.m;
        let delta = a * d - b * c;
        let mut r = Decomposed {
            x: e,
            y: f,
            rotation: 0.0,
            scale_x: 0.0,
            scale_y: 0.0,
            skew_x: 0.0,
            skew_y: 0.0,
        };
        // `delta && expr`: a zero (or NaN) delta is the result itself.
        let and = |delta: f64, value: f64| if js::truthy(delta) { value } else { delta };
        if a != 0.0 || b != 0.0 {
            let rr = (a * a + b * b).sqrt();
            r.rotation = if b > 0.0 {
                libm::acos(a / rr)
            } else {
                -libm::acos(a / rr)
            };
            r.scale_x = rr;
            r.scale_y = delta / rr;
            r.skew_x = and(delta, (a * c + b * d) / delta);
            r.skew_y = 0.0;
        } else if c != 0.0 || d != 0.0 {
            let s = (c * c + d * d).sqrt();
            r.rotation = std::f64::consts::PI / 2.0
                - if d > 0.0 {
                    libm::acos(-c / s)
                } else {
                    -libm::acos(c / s)
                };
            r.scale_x = delta / s;
            r.scale_y = s;
            r.skew_x = 0.0;
            r.skew_y = and(delta, (a * c + b * d) / delta);
        }
        // `Util._getRotation`: radians to degrees.
        r.rotation *= DEG180_OVER_PI;
        r
    }
}
