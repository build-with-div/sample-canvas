//! Compares the port's `RoughCanvas` with rough.js 4.6.6's.
//!
//! `fixtures/canvas-cases.json` lists `RoughCanvas` calls.
//! `fixtures/rough-4.6.6-canvas.json` holds every access rough.js 4.6.6
//! made to its 2D context for each call, recorded under Node by
//! `tools/rough-oracle/oracle.cjs` with a Proxy for the context. This test
//! makes the same calls on the port with a `CallRecorder` and compares the
//! traces entry by entry: names and strings exactly, numbers under the same
//! rule as `oracle.rs` (identical, or within `MAX_ULPS`).
//!
//! The recorder is `// div-gap: W2` scaffolding; it draws nothing. This
//! test establishes that the port asks a 2D context for what rough.js asks
//! for, not what any context renders.

mod common;

use common::{mulberry32, options, points, Tally};
use roughjs::js::set_math_random;
use roughjs::{CallRecorder, CanvasCall, RoughCanvas};
use serde_json::Value;

/// Runs one call; `Err` stands for a thrown error.
fn run(rc: &mut RoughCanvas<CallRecorder>, call: &Value) -> Result<(), String> {
    let args = call["args"].as_array().unwrap();
    let n = |i: usize| args[i].as_f64().unwrap();
    let o = options(&call["options"]);
    let o = o.as_ref();
    match call["method"].as_str().unwrap() {
        "line" => drop(rc.line(n(0), n(1), n(2), n(3), o)),
        "rectangle" => drop(rc.rectangle(n(0), n(1), n(2), n(3), o)),
        "ellipse" => drop(rc.ellipse(n(0), n(1), n(2), n(3), o)),
        "circle" => drop(rc.circle(n(0), n(1), n(2), o)),
        "linearPath" => drop(rc.linear_path(&points(&args[0]), o)),
        "polygon" => drop(rc.polygon(&points(&args[0]), o)),
        "arc" => drop(rc.arc(
            n(0),
            n(1),
            n(2),
            n(3),
            n(4),
            n(5),
            args[6].as_bool().unwrap(),
            o,
        )),
        "curve" => drop(rc.curve(&points(&args[0]), o)),
        "path" => drop(
            rc.path(args[0].as_str().unwrap(), o)
                .map_err(|e| format!("{e:?}"))?,
        ),
        other => panic!("unknown method {other}"),
    }
    Ok(())
}

enum Arg<'a> {
    Num(f64),
    Str(&'a str),
    Nums(&'a [f64]),
}

/// A recorded call in the oracle's form: `[name, ...args]`.
fn web_form(call: &CanvasCall) -> (&'static str, Vec<Arg<'_>>) {
    use Arg::*;
    match call {
        CanvasCall::Save => ("save", vec![]),
        CanvasCall::Restore => ("restore", vec![]),
        CanvasCall::SetStrokeStyle(s) => ("strokeStyle=", vec![Str(s)]),
        CanvasCall::SetFillStyle(s) => ("fillStyle=", vec![Str(s)]),
        CanvasCall::SetLineWidth(v) => ("lineWidth=", vec![Num(*v)]),
        CanvasCall::SetLineDash(v) => ("setLineDash", vec![Nums(v)]),
        CanvasCall::SetLineDashOffset(v) => ("lineDashOffset=", vec![Num(*v)]),
        CanvasCall::BeginPath => ("beginPath", vec![]),
        CanvasCall::MoveTo(x, y) => ("moveTo", vec![Num(*x), Num(*y)]),
        CanvasCall::LineTo(x, y) => ("lineTo", vec![Num(*x), Num(*y)]),
        CanvasCall::BezierCurveTo(d) => ("bezierCurveTo", d.iter().map(|v| Num(*v)).collect()),
        CanvasCall::Fill(rule) => ("fill", vec![Str(rule.as_str())]),
        CanvasCall::Stroke => ("stroke", vec![]),
    }
}

fn compare(ctx: &str, got: &[CanvasCall], want: &[Value], tally: &mut Tally) {
    tally.structure(
        ctx,
        got.len() == want.len(),
        format!("{} context calls vs {}", got.len(), want.len()),
    );
    for (i, (call, entry)) in got.iter().zip(want).enumerate() {
        let ctx = format!("{ctx} entry {i}");
        let entry = entry.as_array().unwrap();
        let (name, args) = web_form(call);
        tally.structure(&ctx, entry[0] == name, format!("{name} vs {}", entry[0]));
        let want_args = &entry[1..];
        tally.structure(
            &ctx,
            args.len() == want_args.len(),
            format!("{name}: {} arguments vs {}", args.len(), want_args.len()),
        );
        for (j, (arg, w)) in args.iter().zip(want_args).enumerate() {
            let ctx = format!("{ctx} {name} argument {j}");
            match arg {
                Arg::Num(v) => tally.number(&ctx, *v, w),
                Arg::Str(s) => tally.structure(&ctx, *w == **s, format!("{s:?} vs {w}")),
                Arg::Nums(v) => {
                    let w = w.as_array().unwrap();
                    tally.structure(&ctx, v.len() == w.len(), "array length".into());
                    for (x, wx) in v.iter().zip(w) {
                        tally.number(&ctx, *x, wx);
                    }
                }
            }
        }
    }
}

#[test]
fn canvas_matches_rough_js_4_6_6() {
    let cases: Value = serde_json::from_str(include_str!("fixtures/canvas-cases.json")).unwrap();
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/rough-4.6.6-canvas.json")).unwrap();
    assert_eq!(oracle["roughjs"], "4.6.6");
    let mut tally = Tally::default();
    let (mut calls, mut entries) = (0, 0);
    for (group, results) in cases["groups"]
        .as_array()
        .unwrap()
        .iter()
        .zip(oracle["results"].as_array().unwrap())
    {
        let name = group["name"].as_str().unwrap();
        // rough.canvas(canvas) asks for exactly one context, a '2d' one.
        assert_eq!(results["contextKinds"], serde_json::json!(["2d"]), "{name}");
        set_math_random(mulberry32(group["mathRandomSeed"].as_i64().unwrap() as i32));
        let config = options(&group["config"]["options"]);
        let mut rc = RoughCanvas::new(CallRecorder::default(), config.as_ref());
        for (i, (call, want)) in group["calls"]
            .as_array()
            .unwrap()
            .iter()
            .zip(results["calls"].as_array().unwrap())
            .enumerate()
        {
            calls += 1;
            let ctx = format!("{name} #{i} {}", call["method"]);
            rc.context_mut().calls.clear();
            let result = run(&mut rc, call);
            let trace = want["trace"].as_array().unwrap();
            entries += trace.len();
            match (&result, want.get("error")) {
                (Ok(()), None) | (Err(_), Some(_)) => {}
                (Ok(()), Some(e)) => {
                    tally.structure(&ctx, false, format!("rough.js threw {e}, the port did not"))
                }
                (Err(e), None) => tally.structure(
                    &ctx,
                    false,
                    format!("the port failed ({e}), rough.js did not"),
                ),
            }
            compare(&ctx, &rc.context().calls, trace, &mut tally);
        }
    }
    println!(
        "rough.js 4.6.6 canvas oracle: {calls} calls, {entries} context accesses; {} of {} numbers identical (max |diff| {:e}, max {} ulps)",
        tally.identical, tally.numbers, tally.max_abs_diff, tally.max_ulps
    );
    tally.assert_gate();
}
