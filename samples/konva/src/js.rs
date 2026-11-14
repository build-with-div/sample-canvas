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

/// `Math.round`: halves round up, and the result keeps the sign of zero.
pub fn round(v: f64) -> f64 {
    if !v.is_finite() {
        return v;
    }
    let f = v.floor();
    let r = if v - f >= 0.5 { f + 1.0 } else { f };
    if r == 0.0 && (v < 0.0 || v.is_sign_negative()) {
        -0.0
    } else {
        r
    }
}

/// `Number.prototype.toString()`: the shortest digits that round-trip,
/// laid out as ECMAScript's Number::toString lays them out.
pub fn number_to_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v == 0.0 {
        return "0".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if v < 0.0 {
        return format!("-{}", number_to_string(-v));
    }
    // `{:e}` gives the shortest round-trip digits as `d.ddde±x`.
    let sci = format!("{v:e}");
    let (mantissa, exp) = sci.split_once('e').unwrap();
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let exp = exp.parse::<i32>().unwrap();
    let digits = nearest_even(v, digits, exp);
    let k = digits.len() as i32;
    let n = exp + 1;
    if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let sign = if e < 0 { '-' } else { '+' };
        let rest = if k > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        format!("{}{rest}e{sign}{}", &digits[..1], e.abs())
    }
}

/// Number::toString picks, of the shortest digit strings that round-trip,
/// the one nearest the value, and the even one of two equally near. Rust's
/// shortest form can take the odd one: 1791573930531740.25 prints as
/// ...740.3 where V8 prints ...740.2.
fn nearest_even(v: f64, digits: String, exp: i32) -> String {
    // The exact value: no double has more than 767 significant digits.
    let exact = format!("{v:.800e}");
    let (mantissa, exact_exp) = exact.split_once('e').unwrap();
    if exact_exp.parse::<i32>().unwrap() != exp {
        return digits;
    }
    let all: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len();
    let rest = &all[k..];
    // A tie: the dropped digits are exactly one half.
    if !(rest.starts_with('5') && rest[1..].bytes().all(|b| b == b'0')) {
        return digits;
    }
    let lower = &all[..k];
    let last = lower.as_bytes()[k - 1] - b'0';
    let pick = if last.is_multiple_of(2) {
        lower.to_string()
    } else if last < 9 {
        format!("{}{}", &lower[..k - 1], last + 1)
    } else {
        return digits;
    };
    // Only if it still reads back as the same number.
    let candidate = format!("{}.{}e{exp}", &pick[..1], &pick[1..]);
    if candidate.parse::<f64>() == Ok(v) {
        pick
    } else {
        digits
    }
}

/// JavaScript's white space and line terminators, as `\s` and `trim()`
/// match them.
pub fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `String.prototype.trim()`.
pub fn trim(s: &str) -> &str {
    s.trim_matches(is_space)
}
