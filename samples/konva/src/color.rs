//! Konva's color functions (`Util.ts`): `colorToRGBA`, which parses CSS
//! color strings for shadows, and the random hit colors that key each
//! shape in the hit canvas.

use crate::colors::COLORS;
use crate::js;
use crate::transform::DEG180_OVER_PI;
use std::cell::RefCell;
use std::collections::HashSet;

/// What `Util.colorToRGBA` returns. The channels are numbers, not bytes:
/// `rgb(10.5, 0, 0)` keeps its half.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

/// `Util.colorToRGBA(str)`: `None` for a color Konva can't parse.
pub fn color_to_rgba(s: Option<&str>) -> Option<Rgba> {
    let s = js::trim(s.unwrap_or(""));
    let s = if s.is_empty() { "black" } else { s };
    let color = named(s)
        .or_else(|| hex(s, 4))
        .or_else(|| hex(s, 5))
        .or_else(|| hex(s, 7))
        .or_else(|| hex(s, 9))
        .or_else(|| rgb(s))
        .or_else(|| hsl(s))?;
    // `Util._isNumber`: finite and not NaN.
    [color.r, color.g, color.b, color.a]
        .iter()
        .all(|v| v.is_finite())
        .then_some(color)
}

fn named(s: &str) -> Option<Rgba> {
    let lower = s.to_lowercase();
    let i = COLORS
        .binary_search_by(|(name, _, _)| name.cmp(&lower.as_str()))
        .ok()?;
    let (_, [r, g, b], a) = COLORS[i];
    Some(Rgba {
        r: r.into(),
        g: g.into(),
        b: b.into(),
        a: a.unwrap_or(1.0),
    })
}

/// `#rgb`, `#rgba`, `#rrggbb` and `#rrggbbaa`, by string length as Konva
/// tells them apart.
fn hex(s: &str, len: usize) -> Option<Rgba> {
    let digits = s.strip_prefix('#')?;
    if s.len() != len || digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |hex: &str| u8::from_str_radix(hex, 16).unwrap() as f64;
    let short = |i: usize| byte(&digits[i..=i].repeat(2));
    let long = |i: usize| byte(&digits[i * 2..i * 2 + 2]);
    Some(match len {
        4 => Rgba {
            r: short(0),
            g: short(1),
            b: short(2),
            a: 1.0,
        },
        5 => Rgba {
            r: short(0),
            g: short(1),
            b: short(2),
            a: short(3) / 255.0,
        },
        7 => Rgba {
            r: long(0),
            g: long(1),
            b: long(2),
            a: 1.0,
        },
        _ => Rgba {
            r: long(0),
            g: long(1),
            b: long(2),
            a: long(3) / 255.0,
        },
    })
}

/// The inside of `name(…)`, matched case-insensitively as
/// `/^names?\(([^)]*)\)$/i`.
fn function_args<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let open = s.find('(')?;
    let fname = s[..open].to_ascii_lowercase();
    if fname != name && fname != format!("{name}a") {
        return None;
    }
    let inner = s[open + 1..].strip_suffix(')')?;
    (!inner.contains(')')).then_some(inner)
}

/// `splitColorComponents`: commas (legacy syntax), or spaces and a slash
/// before the alpha, never both.
fn split_components(s: &str) -> Vec<&str> {
    let s = js::trim(s);
    if s.contains(',') {
        // `/\s*,\s*/` on a trimmed string.
        return s.split(',').map(js::trim).collect();
    }
    // `/\s*\/\s*|\s+/`.
    let mut parts = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (pos, c) = chars[i];
        if js::is_space(c) || c == '/' {
            let mut j = i;
            while j < chars.len() && js::is_space(chars[j].1) {
                j += 1;
            }
            // A slash with the space around it, or else a run of space.
            if j < chars.len() && chars[j].1 == '/' {
                j += 1;
                while j < chars.len() && js::is_space(chars[j].1) {
                    j += 1;
                }
            }
            parts.push(&s[start..pos]);
            start = chars.get(j).map_or(s.len(), |(p, _)| *p);
            i = j;
        } else {
            i += 1;
        }
    }
    parts.push(&s[start..]);
    parts
}

/// The length of a CSS number at the start of `s`
/// (`[+-]?(?:\d+\.?\d*|\.\d+)(?:e[+-]?\d+)?`), if one is there.
fn number_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let int_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i > int_start {
        if i < b.len() && b[i] == b'.' {
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
    } else {
        if i >= b.len() || b[i] != b'.' {
            return None;
        }
        i += 1;
        let frac = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == frac {
            return None;
        }
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let exp = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp {
            i = j;
        }
    }
    Some(i)
}

/// `Number(str)` for a string `number_len` accepted.
fn number(s: &str) -> f64 {
    let s = s.strip_prefix('+').unwrap_or(s);
    s.parse().unwrap()
}

/// `parseColorComponent(value, max)`: a number or a percentage of `max`,
/// clamped to `[0, max]`; NaN if it is neither.
fn component(value: &str, max: f64) -> f64 {
    let Some(len) = number_len(value) else {
        return f64::NAN;
    };
    let rest = &value[len..];
    let n = match rest {
        "" => number(&value[..len]),
        "%" => (number(&value[..len]) / 100.0) * max,
        _ => return f64::NAN,
    };
    js::min(&[js::max(&[n, 0.0]), max])
}

fn rgb(s: &str) -> Option<Rgba> {
    let parts = split_components(function_args(s, "rgb")?);
    if parts.len() < 3 || parts.len() > 4 {
        return None;
    }
    Some(Rgba {
        r: component(parts[0], 255.0),
        g: component(parts[1], 255.0),
        b: component(parts[2], 255.0),
        a: if parts.len() > 3 {
            component(parts[3], 1.0)
        } else {
            1.0
        },
    })
}

fn hsl(s: &str) -> Option<Rgba> {
    let parts = split_components(function_args(s, "hsl")?);
    if parts.len() < 3 || parts.len() > 4 {
        return None;
    }
    let hue = parts[0];
    let len = number_len(hue)?;
    let unit = match hue[len..].to_ascii_lowercase().as_str() {
        "" | "deg" => 1.0,
        "grad" => 0.9,
        "rad" => DEG180_OVER_PI,
        "turn" => 360.0,
        _ => return None,
    };
    let degrees = number(&hue[..len]) * unit;
    if !degrees.is_finite() {
        return None;
    }
    let h = (((degrees % 360.0) + 360.0) % 360.0) / 360.0;
    let s = component(parts[1], 100.0) / 100.0;
    let l = component(parts[2], 100.0) / 100.0;
    let a = if parts.len() > 3 {
        component(parts[3], 1.0)
    } else {
        1.0
    };
    let t2 = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let t1 = 2.0 * l - t2;
    let mut rgb = [0.0; 3];
    for (i, out) in rgb.iter_mut().enumerate() {
        let mut t3 = h + (1.0 / 3.0) * -(i as f64 - 1.0);
        if t3 < 0.0 {
            t3 += 1.0;
        }
        if t3 > 1.0 {
            t3 -= 1.0;
        }
        let val = if 6.0 * t3 < 1.0 {
            t1 + (t2 - t1) * 6.0 * t3
        } else if 2.0 * t3 < 1.0 {
            t2
        } else if 3.0 * t3 < 2.0 {
            t1 + (t2 - t1) * (2.0 / 3.0 - t3) * 6.0
        } else {
            t1
        };
        *out = val * 255.0;
    }
    Some(Rgba {
        r: js::round(rgb[0]),
        g: js::round(rgb[1]),
        b: js::round(rgb[2]),
        a,
    })
}

// ---- Hit colors ----

type Random = Box<dyn FnMut() -> f64>;

thread_local! {
    static MATH_RANDOM: RefCell<Random> = RefCell::new(Box::new(mulberry32(0)));
    /// Konva's `shapes` registry: the color keys in use.
    static KEYS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// A seeded generator with `Math.random`'s range, for the default.
fn mulberry32(mut seed: u32) -> impl FnMut() -> f64 {
    move || {
        seed = seed.wrapping_add(0x6d2b_79f5);
        let mut t = (seed ^ (seed >> 15)).wrapping_mul(1 | seed);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

/// Replaces the `Math.random` that hit colors come from, so a scene's keys
/// can be reproduced.
pub fn set_math_random(f: impl FnMut() -> f64 + 'static) {
    MATH_RANDOM.with(|r| *r.borrow_mut() = Box::new(f));
}

fn math_random() -> f64 {
    MATH_RANDOM.with(|r| (r.borrow_mut())())
}

/// Empties Konva's registry of shape color keys, as a new page would.
pub fn clear_color_keys() {
    KEYS.with(|k| k.borrow_mut().clear());
}

fn rgb_to_hex(r: i64, g: i64, b: i64) -> String {
    format!("{:06x}", (r << 16) + (g << 8) + b)
}

/// `Util.getRandomColor()`.
fn random_color() -> String {
    let v = (math_random() * 16_777_215.0) as i32;
    format!("#{v:06x}")
}

/// `Util.getHitColorKey(r, g, b)`: channels snapped to a grid of 3, so a
/// hit pixel read back slightly off still finds its shape.
// div-gap: W7 getImageData. Konva first checks for canvas farbling by
// reading pixels back, and snaps to 5 if it finds it; with no readback the
// port takes the unfarbled grid.
fn hit_color_key(r: f64, g: f64, b: f64) -> String {
    let snap = |v: f64| (js::round(v / 3.0) * 3.0) as i64;
    format!("#{}", rgb_to_hex(snap(r), snap(g), snap(b)))
}

/// A new shape's color key, as Konva's `Shape` constructor picks it: a
/// random hit color not in use.
pub(crate) fn new_color_key() -> String {
    let mut attempts = 0;
    let key = loop {
        let channel = || (math_random() * 256.0) as i32 as f64;
        let (r, g, b) = (channel(), channel(), channel());
        let key = hit_color_key(r, g, b);
        if !KEYS.with(|k| k.borrow().contains(&key)) {
            break key;
        }
        attempts += 1;
        if attempts >= 10_000 {
            break random_color();
        }
    };
    KEYS.with(|k| k.borrow_mut().insert(key.clone()));
    key
}

pub(crate) fn release_color_key(key: &str) {
    KEYS.with(|k| k.borrow_mut().remove(key));
}
