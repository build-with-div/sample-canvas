//! Helpers shared by the oracle tests: the seeded `Math.random`, the
//! mapping from the cases' option objects and the comparison tally.

// Each test binary uses part of this module.
#![allow(dead_code)]

use roughjs::{Options, Point};
use serde_json::Value;

/// mulberry32, as `oracle.cjs` defines it.
pub fn mulberry32(seed: i32) -> impl FnMut() -> f64 {
    let mut a = seed;
    move || {
        a = a.wrapping_add(0x6d2b_79f5);
        let mut t = (a ^ ((a as u32 >> 15) as i32)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ ((t as u32 >> 7) as i32)).wrapping_mul(61 | t)) ^ t;
        f64::from((t ^ ((t as u32 >> 14) as i32)) as u32) / 4_294_967_296.0
    }
}

/// Maps a case's options object; `null` is `None`.
pub fn options(value: &Value) -> Option<Options> {
    let map = value.as_object()?;
    let mut o = Options::default();
    for (key, v) in map {
        let num = || v.as_f64().expect("number");
        let text = || v.as_str().expect("string").to_string();
        let flag = || v.as_bool().expect("bool");
        let nums = || {
            v.as_array()
                .expect("array")
                .iter()
                .map(|x| x.as_f64().expect("number"))
                .collect::<Vec<_>>()
        };
        match key.as_str() {
            "seed" => o.seed = Some(num()),
            "fill" => o.fill = Some(text()),
            "fillStyle" => o.fill_style = Some(text()),
            "stroke" => o.stroke = Some(text()),
            "roughness" => o.roughness = Some(num()),
            "bowing" => o.bowing = Some(num()),
            "maxRandomnessOffset" => o.max_randomness_offset = Some(num()),
            "strokeWidth" => o.stroke_width = Some(num()),
            "fillWeight" => o.fill_weight = Some(num()),
            "hachureAngle" => o.hachure_angle = Some(num()),
            "hachureGap" => o.hachure_gap = Some(num()),
            "dashOffset" => o.dash_offset = Some(num()),
            "dashGap" => o.dash_gap = Some(num()),
            "zigzagOffset" => o.zigzag_offset = Some(num()),
            "curveFitting" => o.curve_fitting = Some(num()),
            "curveStepCount" => o.curve_step_count = Some(num()),
            "curveTightness" => o.curve_tightness = Some(num()),
            "simplification" => o.simplification = Some(num()),
            "preserveVertices" => o.preserve_vertices = Some(flag()),
            "disableMultiStroke" => o.disable_multi_stroke = Some(flag()),
            "disableMultiStrokeFill" => o.disable_multi_stroke_fill = Some(flag()),
            "strokeLineDash" => o.stroke_line_dash = Some(nums()),
            "strokeLineDashOffset" => o.stroke_line_dash_offset = Some(num()),
            "fillLineDash" => o.fill_line_dash = Some(nums()),
            "fillLineDashOffset" => o.fill_line_dash_offset = Some(num()),
            "fixedDecimalPlaceDigits" => o.fixed_decimal_place_digits = Some(num()),
            other => panic!("the cases use option {other}, which the tests do not map"),
        }
    }
    Some(o)
}

pub fn point(v: &Value) -> Point {
    [v[0].as_f64().unwrap(), v[1].as_f64().unwrap()]
}

pub fn points(v: &Value) -> Vec<Point> {
    v.as_array().unwrap().iter().map(point).collect()
}

/// The largest difference allowed, in units in the last place.
pub const MAX_ULPS: u64 = 2;

#[derive(Default)]
pub struct Tally {
    pub numbers: usize,
    pub identical: usize,
    pub max_abs_diff: f64,
    pub max_ulps: u64,
    pub strings: usize,
    pub strings_identical: usize,
    pub mismatches: Vec<String>,
    pub structural: Vec<String>,
}

fn ulps(a: f64, b: f64) -> u64 {
    if a == b {
        return 0;
    }
    if a.is_nan() || b.is_nan() || a.signum() != b.signum() {
        return u64::MAX;
    }
    a.to_bits().abs_diff(b.to_bits())
}

impl Tally {
    pub fn number(&mut self, ctx: &str, got: f64, want: &Value) {
        // JSON.stringify writes NaN and infinities as null.
        let want = want.as_f64().unwrap_or(f64::NAN);
        self.numbers += 1;
        if got == want || (got.is_nan() && want.is_nan()) || (!got.is_finite() && want.is_nan()) {
            self.identical += 1;
            return;
        }
        self.differs(ctx, got, want);
    }

    fn differs(&mut self, ctx: &str, got: f64, want: f64) {
        let diff = (got - want).abs();
        let ulps = ulps(got, want);
        self.max_abs_diff = self.max_abs_diff.max(diff);
        self.max_ulps = self.max_ulps.max(ulps);
        if self.mismatches.len() < 5 {
            self.mismatches.push(format!(
                "{ctx}: port {got:?}, rough.js {want:?} ({ulps} ulps)"
            ));
        }
    }

    /// Compares strings token by token: text must match, and numbers are
    /// held to the same rule as other numbers.
    pub fn string(&mut self, ctx: &str, got: &str, want: &Value) {
        let want = want.as_str().unwrap();
        self.strings += 1;
        if got == want {
            self.strings_identical += 1;
            return;
        }
        let (got_tokens, want_tokens) = (tokens(got), tokens(want));
        if got_tokens.len() != want_tokens.len() {
            self.structure(
                ctx,
                false,
                format!("token counts differ: {got:?} vs {want:?}"),
            );
            return;
        }
        for (g, w) in got_tokens.iter().zip(&want_tokens) {
            match (g.parse::<f64>(), w.parse::<f64>()) {
                (Ok(a), Ok(b)) if a != b => self.differs(ctx, a, b),
                (Ok(_), Ok(_)) => {}
                _ if g != w => self.structure(ctx, false, format!("text differs: {g:?} vs {w:?}")),
                _ => {}
            }
        }
    }

    pub fn structure(&mut self, ctx: &str, ok: bool, what: String) {
        if !ok {
            self.structural.push(format!("{ctx}: {what}"));
        }
    }

    /// Prints the tally's mismatches and asserts the gate.
    pub fn assert_gate(&self) {
        for m in self.structural.iter().chain(&self.mismatches) {
            println!("  {m}");
        }
        assert!(
            self.structural.is_empty(),
            "the port's structure differs from rough.js 4.6.6"
        );
        assert!(
            self.max_ulps <= MAX_ULPS,
            "a number differs by more than {MAX_ULPS} ulps"
        );
    }
}

/// Splits a string into runs of number characters and runs of others.
fn tokens(s: &str) -> Vec<&str> {
    let is_num = |c: char| c.is_ascii_digit() || matches!(c, '.' | '-' | 'e' | '+');
    let mut out = Vec::new();
    let mut start = 0;
    let mut prev: Option<bool> = None;
    for (i, c) in s.char_indices() {
        let kind = is_num(c);
        if prev.is_some_and(|p| p != kind) {
            out.push(&s[start..i]);
            start = i;
        }
        prev = Some(kind);
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}
