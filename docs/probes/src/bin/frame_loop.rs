//! Native probe: drive a canvas animation from Div's only per-frame signal.
//!
//! div-gap: W11 (`requestAnimationFrame`). Div has none yet. `App::on_platform_event` sees
//! `PlatformEvent::Presented` after each present; pushing a new canvas image
//! there makes the document dirty, which schedules the next frame. This runs
//! for `SECONDS` (default 3), then prints the frame rate and intervals.
//!
//!     cargo run --release --bin frame_loop -- [seconds] [auto|software|opengl]

use canvas_probes::{raster_surface, SurfaceBridge};
use div::prelude::*;
use div::PlatformEvent;
use skia_safe::{Color as SkColor, Paint};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let seconds: f64 = args.next().map(|s| s.parse()).transpose()?.unwrap_or(3.0);
    let backend = match args.next().as_deref() {
        Some("software") => BackendPreference::Software,
        Some("opengl") => BackendPreference::OpenGl,
        _ => BackendPreference::Auto,
    };
    let (cw, ch) = (640, 480);
    let mut app = App::builder()
        .title("canvas frame loop probe")
        .size(LogicalSize::new(800.0, 600.0))
        .backend(backend)
        .build()?;
    app.mount(|| view! { <canvas id="c"></canvas> })?;
    let document = app.document().clone();
    let canvas = document.element_by_id("c")?.expect("canvas");
    canvas.set_width(LengthValue::px(cw as f32))?;
    canvas.set_height(LengthValue::px(ch as f32))?;

    let exit = app.exit_handle();
    let mut surface = raster_surface(cw, ch);
    let mut bridge = SurfaceBridge::new(&document, &canvas, (cw as f32, ch as f32), "loop");
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let mut started: Option<Instant> = None;
    let mut last = None::<f64>;
    let mut intervals = Vec::<f64>::new();
    let mut push_ms = 0.0;
    app.on_platform_event(move |event| match event {
        PlatformEvent::BackendChanged(status) => println!("backend: {status:?}"),
        PlatformEvent::Resized(viewport) => println!("viewport: {viewport:?}"),
        PlatformEvent::Presented {
            frame_number,
            time_ms,
        } => {
            let begin = *started.get_or_insert_with(Instant::now);
            if let Some(previous) = last {
                intervals.push(time_ms - previous);
            }
            last = Some(*time_ms);
            if begin.elapsed().as_secs_f64() >= seconds {
                let n = intervals.len().max(1) as f64;
                let mean = intervals.iter().sum::<f64>() / n;
                let mut sorted = intervals.clone();
                sorted.sort_by(f64::total_cmp);
                let p = |q: f64| sorted.get(((sorted.len() as f64 - 1.0) * q) as usize);
                println!(
                    "presented {} frames ({} canvas pushes) in {:.2} s: {:.1} fps, \
                     interval mean {:.2} ms, p50 {:?} ms, p95 {:?} ms, push {:.2} ms/frame",
                    frame_number,
                    bridge.frames(),
                    begin.elapsed().as_secs_f64(),
                    intervals.len() as f64 / begin.elapsed().as_secs_f64(),
                    mean,
                    p(0.5),
                    p(0.95),
                    push_ms / bridge.frames().max(1) as f64
                );
                exit.request_exit();
                return;
            }
            let c = surface.canvas();
            c.clear(SkColor::from_rgb(20, 20, 30));
            for i in 0..200u64 {
                let t = (frame_number + i) as f32 * 0.05;
                paint.set_color(SkColor::from_rgb((i * 7 % 255) as u8, 160, 220));
                c.draw_circle(
                    (320.0 + 250.0 * t.cos(), 240.0 + 180.0 * (t * 1.3).sin()),
                    6.0,
                    &paint,
                );
            }
            let push = Instant::now();
            bridge.present(&mut surface).expect("push a frame");
            push_ms += push.elapsed().as_secs_f64() * 1000.0;
        }
        _ => {}
    });
    app.run_document()?;
    Ok(())
}
