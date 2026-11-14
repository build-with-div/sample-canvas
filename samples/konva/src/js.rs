//! The JavaScript number functions Konva's results depend on.

/// `Math.min(a, b, …)`: NaN if any argument is NaN.
pub fn min(values: &[f64]) -> f64 {
    let mut out = f64::INFINITY;
    for &v in values {
        if v.is_nan() {
            return f64::NAN;
        }
        // -0 is less than +0.
        if v < out || (v == 0.0 && out == 0.0 && v.is_sign_negative()) {
            out = v;
        }
    }
    out
}

/// `Math.max(a, b, …)`: NaN if any argument is NaN.
pub fn max(values: &[f64]) -> f64 {
    let mut out = f64::NEG_INFINITY;
    for &v in values {
        if v.is_nan() {
            return f64::NAN;
        }
        if v > out || (v == 0.0 && out == 0.0 && v.is_sign_positive()) {
            out = v;
        }
    }
    out
}

/// `Math.hypot(a, b)` as V8 computes it (`MathHypot` in V8's
/// math.tq): the arguments are scaled by the largest magnitude and their
/// squares summed with Kahan compensation. This is not the C library's
/// `hypot`, which can differ in the last bit.
pub fn hypot(values: &[f64]) -> f64 {
    let mut max = 0.0f64;
    let mut one_arg_is_nan = false;
    let abs: Vec<f64> = values.iter().map(|v| v.abs()).collect();
    for &a in &abs {
        if a.is_nan() {
            one_arg_is_nan = true;
        } else if a > max {
            max = a;
        }
    }
    if max == f64::INFINITY {
        return f64::INFINITY;
    }
    if one_arg_is_nan {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for &a in &abs {
        let n = a / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

/// JavaScript truthiness of a number.
pub fn truthy(v: f64) -> bool {
    v != 0.0 && !v.is_nan()
}
