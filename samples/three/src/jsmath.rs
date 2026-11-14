//! `Math.sin`, `Math.cos` and `Math.pow` as pinned Chromium computes them.
//!
//! Chromium 147's `Math.sin` and `Math.cos` are correctly rounded except
//! for some near-ties, and its `Math.pow` is correctly rounded except for
//! exact ties (docs/phase2-plan.md, P2-6a). `libm` is within an ulp but not
//! always correctly rounded, and the orbit state carries a 1-ulp difference
//! forward through every later operation. These evaluate in double-double
//! arithmetic (about 100 bits) and round once, so a result can differ from
//! the correctly rounded one only when the exact value lies within about
//! 2^-95 of a rounding boundary. Arguments outside the ranges handled here
//! (no value this port produces) fall back to `libm`.
//!
//! Written for this port from the textbook methods (Cody-Waite reduction,
//! Taylor series, Newton's method); no glibc (LGPL) code.

/// π/2 in three parts of 33 bits (fdlibm's `pio2_1`, `pio2_2`, `pio2_3`),
/// so `k * part` is exact for `k < 2^20`.
const PIO2_1: f64 = f64::from_bits(0x3FF9_21FB_5440_0000);
const PIO2_2: f64 = f64::from_bits(0x3DD0_B461_1A60_0000);
const PIO2_3: f64 = f64::from_bits(0x3BA3_198A_2E00_0000);
/// The rest of π/2 beyond the three parts.
const PIO2_3T: f64 = f64::from_bits(0x397B_839A_2520_49C1);
const MAX_REDUCED: f64 = 1.0e5;

#[derive(Clone, Copy)]
struct Dd {
    hi: f64,
    lo: f64,
}

fn two_sum(a: f64, b: f64) -> Dd {
    let s = a + b;
    let bb = s - a;
    let lo = (a - (s - bb)) + (b - bb);
    Dd { hi: s, lo }
}

fn quick_two_sum(a: f64, b: f64) -> Dd {
    let s = a + b;
    Dd {
        hi: s,
        lo: b - (s - a),
    }
}

/// Splits into two 26-bit halves (Dekker).
fn split(a: f64) -> (f64, f64) {
    let t = 134_217_729.0 * a;
    let hi = t - (t - a);
    (hi, a - hi)
}

fn two_prod(a: f64, b: f64) -> Dd {
    let p = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    let lo = ((ah * bh - p) + ah * bl + al * bh) + al * bl;
    Dd { hi: p, lo }
}

fn add(a: Dd, b: Dd) -> Dd {
    let s = two_sum(a.hi, b.hi);
    let t = two_sum(a.lo, b.lo);
    let s = quick_two_sum(s.hi, s.lo + t.hi);
    quick_two_sum(s.hi, s.lo + t.lo)
}

fn mul(a: Dd, b: Dd) -> Dd {
    let p = two_prod(a.hi, b.hi);
    quick_two_sum(p.hi, p.lo + (a.hi * b.lo + a.lo * b.hi))
}

fn div_f64(a: Dd, b: f64) -> Dd {
    let q1 = a.hi / b;
    let p = two_prod(q1, b);
    let r = add(
        a,
        Dd {
            hi: -p.hi,
            lo: -p.lo,
        },
    );
    let q2 = r.hi / b;
    quick_two_sum(q1, q2)
}

/// `x - k·π/2` as a double-double, with `k` the nearest integer to
/// `x / (π/2)`.
fn reduce(x: f64) -> (Dd, i64) {
    let k = (x * core::f64::consts::FRAC_2_PI).round();
    // x - k·p1 is exact: k·p1 is exact and within a factor of two of x.
    let mut r = Dd {
        hi: x - k * PIO2_1,
        lo: 0.0,
    };
    for part in [PIO2_2, PIO2_3, PIO2_3T] {
        let p = two_prod(k, part);
        r = add(
            r,
            Dd {
                hi: -p.hi,
                lo: -p.lo,
            },
        );
    }
    (r, k as i64)
}

/// sin and cos of `r`, `|r| <= π/4`, by their Taylor series.
fn sin_cos_reduced(r: Dd) -> (Dd, Dd) {
    let r2 = mul(r, r);
    let neg_r2 = Dd {
        hi: -r2.hi,
        lo: -r2.lo,
    };
    let mut sin = r;
    let mut term = r;
    let mut n = 1.0;
    // (π/4)^31 / 31! < 2^-110.
    while n < 31.0 {
        term = div_f64(mul(term, neg_r2), (n + 1.0) * (n + 2.0));
        sin = add(sin, term);
        n += 2.0;
    }
    let mut cos = Dd { hi: 1.0, lo: 0.0 };
    let mut term = Dd { hi: 1.0, lo: 0.0 };
    let mut n = 0.0;
    while n < 30.0 {
        term = div_f64(mul(term, neg_r2), (n + 1.0) * (n + 2.0));
        cos = add(cos, term);
        n += 2.0;
    }
    (sin, cos)
}

fn round(v: Dd) -> f64 {
    v.hi + v.lo
}

/// `Math.sin(x)`.
pub fn sin(x: f64) -> f64 {
    if !x.is_finite() || x.abs() > MAX_REDUCED {
        return libm::sin(x);
    }
    if x.abs() < 1.0e-8 {
        // sin x = x(1 - x²/6 + …) rounds to x; keeps -0.
        return x;
    }
    let (r, k) = reduce(x);
    let (s, c) = sin_cos_reduced(r);
    match k.rem_euclid(4) {
        0 => round(s),
        1 => round(c),
        2 => -round(s),
        _ => -round(c),
    }
}

/// `Math.cos(x)`.
pub fn cos(x: f64) -> f64 {
    if !x.is_finite() || x.abs() > MAX_REDUCED {
        return libm::cos(x);
    }
    if x.abs() < 1.0e-8 {
        // cos x = 1 - x²/2 + … rounds to 1.
        return 1.0;
    }
    let (r, k) = reduce(x);
    let (s, c) = sin_cos_reduced(r);
    match k.rem_euclid(4) {
        0 => round(c),
        1 => -round(s),
        2 => -round(c),
        _ => round(s),
    }
}

/// ln 2 in three parts.
const LN2: [f64; 3] = [
    core::f64::consts::LN_2,
    f64::from_bits(0x3C7A_BC9E_3B39_803F),
    f64::from_bits(0x3907_B57A_079A_1934),
];

/// `e^a` as `(m, k)` with `e^a = m·2^k`, `m` in [1/√2, √2], for
/// `|a| < 700`.
fn exp_dd(a: Dd) -> (Dd, i32) {
    let k = (a.hi / LN2[0]).round();
    let mut r = a;
    for part in LN2 {
        let p = two_prod(k, part);
        r = add(
            r,
            Dd {
                hi: -p.hi,
                lo: -p.lo,
            },
        );
    }
    // |r| <= ln2/2; e^r = (e^(r/16))^16, and |r/16| < 0.022 needs 18
    // terms of the series for 2^-110.
    let r = Dd {
        hi: r.hi / 16.0,
        lo: r.lo / 16.0,
    };
    let mut sum = Dd { hi: 1.0, lo: 0.0 };
    let mut term = Dd { hi: 1.0, lo: 0.0 };
    for n in 1..=18 {
        term = div_f64(mul(term, r), f64::from(n));
        sum = add(sum, term);
    }
    for _ in 0..4 {
        sum = mul(sum, sum);
    }
    (sum, k as i32)
}

/// `ln x` for finite `x > 0`: `libm`'s, then one step of Newton's method
/// with a second-order correction.
fn log_dd(x: f64) -> Dd {
    let y0 = libm::log(x);
    let (m, k) = exp_dd(Dd { hi: -y0, lo: 0.0 });
    // d = x·e^-y0 - 1, so ln x = y0 + ln(1 + d) = y0 + d - d²/2 + …
    let xm = mul(Dd { hi: x, lo: 0.0 }, m);
    let xm = Dd {
        hi: libm::ldexp(xm.hi, k),
        lo: libm::ldexp(xm.lo, k),
    };
    let d = add(xm, Dd { hi: -1.0, lo: 0.0 });
    let d2 = mul(d, d);
    let d3 = mul(d2, d);
    let correction = add(
        d,
        add(
            Dd {
                hi: -d2.hi / 2.0,
                lo: -d2.lo / 2.0,
            },
            div_f64(d3, 3.0),
        ),
    );
    add(Dd { hi: y0, lo: 0.0 }, correction)
}

/// `Math.pow(x, y)`.
pub fn pow(x: f64, y: f64) -> f64 {
    if y.is_nan() {
        return f64::NAN;
    }
    if y == 0.0 {
        return 1.0;
    }
    if (x == 1.0 || x == -1.0) && y.is_infinite() {
        // ECMAScript, unlike C, has no exception for ±1 to an infinity.
        return f64::NAN;
    }
    if !(x > 0.0 && x.is_finite() && y.is_finite()) || x == 1.0 {
        return libm::pow(x, y);
    }
    let t = mul(Dd { hi: y, lo: 0.0 }, log_dd(x));
    if t.hi.is_nan() || t.hi.abs() >= 700.0 {
        return libm::pow(x, y);
    }
    let (m, k) = exp_dd(t);
    let result = libm::ldexp(round(m), k);
    if result.abs() < f64::MIN_POSITIVE {
        // Subnormal: rounding and then scaling would round twice.
        return libm::pow(x, y);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chromium 147's `Math.sin`, `Math.cos` and `Math.pow` on arguments
    /// where `libm` is an ulp off; all correctly rounded (checked against an
    /// 80-digit series).
    #[test]
    fn matches_chromium_where_libm_is_an_ulp_off() {
        assert_eq!(sin(0.9324633753703315), 0.8030901967007242);
        assert_eq!(cos(0.6431398822687484), 0.8002166837161679);
        assert_eq!(pow(0.95, 100.0), 0.0059205292203339975);
        assert_eq!(pow(0.95, 32.065751507307645), 0.193059271342603);
        assert_eq!(pow(0.95, 1.4989719696728911), 0.9259942901319974);
    }

    #[test]
    fn special_values() {
        assert_eq!(sin(-0.0).to_bits(), (-0.0f64).to_bits());
        assert_eq!(cos(0.0), 1.0);
        assert!(sin(f64::NAN).is_nan());
        assert_eq!(pow(0.95, 0.0), 1.0);
        assert_eq!(pow(f64::NAN, 0.0), 1.0);
        assert!(pow(1.0, f64::INFINITY).is_nan());
        assert_eq!(pow(0.95, f64::INFINITY), 0.0);
        assert_eq!(pow(2.0, 10.0), 1024.0);
        assert_eq!(pow(4.0, 0.5), 2.0);
    }
}
