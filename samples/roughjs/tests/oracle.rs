//! Compares the port with rough.js 4.6.6 itself.
//!
//! `fixtures/cases.json` lists generator calls. `fixtures/rough-4.6.6.json`
//! holds what rough.js 4.6.6 returned for them, recorded under Node by
//! `tools/rough-oracle/oracle.cjs`. This test makes the same calls on the
//! port and compares every operation, every coordinate and the SVG path
//! strings from `toPaths` and `opsToPath`.
//!
//! Both sides replace `Math.random` with mulberry32 from the case group's
//! seed, so unseeded shapes and the `dots` fill are comparable too.
//!
//! The gate: every call must produce the same shapes, sets, op sequence and
//! data lengths, and the same strings apart from the digits of numbers.
//! Numbers must be identical or within `MAX_ULPS`. The tolerance exists
//! for `Math.sin` and `Math.cos` only. The port uses fdlibm, as `libm`
//! provides, which matches Node 22's V8 bit for bit for `tan`, `asin` and
//! `atan` on 200,000 inputs. For `sin` and `cos` about 0.9% of results
//! differ by one bit. That is consistent with V8 building with its
//! glibc-derived `sin` and `cos` (`v8_use_libm_trig_functions`). See
//! docs/phase2-plan.md.

use roughjs::js::set_math_random;
use roughjs::{Drawable, Options, Point, RoughGenerator};
use serde_json::Value;

/// mulberry32, as `oracle.cjs` defines it.
fn mulberry32(seed: i32) -> impl FnMut() -> f64 {
    let mut a = seed;
    move || {
        a = a.wrapping_add(0x6d2b_79f5);
        let mut t = (a ^ ((a as u32 >> 15) as i32)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ ((t as u32 >> 7) as i32)).wrapping_mul(61 | t)) ^ t;
        f64::from((t ^ ((t as u32 >> 14) as i32)) as u32) / 4_294_967_296.0
    }
}

fn options(value: &Value) -> Option<Options> {
    let map = value.as_object()?;
    let mut o = Options::default();
    for (key, v) in map {
        let num = || v.as_f64().expect("number");
        let text = || v.as_str().expect("string").to_string();
        let flag = || v.as_bool().expect("bool");
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
            other => panic!("cases.json uses option {other}, which this test does not map"),
        }
    }
    Some(o)
}

fn point(v: &Value) -> Point {
    [v[0].as_f64().unwrap(), v[1].as_f64().unwrap()]
}

fn points(v: &Value) -> Vec<Point> {
    v.as_array().unwrap().iter().map(point).collect()
}

/// Runs one call; `Err` stands for a thrown error.
fn run(generator: &mut RoughGenerator, call: &Value) -> Result<Drawable, String> {
    let args = call["args"].as_array().unwrap();
    let n = |i: usize| args[i].as_f64().unwrap();
    let o = options(&call["options"]);
    let o = o.as_ref();
    Ok(match call["method"].as_str().unwrap() {
        "line" => generator.line(n(0), n(1), n(2), n(3), o),
        "rectangle" => generator.rectangle(n(0), n(1), n(2), n(3), o),
        "ellipse" => generator.ellipse(n(0), n(1), n(2), n(3), o),
        "circle" => generator.circle(n(0), n(1), n(2), o),
        "linearPath" => generator.linear_path(&points(&args[0]), o),
        "polygon" => generator.polygon(&points(&args[0]), o),
        "arc" => generator.arc(
            n(0),
            n(1),
            n(2),
            n(3),
            n(4),
            n(5),
            args[6].as_bool().unwrap(),
            o,
        ),
        "curve" => {
            let list = args[0].as_array().unwrap();
            // `curve` takes `Point[]` or `Point[][]`, told apart by
            // whether the first element's first element is a number.
            if list.first().is_some_and(|p| p[0].is_number()) {
                generator.curve(&points(&args[0]), o)
            } else {
                generator.curves(&list.iter().map(points).collect::<Vec<_>>(), o)
            }
        }
        "path" => generator
            .path(args[0].as_str().unwrap(), o)
            .map_err(|e| format!("{e:?}"))?,
        other => panic!("unknown method {other}"),
    })
}

/// The largest difference allowed, in units in the last place.
const MAX_ULPS: u64 = 2;

#[derive(Default)]
struct Tally {
    numbers: usize,
    identical: usize,
    max_abs_diff: f64,
    max_ulps: u64,
    strings: usize,
    strings_identical: usize,
    mismatches: Vec<String>,
    structural: Vec<String>,
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
    fn number(&mut self, ctx: &str, got: f64, want: &Value) {
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
    fn string(&mut self, ctx: &str, got: &str, want: &Value) {
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

    fn structure(&mut self, ctx: &str, ok: bool, what: String) {
        if !ok {
            self.structural.push(format!("{ctx}: {what}"));
        }
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

fn compare(generator: &RoughGenerator, ctx: &str, got: &Drawable, want: &Value, tally: &mut Tally) {
    tally.structure(
        ctx,
        got.shape == want["shape"],
        format!("shape {} vs {}", got.shape, want["shape"]),
    );
    let sets = want["sets"].as_array().unwrap();
    tally.structure(
        ctx,
        got.sets.len() == sets.len(),
        format!("{} sets vs {}", got.sets.len(), sets.len()),
    );
    for (s, (set, want_set)) in got.sets.iter().zip(sets).enumerate() {
        let ctx = format!("{ctx} set {s}");
        tally.structure(
            &ctx,
            set.kind.as_str() == want_set["type"],
            format!("type {:?}", set.kind),
        );
        let ops = want_set["ops"].as_array().unwrap();
        tally.structure(
            &ctx,
            set.ops.len() == ops.len(),
            format!("{} ops vs {}", set.ops.len(), ops.len()),
        );
        for (i, (op, want_op)) in set.ops.iter().zip(ops).enumerate() {
            let ctx = format!("{ctx} op {i}");
            tally.structure(
                &ctx,
                op.op.as_str() == want_op["op"],
                format!("op {:?} vs {}", op.op, want_op["op"]),
            );
            let data = want_op["data"].as_array().unwrap();
            tally.structure(&ctx, op.data.len() == data.len(), "data length".into());
            for (j, (x, w)) in op.data.iter().zip(data).enumerate() {
                tally.number(&format!("{ctx} data {j}"), *x, w);
            }
        }
    }
    let paths = generator.to_paths(got);
    let want_paths = want["paths"].as_array().unwrap();
    tally.structure(
        ctx,
        paths.len() == want_paths.len(),
        "toPaths length".into(),
    );
    for (i, (p, w)) in paths.iter().zip(want_paths).enumerate() {
        let ctx = format!("{ctx} toPaths {i}");
        tally.string(&format!("{ctx} d"), &p.d, &w["d"]);
        tally.string(&format!("{ctx} stroke"), &p.stroke, &w["stroke"]);
        tally.string(&format!("{ctx} fill"), &p.fill, &w["fill"]);
        tally.number(
            &format!("{ctx} strokeWidth"),
            p.stroke_width,
            &w["strokeWidth"],
        );
    }
    for (i, (set, w)) in got
        .sets
        .iter()
        .zip(want["fixed2"].as_array().unwrap())
        .enumerate()
    {
        tally.string(
            &format!("{ctx} opsToPath(set {i}, 2)"),
            &generator.ops_to_path(set, Some(2.0)),
            w,
        );
    }
}

#[test]
fn port_matches_rough_js_4_6_6() {
    let cases: Value = serde_json::from_str(include_str!("fixtures/cases.json")).unwrap();
    let oracle: Value = serde_json::from_str(include_str!("fixtures/rough-4.6.6.json")).unwrap();
    assert_eq!(oracle["roughjs"], "4.6.6");
    let mut tally = Tally::default();
    let mut calls = 0;
    for (group, results) in cases["groups"]
        .as_array()
        .unwrap()
        .iter()
        .zip(oracle["results"].as_array().unwrap())
    {
        let name = group["name"].as_str().unwrap();
        set_math_random(mulberry32(group["mathRandomSeed"].as_i64().unwrap() as i32));
        let config = options(&group["config"]["options"]);
        let mut generator = RoughGenerator::new(config.as_ref());
        for (i, (call, want)) in group["calls"]
            .as_array()
            .unwrap()
            .iter()
            .zip(results["calls"].as_array().unwrap())
            .enumerate()
        {
            calls += 1;
            let ctx = format!("{name} #{i} {}", call["method"]);
            match (run(&mut generator, call), want.get("error")) {
                (Ok(got), None) => compare(&generator, &ctx, &got, want, &mut tally),
                (Err(_), Some(_)) => {}
                (Ok(_), Some(e)) => {
                    tally.structure(&ctx, false, format!("rough.js threw {e}, the port did not"))
                }
                (Err(e), None) => tally.structure(
                    &ctx,
                    false,
                    format!("the port failed ({e}), rough.js did not"),
                ),
            }
        }
    }
    println!(
        "rough.js 4.6.6 oracle: {calls} calls; {} of {} numbers identical (max |diff| {:e}, max {} ulps); {} of {} strings identical",
        tally.identical, tally.numbers, tally.max_abs_diff, tally.max_ulps, tally.strings_identical, tally.strings
    );
    for m in tally.structural.iter().chain(&tally.mismatches) {
        println!("  {m}");
    }
    assert!(
        tally.structural.is_empty(),
        "the port's structure differs from rough.js 4.6.6"
    );
    assert!(
        tally.max_ulps <= MAX_ULPS,
        "a number differs by more than {MAX_ULPS} ulps"
    );
}
