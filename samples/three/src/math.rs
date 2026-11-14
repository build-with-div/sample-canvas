//! three.js's math classes (`src/math/`), as far as the camera and
//! `OrbitControls` use them: `Vector2`, `Vector3`, `Quaternion`, `Euler`,
//! `Matrix4` and `Spherical`, with `MathUtils.clamp`.
//!
//! Each method keeps three.js's arithmetic in its order, so results match
//! bit for bit where the elementary functions agree. `Math.sin`, `cos`,
//! `tan`, `asin`, `acos` and `atan2` come from the `libm` crate (a port of
//! musl, from fdlibm); V8's are fdlibm ports too but can differ by an ulp.

/// `MathUtils.DEG2RAD`.
pub const DEG2RAD: f64 = std::f64::consts::PI / 180.0;
/// `MathUtils.RAD2DEG`.
pub const RAD2DEG: f64 = 180.0 / std::f64::consts::PI;

/// `Math.max(a, b)` as JavaScript computes it: NaN wins, and +0 > -0.
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a == 0.0 && (a.is_sign_positive() || b.is_sign_positive()) {
            0.0
        } else {
            a
        }
    } else if a > b {
        a
    } else {
        b
    }
}

/// `Math.min(a, b)` as JavaScript computes it: NaN wins, and -0 < +0.
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a == 0.0 && (a.is_sign_negative() || b.is_sign_negative()) {
            -0.0
        } else {
            a
        }
    } else if a < b {
        a
    } else {
        b
    }
}

/// `MathUtils.clamp(value, min, max)`.
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    js_max(min, js_min(max, value))
}

/// JavaScript truthiness of a number.
pub fn truthy(v: f64) -> bool {
    v != 0.0 && !v.is_nan()
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector2 {
    pub x: f64,
    pub y: f64,
}

impl Vector2 {
    pub fn set(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    /// `subVectors(a, b)`.
    pub fn sub_vectors(&mut self, a: Vector2, b: Vector2) {
        self.x = a.x - b.x;
        self.y = a.y - b.y;
    }

    pub fn multiply_scalar(&mut self, s: f64) {
        self.x *= s;
        self.y *= s;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Vector3 {
        Vector3 { x, y, z }
    }

    pub fn set(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self
    }

    pub fn add(&mut self, v: Vector3) -> &mut Self {
        self.x += v.x;
        self.y += v.y;
        self.z += v.z;
        self
    }

    pub fn sub(&mut self, v: Vector3) -> &mut Self {
        self.x -= v.x;
        self.y -= v.y;
        self.z -= v.z;
        self
    }

    /// `subVectors(a, b)`.
    pub fn sub_vectors(&mut self, a: Vector3, b: Vector3) -> &mut Self {
        self.x = a.x - b.x;
        self.y = a.y - b.y;
        self.z = a.z - b.z;
        self
    }

    pub fn add_scaled_vector(&mut self, v: Vector3, s: f64) -> &mut Self {
        self.x += v.x * s;
        self.y += v.y * s;
        self.z += v.z * s;
        self
    }

    pub fn multiply_scalar(&mut self, s: f64) -> &mut Self {
        self.x *= s;
        self.y *= s;
        self.z *= s;
        self
    }

    pub fn divide_scalar(&mut self, s: f64) -> &mut Self {
        self.multiply_scalar(1.0 / s)
    }

    pub fn dot(&self, v: Vector3) -> f64 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    pub fn length_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// `normalize()`: divides by the length, or by 1 if it is 0 or NaN.
    pub fn normalize(&mut self) -> &mut Self {
        let l = self.length();
        self.divide_scalar(if truthy(l) { l } else { 1.0 })
    }

    /// `clampLength(min, max)`.
    pub fn clamp_length(&mut self, min: f64, max: f64) -> &mut Self {
        let length = self.length();
        self.divide_scalar(if truthy(length) { length } else { 1.0 })
            .multiply_scalar(clamp(length, min, max))
    }

    /// `crossVectors(a, b)`.
    pub fn cross_vectors(&mut self, a: Vector3, b: Vector3) -> &mut Self {
        let (ax, ay, az) = (a.x, a.y, a.z);
        let (bx, by, bz) = (b.x, b.y, b.z);
        self.x = ay * bz - az * by;
        self.y = az * bx - ax * bz;
        self.z = ax * by - ay * bx;
        self
    }

    pub fn distance_to_squared(&self, v: Vector3) -> f64 {
        let (dx, dy, dz) = (self.x - v.x, self.y - v.y, self.z - v.z);
        dx * dx + dy * dy + dz * dz
    }

    pub fn distance_to(&self, v: Vector3) -> f64 {
        self.distance_to_squared(v).sqrt()
    }

    /// `applyQuaternion(q)`.
    pub fn apply_quaternion(&mut self, q: &Quaternion) -> &mut Self {
        let (vx, vy, vz) = (self.x, self.y, self.z);
        let (qx, qy, qz, qw) = (q.x, q.y, q.z, q.w);
        let tx = 2.0 * (qy * vz - qz * vy);
        let ty = 2.0 * (qz * vx - qx * vz);
        let tz = 2.0 * (qx * vy - qy * vx);
        self.x = vx + qw * tx + qy * tz - qz * ty;
        self.y = vy + qw * ty + qz * tx - qx * tz;
        self.z = vz + qw * tz + qx * ty - qy * tx;
        self
    }

    /// `applyMatrix4(m)`, with the perspective divide.
    pub fn apply_matrix4(&mut self, m: &Matrix4) -> &mut Self {
        let (x, y, z) = (self.x, self.y, self.z);
        let e = &m.elements;
        let w = 1.0 / (e[3] * x + e[7] * y + e[11] * z + e[15]);
        self.x = (e[0] * x + e[4] * y + e[8] * z + e[12]) * w;
        self.y = (e[1] * x + e[5] * y + e[9] * z + e[13]) * w;
        self.z = (e[2] * x + e[6] * y + e[10] * z + e[14]) * w;
        self
    }

    /// `transformDirection(m)`.
    pub fn transform_direction(&mut self, m: &Matrix4) -> &mut Self {
        let (x, y, z) = (self.x, self.y, self.z);
        let e = &m.elements;
        self.x = e[0] * x + e[4] * y + e[8] * z;
        self.y = e[1] * x + e[5] * y + e[9] * z;
        self.z = e[2] * x + e[6] * y + e[10] * z;
        self.normalize()
    }

    /// `setFromSpherical(s)`.
    pub fn set_from_spherical(&mut self, s: &Spherical) -> &mut Self {
        self.set_from_spherical_coords(s.radius, s.phi, s.theta)
    }

    /// `setFromSphericalCoords(radius, phi, theta)`.
    pub fn set_from_spherical_coords(&mut self, radius: f64, phi: f64, theta: f64) -> &mut Self {
        let sin_phi_radius = crate::jsmath::sin(phi) * radius;
        self.x = sin_phi_radius * crate::jsmath::sin(theta);
        self.y = crate::jsmath::cos(phi) * radius;
        self.z = sin_phi_radius * crate::jsmath::cos(theta);
        self
    }

    /// `setFromMatrixPosition(m)`.
    pub fn set_from_matrix_position(&mut self, m: &Matrix4) -> &mut Self {
        let e = &m.elements;
        self.set(e[12], e[13], e[14])
    }

    /// `setFromMatrixColumn(m, index)`.
    pub fn set_from_matrix_column(&mut self, m: &Matrix4, index: usize) -> &mut Self {
        let e = &m.elements;
        self.set(e[index * 4], e[index * 4 + 1], e[index * 4 + 2])
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Default for Quaternion {
    fn default() -> Self {
        Quaternion {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }
}

impl Quaternion {
    pub fn set(&mut self, x: f64, y: f64, z: f64, w: f64) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.w = w;
    }

    pub fn identity(&mut self) {
        self.set(0.0, 0.0, 0.0, 1.0);
    }

    /// `setFromEuler(euler)`.
    pub fn set_from_euler(&mut self, euler: &Euler) -> &mut Self {
        let (x, y, z) = (euler.x, euler.y, euler.z);
        let c1 = crate::jsmath::cos(x / 2.0);
        let c2 = crate::jsmath::cos(y / 2.0);
        let c3 = crate::jsmath::cos(z / 2.0);
        let s1 = crate::jsmath::sin(x / 2.0);
        let s2 = crate::jsmath::sin(y / 2.0);
        let s3 = crate::jsmath::sin(z / 2.0);
        match euler.order {
            EulerOrder::Xyz => {
                self.x = s1 * c2 * c3 + c1 * s2 * s3;
                self.y = c1 * s2 * c3 - s1 * c2 * s3;
                self.z = c1 * c2 * s3 + s1 * s2 * c3;
                self.w = c1 * c2 * c3 - s1 * s2 * s3;
            }
            EulerOrder::Yxz => {
                self.x = s1 * c2 * c3 + c1 * s2 * s3;
                self.y = c1 * s2 * c3 - s1 * c2 * s3;
                self.z = c1 * c2 * s3 - s1 * s2 * c3;
                self.w = c1 * c2 * c3 + s1 * s2 * s3;
            }
            EulerOrder::Zxy => {
                self.x = s1 * c2 * c3 - c1 * s2 * s3;
                self.y = c1 * s2 * c3 + s1 * c2 * s3;
                self.z = c1 * c2 * s3 + s1 * s2 * c3;
                self.w = c1 * c2 * c3 - s1 * s2 * s3;
            }
            EulerOrder::Zyx => {
                self.x = s1 * c2 * c3 - c1 * s2 * s3;
                self.y = c1 * s2 * c3 + s1 * c2 * s3;
                self.z = c1 * c2 * s3 - s1 * s2 * c3;
                self.w = c1 * c2 * c3 + s1 * s2 * s3;
            }
            EulerOrder::Yzx => {
                self.x = s1 * c2 * c3 + c1 * s2 * s3;
                self.y = c1 * s2 * c3 + s1 * c2 * s3;
                self.z = c1 * c2 * s3 - s1 * s2 * c3;
                self.w = c1 * c2 * c3 - s1 * s2 * s3;
            }
            EulerOrder::Xzy => {
                self.x = s1 * c2 * c3 - c1 * s2 * s3;
                self.y = c1 * s2 * c3 - s1 * c2 * s3;
                self.z = c1 * c2 * s3 + s1 * s2 * c3;
                self.w = c1 * c2 * c3 + s1 * s2 * s3;
            }
        }
        self
    }

    /// `setFromRotationMatrix(m)`: `m`'s upper 3×3 must be a pure rotation.
    pub fn set_from_rotation_matrix(&mut self, m: &Matrix4) -> &mut Self {
        let te = &m.elements;
        let (m11, m12, m13) = (te[0], te[4], te[8]);
        let (m21, m22, m23) = (te[1], te[5], te[9]);
        let (m31, m32, m33) = (te[2], te[6], te[10]);
        let trace = m11 + m22 + m33;
        if trace > 0.0 {
            let s = 0.5 / (trace + 1.0).sqrt();
            self.w = 0.25 / s;
            self.x = (m32 - m23) * s;
            self.y = (m13 - m31) * s;
            self.z = (m21 - m12) * s;
        } else if m11 > m22 && m11 > m33 {
            let s = 2.0 * (1.0 + m11 - m22 - m33).sqrt();
            self.w = (m32 - m23) / s;
            self.x = 0.25 * s;
            self.y = (m12 + m21) / s;
            self.z = (m13 + m31) / s;
        } else if m22 > m33 {
            let s = 2.0 * (1.0 + m22 - m11 - m33).sqrt();
            self.w = (m13 - m31) / s;
            self.x = (m12 + m21) / s;
            self.y = 0.25 * s;
            self.z = (m23 + m32) / s;
        } else {
            let s = 2.0 * (1.0 + m33 - m11 - m22).sqrt();
            self.w = (m21 - m12) / s;
            self.x = (m13 + m31) / s;
            self.y = (m23 + m32) / s;
            self.z = 0.25 * s;
        }
        self
    }

    /// `setFromUnitVectors(vFrom, vTo)`.
    pub fn set_from_unit_vectors(&mut self, v_from: Vector3, v_to: Vector3) -> &mut Self {
        let mut r = v_from.dot(v_to) + 1.0;
        if r < 1e-8 {
            r = 0.0;
            if v_from.x.abs() > v_from.z.abs() {
                self.set(-v_from.y, v_from.x, 0.0, r);
            } else {
                self.set(0.0, -v_from.z, v_from.y, r);
            }
        } else {
            self.x = v_from.y * v_to.z - v_from.z * v_to.y;
            self.y = v_from.z * v_to.x - v_from.x * v_to.z;
            self.z = v_from.x * v_to.y - v_from.y * v_to.x;
            self.w = r;
        }
        self.normalize()
    }

    /// `invert()`, which is `conjugate()`.
    pub fn invert(&mut self) -> &mut Self {
        self.x *= -1.0;
        self.y *= -1.0;
        self.z *= -1.0;
        self
    }

    pub fn dot(&self, v: &Quaternion) -> f64 {
        self.x * v.x + self.y * v.y + self.z * v.z + self.w * v.w
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt()
    }

    pub fn normalize(&mut self) -> &mut Self {
        let mut l = self.length();
        if l == 0.0 {
            self.set(0.0, 0.0, 0.0, 1.0);
        } else {
            l = 1.0 / l;
            self.x *= l;
            self.y *= l;
            self.z *= l;
            self.w *= l;
        }
        self
    }
}

/// `Euler`'s rotation orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EulerOrder {
    #[default]
    Xyz,
    Yxz,
    Zxy,
    Zyx,
    Yzx,
    Xzy,
}

impl EulerOrder {
    pub const ALL: [EulerOrder; 6] = [
        EulerOrder::Xyz,
        EulerOrder::Yxz,
        EulerOrder::Zxy,
        EulerOrder::Zyx,
        EulerOrder::Yzx,
        EulerOrder::Xzy,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            EulerOrder::Xyz => "XYZ",
            EulerOrder::Yxz => "YXZ",
            EulerOrder::Zxy => "ZXY",
            EulerOrder::Zyx => "ZYX",
            EulerOrder::Yzx => "YZX",
            EulerOrder::Xzy => "XZY",
        }
    }

    pub fn parse(s: &str) -> Option<EulerOrder> {
        EulerOrder::ALL.into_iter().find(|o| o.as_str() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Euler {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub order: EulerOrder,
}

impl Euler {
    /// `setFromRotationMatrix(m, order)`.
    pub fn set_from_rotation_matrix(&mut self, m: &Matrix4, order: EulerOrder) -> &mut Self {
        let te = &m.elements;
        let (m11, m12, m13) = (te[0], te[4], te[8]);
        let (m21, m22, m23) = (te[1], te[5], te[9]);
        let (m31, m32, m33) = (te[2], te[6], te[10]);
        match order {
            EulerOrder::Xyz => {
                self.y = libm::asin(clamp(m13, -1.0, 1.0));
                if m13.abs() < 0.9999999 {
                    self.x = libm::atan2(-m23, m33);
                    self.z = libm::atan2(-m12, m11);
                } else {
                    self.x = libm::atan2(m32, m22);
                    self.z = 0.0;
                }
            }
            EulerOrder::Yxz => {
                self.x = libm::asin(-clamp(m23, -1.0, 1.0));
                if m23.abs() < 0.9999999 {
                    self.y = libm::atan2(m13, m33);
                    self.z = libm::atan2(m21, m22);
                } else {
                    self.y = libm::atan2(-m31, m11);
                    self.z = 0.0;
                }
            }
            EulerOrder::Zxy => {
                self.x = libm::asin(clamp(m32, -1.0, 1.0));
                if m32.abs() < 0.9999999 {
                    self.y = libm::atan2(-m31, m33);
                    self.z = libm::atan2(-m12, m22);
                } else {
                    self.y = 0.0;
                    self.z = libm::atan2(m21, m11);
                }
            }
            EulerOrder::Zyx => {
                self.y = libm::asin(-clamp(m31, -1.0, 1.0));
                if m31.abs() < 0.9999999 {
                    self.x = libm::atan2(m32, m33);
                    self.z = libm::atan2(m21, m11);
                } else {
                    self.x = 0.0;
                    self.z = libm::atan2(-m12, m22);
                }
            }
            EulerOrder::Yzx => {
                self.z = libm::asin(clamp(m21, -1.0, 1.0));
                if m21.abs() < 0.9999999 {
                    self.x = libm::atan2(-m23, m22);
                    self.y = libm::atan2(-m31, m11);
                } else {
                    self.x = 0.0;
                    self.y = libm::atan2(m13, m33);
                }
            }
            EulerOrder::Xzy => {
                self.z = libm::asin(-clamp(m12, -1.0, 1.0));
                if m12.abs() < 0.9999999 {
                    self.x = libm::atan2(m32, m22);
                    self.y = libm::atan2(m13, m11);
                } else {
                    self.x = libm::atan2(-m23, m33);
                    self.y = 0.0;
                }
            }
        }
        self.order = order;
        self
    }

    /// `setFromQuaternion(q, order)`.
    pub fn set_from_quaternion(&mut self, q: &Quaternion, order: EulerOrder) -> &mut Self {
        let mut m = Matrix4::default();
        m.make_rotation_from_quaternion(q);
        self.set_from_rotation_matrix(&m, order)
    }
}

/// A 4×4 matrix, column-major as three.js stores it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix4 {
    pub elements: [f64; 16],
}

impl Default for Matrix4 {
    fn default() -> Self {
        let mut elements = [0.0; 16];
        elements[0] = 1.0;
        elements[5] = 1.0;
        elements[10] = 1.0;
        elements[15] = 1.0;
        Matrix4 { elements }
    }
}

/// The clip space a projection maps to: WebGL's `z` in [-1, 1], or
/// WebGPU's in [0, 1].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoordinateSystem {
    #[default]
    WebGl,
    WebGpu,
}

impl Matrix4 {
    /// `set(n11, n12, ..., n44)`: the arguments in row-major order.
    pub fn set_all(&mut self, rows: [f64; 16]) -> &mut Self {
        for row in 0..4 {
            for col in 0..4 {
                self.elements[col * 4 + row] = rows[row * 4 + col];
            }
        }
        self
    }

    pub fn identity(&mut self) -> &mut Self {
        *self = Matrix4::default();
        self
    }

    /// `compose(position, quaternion, scale)`.
    pub fn compose(&mut self, position: Vector3, q: &Quaternion, scale: Vector3) -> &mut Self {
        let te = &mut self.elements;
        let (x, y, z, w) = (q.x, q.y, q.z, q.w);
        let (x2, y2, z2) = (x + x, y + y, z + z);
        let (xx, xy, xz) = (x * x2, x * y2, x * z2);
        let (yy, yz, zz) = (y * y2, y * z2, z * z2);
        let (wx, wy, wz) = (w * x2, w * y2, w * z2);
        let (sx, sy, sz) = (scale.x, scale.y, scale.z);
        te[0] = (1.0 - (yy + zz)) * sx;
        te[1] = (xy + wz) * sx;
        te[2] = (xz - wy) * sx;
        te[3] = 0.0;
        te[4] = (xy - wz) * sy;
        te[5] = (1.0 - (xx + zz)) * sy;
        te[6] = (yz + wx) * sy;
        te[7] = 0.0;
        te[8] = (xz + wy) * sz;
        te[9] = (yz - wx) * sz;
        te[10] = (1.0 - (xx + yy)) * sz;
        te[11] = 0.0;
        te[12] = position.x;
        te[13] = position.y;
        te[14] = position.z;
        te[15] = 1.0;
        self
    }

    /// `makeRotationFromQuaternion(q)`.
    pub fn make_rotation_from_quaternion(&mut self, q: &Quaternion) -> &mut Self {
        self.compose(Vector3::new(0.0, 0.0, 0.0), q, Vector3::new(1.0, 1.0, 1.0))
    }

    /// `lookAt(eye, target, up)`: sets the rotation part only.
    pub fn look_at(&mut self, eye: Vector3, target: Vector3, up: Vector3) -> &mut Self {
        let mut z = Vector3::default();
        let mut x = Vector3::default();
        let mut y = Vector3::default();
        z.sub_vectors(eye, target);
        if z.length_sq() == 0.0 {
            z.z = 1.0;
        }
        z.normalize();
        x.cross_vectors(up, z);
        if x.length_sq() == 0.0 {
            if up.z.abs() == 1.0 {
                z.x += 0.0001;
            } else {
                z.z += 0.0001;
            }
            z.normalize();
            x.cross_vectors(up, z);
        }
        x.normalize();
        y.cross_vectors(z, x);
        let te = &mut self.elements;
        te[0] = x.x;
        te[4] = y.x;
        te[8] = z.x;
        te[1] = x.y;
        te[5] = y.y;
        te[9] = z.y;
        te[2] = x.z;
        te[6] = y.z;
        te[10] = z.z;
        self
    }

    /// `multiplyMatrices(a, b)`.
    pub fn multiply_matrices(&mut self, a: &Matrix4, b: &Matrix4) -> &mut Self {
        let ae = a.elements;
        let be = b.elements;
        let (a11, a12, a13, a14) = (ae[0], ae[4], ae[8], ae[12]);
        let (a21, a22, a23, a24) = (ae[1], ae[5], ae[9], ae[13]);
        let (a31, a32, a33, a34) = (ae[2], ae[6], ae[10], ae[14]);
        let (a41, a42, a43, a44) = (ae[3], ae[7], ae[11], ae[15]);
        let (b11, b12, b13, b14) = (be[0], be[4], be[8], be[12]);
        let (b21, b22, b23, b24) = (be[1], be[5], be[9], be[13]);
        let (b31, b32, b33, b34) = (be[2], be[6], be[10], be[14]);
        let (b41, b42, b43, b44) = (be[3], be[7], be[11], be[15]);
        let te = &mut self.elements;
        te[0] = a11 * b11 + a12 * b21 + a13 * b31 + a14 * b41;
        te[4] = a11 * b12 + a12 * b22 + a13 * b32 + a14 * b42;
        te[8] = a11 * b13 + a12 * b23 + a13 * b33 + a14 * b43;
        te[12] = a11 * b14 + a12 * b24 + a13 * b34 + a14 * b44;
        te[1] = a21 * b11 + a22 * b21 + a23 * b31 + a24 * b41;
        te[5] = a21 * b12 + a22 * b22 + a23 * b32 + a24 * b42;
        te[9] = a21 * b13 + a22 * b23 + a23 * b33 + a24 * b43;
        te[13] = a21 * b14 + a22 * b24 + a23 * b34 + a24 * b44;
        te[2] = a31 * b11 + a32 * b21 + a33 * b31 + a34 * b41;
        te[6] = a31 * b12 + a32 * b22 + a33 * b32 + a34 * b42;
        te[10] = a31 * b13 + a32 * b23 + a33 * b33 + a34 * b43;
        te[14] = a31 * b14 + a32 * b24 + a33 * b34 + a34 * b44;
        te[3] = a41 * b11 + a42 * b21 + a43 * b31 + a44 * b41;
        te[7] = a41 * b12 + a42 * b22 + a43 * b32 + a44 * b42;
        te[11] = a41 * b13 + a42 * b23 + a43 * b33 + a44 * b43;
        te[15] = a41 * b14 + a42 * b24 + a43 * b34 + a44 * b44;
        self
    }

    /// `determinantAffine()`: the upper 3×3's determinant.
    pub fn determinant_affine(&self) -> f64 {
        let te = &self.elements;
        let (n11, n12, n13) = (te[0], te[4], te[8]);
        let (n21, n22, n23) = (te[1], te[5], te[9]);
        let (n31, n32, n33) = (te[2], te[6], te[10]);
        n11 * (n22 * n33 - n23 * n32) - n12 * (n21 * n33 - n23 * n31)
            + n13 * (n21 * n32 - n22 * n31)
    }

    /// `invert()`; a singular matrix becomes all zeros.
    pub fn invert(&mut self) -> &mut Self {
        let te = &mut self.elements;
        let (n11, n21, n31, n41) = (te[0], te[1], te[2], te[3]);
        let (n12, n22, n32, n42) = (te[4], te[5], te[6], te[7]);
        let (n13, n23, n33, n43) = (te[8], te[9], te[10], te[11]);
        let (n14, n24, n34, n44) = (te[12], te[13], te[14], te[15]);
        let t1 = n11 * n22 - n21 * n12;
        let t2 = n11 * n32 - n31 * n12;
        let t3 = n11 * n42 - n41 * n12;
        let t4 = n21 * n32 - n31 * n22;
        let t5 = n21 * n42 - n41 * n22;
        let t6 = n31 * n42 - n41 * n32;
        let t7 = n13 * n24 - n23 * n14;
        let t8 = n13 * n34 - n33 * n14;
        let t9 = n13 * n44 - n43 * n14;
        let t10 = n23 * n34 - n33 * n24;
        let t11 = n23 * n44 - n43 * n24;
        let t12 = n33 * n44 - n43 * n34;
        let det = t1 * t12 - t2 * t11 + t3 * t10 + t4 * t9 - t5 * t8 + t6 * t7;
        if det == 0.0 {
            *te = [0.0; 16];
            return self;
        }
        let det_inv = 1.0 / det;
        te[0] = (n22 * t12 - n32 * t11 + n42 * t10) * det_inv;
        te[1] = (n31 * t11 - n21 * t12 - n41 * t10) * det_inv;
        te[2] = (n24 * t6 - n34 * t5 + n44 * t4) * det_inv;
        te[3] = (n33 * t5 - n23 * t6 - n43 * t4) * det_inv;
        te[4] = (n32 * t9 - n12 * t12 - n42 * t8) * det_inv;
        te[5] = (n11 * t12 - n31 * t9 + n41 * t8) * det_inv;
        te[6] = (n34 * t3 - n14 * t6 - n44 * t2) * det_inv;
        te[7] = (n13 * t6 - n33 * t3 + n43 * t2) * det_inv;
        te[8] = (n12 * t11 - n22 * t9 + n42 * t7) * det_inv;
        te[9] = (n21 * t9 - n11 * t11 - n41 * t7) * det_inv;
        te[10] = (n14 * t5 - n24 * t3 + n44 * t1) * det_inv;
        te[11] = (n23 * t3 - n13 * t5 - n43 * t1) * det_inv;
        te[12] = (n22 * t8 - n12 * t10 - n32 * t7) * det_inv;
        te[13] = (n11 * t10 - n21 * t8 + n31 * t7) * det_inv;
        te[14] = (n24 * t2 - n14 * t4 - n34 * t1) * det_inv;
        te[15] = (n13 * t4 - n23 * t2 + n33 * t1) * det_inv;
        self
    }

    /// `decompose(position, quaternion, scale)`.
    pub fn decompose(&self) -> (Vector3, Quaternion, Vector3) {
        let te = &self.elements;
        let position = Vector3::new(te[12], te[13], te[14]);
        let mut quaternion = Quaternion::default();
        let det = self.determinant_affine();
        if det == 0.0 {
            return (position, quaternion, Vector3::new(1.0, 1.0, 1.0));
        }
        let mut sx = Vector3::new(te[0], te[1], te[2]).length();
        let sy = Vector3::new(te[4], te[5], te[6]).length();
        let sz = Vector3::new(te[8], te[9], te[10]).length();
        if det < 0.0 {
            sx = -sx;
        }
        let mut m1 = *self;
        let (inv_sx, inv_sy, inv_sz) = (1.0 / sx, 1.0 / sy, 1.0 / sz);
        m1.elements[0] *= inv_sx;
        m1.elements[1] *= inv_sx;
        m1.elements[2] *= inv_sx;
        m1.elements[4] *= inv_sy;
        m1.elements[5] *= inv_sy;
        m1.elements[6] *= inv_sy;
        m1.elements[8] *= inv_sz;
        m1.elements[9] *= inv_sz;
        m1.elements[10] *= inv_sz;
        quaternion.set_from_rotation_matrix(&m1);
        (position, quaternion, Vector3::new(sx, sy, sz))
    }

    /// `makePerspective(left, right, top, bottom, near, far,
    /// coordinateSystem, reversedDepth)`.
    #[allow(clippy::too_many_arguments)]
    pub fn make_perspective(
        &mut self,
        left: f64,
        right: f64,
        top: f64,
        bottom: f64,
        near: f64,
        far: f64,
        coordinate_system: CoordinateSystem,
        reversed_depth: bool,
    ) -> &mut Self {
        let x = 2.0 * near / (right - left);
        let y = 2.0 * near / (top - bottom);
        let a = (right + left) / (right - left);
        let b = (top + bottom) / (top - bottom);
        let (c, d) = if reversed_depth {
            (near / (far - near), (far * near) / (far - near))
        } else {
            match coordinate_system {
                CoordinateSystem::WebGl => (
                    -(far + near) / (far - near),
                    (-2.0 * far * near) / (far - near),
                ),
                CoordinateSystem::WebGpu => (-far / (far - near), (-far * near) / (far - near)),
            }
        };
        self.set_all([
            x, 0.0, a, 0.0, //
            0.0, y, b, 0.0, //
            0.0, 0.0, c, d, //
            0.0, 0.0, -1.0, 0.0,
        ])
    }
}

/// Spherical coordinates: `phi` from the +y axis, `theta` around it from
/// +z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spherical {
    pub radius: f64,
    pub phi: f64,
    pub theta: f64,
}

impl Default for Spherical {
    fn default() -> Self {
        Spherical {
            radius: 1.0,
            phi: 0.0,
            theta: 0.0,
        }
    }
}

impl Spherical {
    pub fn set(&mut self, radius: f64, phi: f64, theta: f64) {
        self.radius = radius;
        self.phi = phi;
        self.theta = theta;
    }

    /// `makeSafe()`: keeps `phi` off the poles.
    pub fn make_safe(&mut self) -> &mut Self {
        const EPS: f64 = 0.000001;
        self.phi = clamp(self.phi, EPS, std::f64::consts::PI - EPS);
        self
    }

    /// `setFromVector3(v)`.
    pub fn set_from_vector3(&mut self, v: Vector3) -> &mut Self {
        self.set_from_cartesian_coords(v.x, v.y, v.z)
    }

    /// `setFromCartesianCoords(x, y, z)`.
    pub fn set_from_cartesian_coords(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.radius = (x * x + y * y + z * z).sqrt();
        if self.radius == 0.0 {
            self.theta = 0.0;
            self.phi = 0.0;
        } else {
            self.theta = libm::atan2(x, z);
            self.phi = libm::acos(clamp(y / self.radius, -1.0, 1.0));
        }
        self
    }
}
