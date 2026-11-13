//! The JavaScript number semantics rough.js relies on.
//!
//! rough.js is JavaScript, so its output depends on how JavaScript rounds,
//! converts and prints numbers. These helpers reproduce the ECMAScript
//! definitions the port needs, so a seeded drawing matches the original.

use std::cell::RefCell;

/// JavaScript truthiness of a number: `0`, `-0` and `NaN` are false.
pub(crate) fn truthy(x: f64) -> bool {
    x != 0.0 && !x.is_nan()
}

/// `x || fallback` for numbers.
pub(crate) fn or(x: f64, fallback: f64) -> f64 {
    if truthy(x) {
        x
    } else {
        fallback
    }
}

/// ECMAScript `ToInt32`.
pub fn to_int32(x: f64) -> i32 {
    if !x.is_finite() {
        return 0;
    }
    let m = x.trunc().rem_euclid(4_294_967_296.0);
    (m as u32) as i32
}

/// `Math.round`: the nearest integer, ties toward positive infinity, and the
/// sign of zero kept.
pub fn round(x: f64) -> f64 {
    if !x.is_finite() || x == x.trunc() {
        return x;
    }
    let floor = x.floor();
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded == 0.0 && x < 0.0 {
        -0.0
    } else {
        rounded
    }
}

/// `Math.sin`, `Math.cos`, `Math.tan`, `Math.asin` and `Math.atan`, from
/// fdlibm as in V8.
pub(crate) fn sin(x: f64) -> f64 {
    libm::sin(x)
}
pub(crate) fn cos(x: f64) -> f64 {
    libm::cos(x)
}
pub(crate) fn tan(x: f64) -> f64 {
    libm::tan(x)
}
pub(crate) fn asin(x: f64) -> f64 {
    libm::asin(x)
}
pub(crate) fn atan(x: f64) -> f64 {
    libm::atan(x)
}

/// `Math.pow(x, 2)`.
pub(crate) fn sq(x: f64) -> f64 {
    x.powi(2)
}

/// `Math.min` and `Math.max` for two numbers, which propagate `NaN`.
pub(crate) fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}
pub(crate) fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b || (a == b && b.is_sign_negative()) {
        a
    } else {
        b
    }
}

thread_local! {
    static MATH_RANDOM: RefCell<Box<dyn FnMut() -> f64>> = RefCell::new(Box::new(default_random()));
}

fn default_random() -> impl FnMut() -> f64 {
    use std::hash::{BuildHasher, RandomState};
    let mut state = RandomState::new().hash_one(0u64) | 1;
    move || {
        // xorshift64*; like Math.random, not reproducible between runs.
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let bits = state.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11;
        bits as f64 / (1u64 << 53) as f64
    }
}

/// `Math.random`. rough.js calls it when a shape has no seed, and the
/// `dots` filler calls it even when one is set.
pub fn math_random() -> f64 {
    MATH_RANDOM.with(|random| (random.borrow_mut())())
}

/// Replaces `Math.random` on this thread, so that tests can feed the port
/// the same sequence the original was given.
pub fn set_math_random(random: impl FnMut() -> f64 + 'static) {
    MATH_RANDOM.with(|slot| *slot.borrow_mut() = Box::new(random));
}

/// `Number.prototype.toString()` with radix 10.
pub fn number_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let sign = if x < 0.0 { "-" } else { "" };
    // Rust's `{:e}` prints the shortest digits that round-trip, which is the
    // digit string ECMAScript's Number::toString chooses.
    let exp_form = format!("{:e}", x.abs());
    let (mantissa, exponent) = exp_form.split_once('e').expect("exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent.parse::<i32>().expect("exponent") + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let e_sign = if e < 0 { '-' } else { '+' };
        if k == 1 {
            format!("{digits}e{e_sign}{}", e.abs())
        } else {
            format!("{}.{}e{e_sign}{}", &digits[..1], &digits[1..], e.abs())
        }
    };
    format!("{sign}{body}")
}

/// `Number.prototype.toFixed(digits)`: exact decimal rounding, ties away
/// from zero.
pub fn to_fixed(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.abs() >= 1e21 {
        return number_to_string(x);
    }
    // A double has at most 1074 fractional decimal digits, so this prints the
    // value exactly.
    let exact = format!("{:.1100}", x.abs());
    let (int_part, frac_part) = exact.split_once('.').expect("decimal point");
    let mut kept: Vec<u8> = int_part
        .bytes()
        .chain(frac_part.bytes().take(digits))
        .collect();
    if frac_part.as_bytes()[digits] >= b'5' {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    let int_len = kept.len() - digits;
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(&kept[..int_len]).expect("ascii"));
    if digits > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(&kept[int_len..]).expect("ascii"));
    }
    out
}

/// `parseFloat` of a string this crate produced (a plain decimal).
pub(crate) fn parse_float(s: &str) -> f64 {
    s.parse().unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_to_string_follows_ecmascript() {
        for (x, s) in [
            (1.0, "1"),
            (-1.5, "-1.5"),
            (0.1, "0.1"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e21, "1e+21"),
            (1.5e-7, "1.5e-7"),
            (0.000001, "0.000001"),
            (-0.0, "0"),
            (100.0, "100"),
        ] {
            assert_eq!(number_to_string(x), s, "{x}");
        }
    }

    #[test]
    fn to_fixed_rounds_ties_away_from_zero() {
        assert_eq!(to_fixed(0.0009765625, 9), "0.000976563");
        assert_eq!(to_fixed(-0.0009765625, 9), "-0.000976563");
        assert_eq!(to_fixed(0.25, 1), "0.3");
        assert_eq!(to_fixed(0.35, 1), "0.3"); // 0.35 is below the tie
        assert_eq!(to_fixed(9.995, 2), "9.99"); // likewise
        assert_eq!(to_fixed(99.5, 0), "100");
        assert_eq!(to_fixed(-0.0, 2), "0.00");
    }

    #[test]
    fn round_matches_math_round() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert!(round(-0.4).is_sign_negative());
        assert_eq!(round(0.49999999999999994), 0.0);
    }

    #[test]
    fn to_int32_wraps() {
        assert_eq!(to_int32(2147483648.0), -2147483648);
        assert_eq!(to_int32(4294967297.0), 1);
        assert_eq!(to_int32(-1.5), -1);
    }
}
