//! What Div's public API allows for canvas today (Div 92ea25a0; first written at 565db46a).
//!
//! Each probe asserts the behavior observed, so a Div change that alters it
//! fails here and the finding in docs/design.md gets revisited. Run with
//! `cargo test --release -- --nocapture --test-threads 1` to see the
//! measurements; timings are only printed, never asserted.

use canvas_probes::{raster_surface, rgba8_resource, SurfaceBridge, RGBA8_MIME};
use div::prelude::*;
use skia_safe::{paint, Color as SkColor, Paint, PathEffect, Point, Shader, TileMode};
use std::time::Instant;

fn headless(width: f64, height: f64, scale: f64) -> HeadlessApp {
    HeadlessApp::new(ViewportMetrics::from_logical_size(width, height, scale).unwrap()).unwrap()
}

/// Premultiplied RGBA of the device pixel at (x, y).
fn pixel(bitmap: &Bitmap, x: u32, y: u32) -> [u8; 4] {
    let at = y as usize * bitmap.stride() + x as usize * 4;
    bitmap.pixels()[at..at + 4].try_into().unwrap()
}

fn canvas_at(document: &Document, left: f32, top: f32, width: f32, height: f32) -> Element {
    let canvas = Element::create(document, "canvas").unwrap();
    canvas.set_position(Position::Absolute).unwrap();
    canvas.set_left(Length::px(left)).unwrap();
    canvas.set_top(Length::px(top)).unwrap();
    canvas.set_width(LengthValue::px(width)).unwrap();
    canvas.set_height(LengthValue::px(height)).unwrap();
    canvas
}

/// P1. `<canvas>` is a known tag: `view!` accepts it and it paints nothing.
/// Its default size is not HTML's: with no CSS size it is 300 wide but 0
/// tall (Chromium: 300x150), whether made by `view!` or `Element::create`.
#[test]
fn p1_canvas_tag_is_transparent_and_zero_height_by_default() {
    let mut app = headless(400.0, 200.0, 1.0);
    app.mount(|| view! { <canvas id="c"></canvas> }).unwrap();
    let document = app.document();
    document.body().set_background_color(Color::WHITE).unwrap();
    let canvas = document.element_by_id("c").unwrap().unwrap();
    let rect = canvas.bounding_rect().unwrap().unwrap();
    let imperative = Element::create(document, "canvas").unwrap();
    document.body().append_child(&imperative).unwrap();
    let rect2 = imperative.bounding_rect().unwrap().unwrap();
    let block = Element::create(document, "canvas").unwrap();
    block.set_display(Display::Block).unwrap();
    document.body().append_child(&block).unwrap();
    let rect3 = block.bounding_rect().unwrap().unwrap();
    println!(
        "P1 default <canvas> rects: view! {rect:?}; created {rect2:?}; display:block {rect3:?}"
    );
    assert_eq!(
        (rect.width, rect.height),
        (300.0, 0.0),
        "div-gap: Chromium gives 300x150"
    );
    // With an explicit size it is a normal box that paints nothing.
    canvas.set_height(LengthValue::px(150.0)).unwrap();
    let rect = canvas.bounding_rect().unwrap().unwrap();
    assert_eq!((rect.width, rect.height), (300.0, 150.0));
    let bitmap = app.render_at(0.0).unwrap();
    let (x, y) = ((rect.x + 150.0) as u32, (rect.y + 75.0) as u32);
    assert_eq!(pixel(&bitmap, x, y), [255, 255, 255, 255]);
}

/// P2. An app can draw with its own skia-safe (Div's patched rev unifies)
/// and show the result in an element through `image/x-openui-rgba8`.
#[test]
fn p2_app_skia_surface_shows_in_a_canvas_element() {
    let app = headless(200.0, 120.0, 1.0);
    let document = app.document();
    document.body().set_background_color(Color::WHITE).unwrap();
    let canvas = canvas_at(document, 20.0, 10.0, 100.0, 50.0);
    document.body().append_child(&canvas).unwrap();

    let mut surface = raster_surface(100, 50);
    let c = surface.canvas();
    c.clear(SkColor::TRANSPARENT);
    let mut fill = Paint::default();
    fill.set_color(SkColor::from_rgb(0, 128, 0));
    c.draw_rect(skia_safe::Rect::from_xywh(0.0, 0.0, 50.0, 50.0), &fill);
    // Stroke, dash and a gradient: everything a 2D context needs is in Skia.
    let mut stroke = Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_style(paint::Style::Stroke);
    stroke.set_stroke_width(4.0);
    stroke.set_path_effect(PathEffect::dash(&[6.0, 3.0], 0.0));
    // The deprecated form keeps the probe short.
    #[allow(deprecated)]
    let gradient = Shader::linear_gradient(
        (Point::new(50.0, 0.0), Point::new(100.0, 0.0)),
        [SkColor::RED, SkColor::BLUE].as_ref(),
        None,
        TileMode::Clamp,
        None,
        None,
    );
    stroke.set_shader(gradient);
    c.draw_circle((75.0, 25.0), 18.0, &stroke);

    let mut bridge = SurfaceBridge::new(document, &canvas, (100.0, 50.0), "p2");
    bridge.present(&mut surface).unwrap();
    let bitmap = app.render_at(0.0).unwrap();
    // The green square lands at the element's position, 1:1.
    assert_eq!(pixel(&bitmap, 20, 10), [0, 128, 0, 255]);
    assert_eq!(pixel(&bitmap, 69, 59), [0, 128, 0, 255]);
    assert_eq!(pixel(&bitmap, 19, 10), [255, 255, 255, 255]);
    // Transparent canvas pixels show the page behind.
    assert_eq!(pixel(&bitmap, 118, 12), [255, 255, 255, 255]);
}

/// P3. Div caches a decoded image by its "sha256" string alone: a second
/// registration under the same key with different pixels shows the old ones.
#[test]
fn p3_same_cache_key_shows_stale_pixels() {
    let app = headless(64.0, 64.0, 1.0);
    let document = app.document();
    let canvas = canvas_at(document, 0.0, 0.0, 8.0, 8.0);
    document.body().append_child(&canvas).unwrap();
    let mut surface = raster_surface(8, 8);
    surface.canvas().clear(SkColor::RED);
    let first = document
        .register_image_resource("a", RGBA8_MIME, "same-key", rgba8_resource(&mut surface))
        .unwrap();
    canvas.set_image_resource(first, Some((8.0, 8.0))).unwrap();
    assert_eq!(pixel(&app.render_at(0.0).unwrap(), 4, 4), [255, 0, 0, 255]);

    surface.canvas().clear(SkColor::BLUE);
    let second = document
        .register_image_resource("b", RGBA8_MIME, "same-key", rgba8_resource(&mut surface))
        .unwrap();
    canvas.set_image_resource(second, Some((8.0, 8.0))).unwrap();
    let shown = pixel(&app.render_at(0.0).unwrap(), 4, 4);
    println!("P3 resource ids {first:?} {second:?}; pixel after blue frame: {shown:?}");
    assert_eq!(shown, [255, 0, 0, 255], "stale: cached by key");
}

/// P4. At device scale 2 a canvas backed by 2x pixels maps 1:1 to device
/// pixels: alternating one-device-pixel columns survive exactly.
#[test]
fn p4_hidpi_backing_store_maps_one_to_one() {
    let app = headless(120.0, 60.0, 2.0);
    let document = app.document();
    let canvas = canvas_at(document, 10.0, 5.0, 50.0, 25.0);
    document.body().append_child(&canvas).unwrap();
    let mut surface = raster_surface(100, 50);
    let c = surface.canvas();
    c.clear(SkColor::WHITE);
    let mut black = Paint::default();
    black.set_color(SkColor::BLACK);
    for x in (0..100).step_by(2) {
        c.draw_rect(skia_safe::Rect::from_xywh(x as f32, 0.0, 1.0, 50.0), &black);
    }
    SurfaceBridge::new(document, &canvas, (50.0, 25.0), "p4")
        .present(&mut surface)
        .unwrap();
    let bitmap = app.render_at(0.0).unwrap();
    assert_eq!(bitmap.width(), 240);
    let row: Vec<u8> = (20..30).map(|x| pixel(&bitmap, x, 30)[0]).collect();
    println!("P4 device row at scale 2: {row:?}");
    assert_eq!(row, [0, 255, 0, 255, 0, 255, 0, 255, 0, 255]);
}

/// P5. Because the canvas is ordinary replaced content in Div's one scene
/// picture, Div content stacks over it, and a scroller clips and moves it.
#[test]
fn p5_overlay_and_scroll_clip_compose_normally() {
    let app = headless(200.0, 200.0, 1.0);
    let document = app.document();
    document.body().set_background_color(Color::WHITE).unwrap();
    let scroller = Element::create(document, "div").unwrap();
    scroller.set_position(Position::Absolute).unwrap();
    scroller.set_left(Length::px(0.0)).unwrap();
    scroller.set_top(Length::px(0.0)).unwrap();
    scroller.set_width(LengthValue::px(100.0)).unwrap();
    scroller.set_height(LengthValue::px(100.0)).unwrap();
    scroller.set_overflow(Overflow::Hidden).unwrap();
    let canvas = Element::create(document, "canvas").unwrap();
    canvas.set_display(Display::Block).unwrap();
    canvas.set_width(LengthValue::px(100.0)).unwrap();
    canvas.set_height(LengthValue::px(300.0)).unwrap();
    scroller.append_child(&canvas).unwrap();
    document.body().append_child(&scroller).unwrap();
    // A "popover" over the canvas.
    let popover = Element::create(document, "div").unwrap();
    popover.set_position(Position::Absolute).unwrap();
    popover.set_left(Length::px(40.0)).unwrap();
    popover.set_top(Length::px(40.0)).unwrap();
    popover.set_width(LengthValue::px(30.0)).unwrap();
    popover.set_height(LengthValue::px(30.0)).unwrap();
    popover.set_z_index(10).unwrap();
    popover
        .set_background_color(Color::from_rgba8(0, 0, 255, 255))
        .unwrap();
    document.body().append_child(&popover).unwrap();

    // Top half red, bottom half green, 1:1.
    let mut surface = raster_surface(100, 300);
    let c = surface.canvas();
    c.clear(SkColor::from_rgb(255, 0, 0));
    let mut green = Paint::default();
    green.set_color(SkColor::from_rgb(0, 255, 0));
    c.draw_rect(skia_safe::Rect::from_xywh(0.0, 150.0, 100.0, 150.0), &green);
    SurfaceBridge::new(document, &canvas, (100.0, 300.0), "p5")
        .present(&mut surface)
        .unwrap();
    let before = app.render_at(0.0).unwrap();
    assert_eq!(pixel(&before, 10, 10), [255, 0, 0, 255]);
    assert_eq!(
        pixel(&before, 50, 50),
        [0, 0, 255, 255],
        "popover over canvas"
    );
    assert_eq!(pixel(&before, 10, 150), [255, 255, 255, 255], "clipped");

    scroller.scroll_to(0.0, 160.0).unwrap();
    let after = app.render_at(0.0).unwrap();
    assert_eq!(pixel(&after, 10, 10), [0, 255, 0, 255], "scrolled content");
    assert_eq!(pixel(&after, 50, 50), [0, 0, 255, 255]);
}

/// P6. Input: events carry viewport coordinates only. Element-local
/// coordinates must come from `bounding_rect`, which is the axis-aligned
/// bounds, so under a rotation the subtraction is wrong.
#[test]
fn p6_pointer_coordinates_are_viewport_only() {
    use std::{cell::RefCell, rc::Rc};
    let app = headless(300.0, 300.0, 1.0);
    let document = app.document();
    let canvas = canvas_at(document, 100.0, 100.0, 100.0, 50.0);
    document.body().append_child(&canvas).unwrap();
    let seen = Rc::new(RefCell::new(Vec::<(String, f32, f32)>::new()));
    for kind in [
        "mousedown",
        "mousemove",
        "mouseup",
        "pointerdown",
        "pointermove",
    ] {
        let seen = seen.clone();
        canvas
            .on(kind, move |event: &Event| {
                seen.borrow_mut()
                    .push((event.event_type.clone(), event.mouse_x, event.mouse_y))
            })
            .unwrap();
    }
    let none = Modifiers::NONE;
    document
        .dispatch_mouse_event(MouseEventType::Down, 110.0, 120.0, MouseButton::Left, none)
        .unwrap();
    let rect = canvas.bounding_rect().unwrap().unwrap();
    let (x, y) = (seen.borrow()[0].1, seen.borrow()[0].2);
    println!(
        "P6 mousedown at viewport ({x}, {y}); local by bounds ({}, {})",
        x - rect.x,
        y - rect.y
    );
    assert_eq!((x - rect.x, y - rect.y), (10.0, 20.0));

    // Pointer capture, then a move outside the canvas.
    canvas.set_pointer_capture(0).unwrap();
    document
        .dispatch_mouse_event(MouseEventType::Move, 20.0, 20.0, MouseButton::Left, none)
        .unwrap();
    document
        .dispatch_mouse_event(MouseEventType::Up, 20.0, 20.0, MouseButton::Left, none)
        .unwrap();
    let events: Vec<String> = seen.borrow().iter().map(|e| e.0.clone()).collect();
    println!("P6 events at the canvas with capture: {events:?}");

    // Rotated by 90 degrees: the bounds become 50x100 around the same
    // center, and viewport minus bounds no longer gives canvas coordinates.
    canvas
        .set_transform(TransformList(vec![TransformOperation::Rotate(90.0)]))
        .unwrap();
    let rotated = canvas.bounding_rect().unwrap().unwrap();
    println!("P6 bounds after rotate(90deg): {rotated:?}");
    assert_eq!((rotated.width, rotated.height), (50.0, 100.0));
}

/// P7. There is no frame callback, so an app animates a canvas by pushing a
/// new image per frame. Measure what one pushed frame costs end to end
/// (draw, readback, register, relayout, re-record, full raster) next to a
/// frame with no change.
#[test]
fn p7_per_frame_cost_of_the_image_workaround() {
    let (vw, vh, cw, ch) = (1024.0, 768.0, 640, 480);
    let app = headless(vw, vh, 1.0);
    let document = app.document();
    document.body().set_background_color(Color::WHITE).unwrap();
    let canvas = canvas_at(document, 32.0, 32.0, cw as f32, ch as f32);
    document.body().append_child(&canvas).unwrap();
    let mut surface = raster_surface(cw, ch);
    let mut bridge = SurfaceBridge::new(document, &canvas, (cw as f32, ch as f32), "p7");
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let draw = |surface: &mut skia_safe::Surface, frame: u64, paint: &mut Paint| {
        let c = surface.canvas();
        c.clear(SkColor::from_rgb(20, 20, 30));
        for i in 0..200u64 {
            let t = (frame + i) as f32 * 0.05;
            paint.set_color(SkColor::from_rgb((i * 7 % 255) as u8, 160, 220));
            c.draw_circle(
                (320.0 + 250.0 * t.cos(), 240.0 + 180.0 * (t * 1.3).sin()),
                6.0,
                paint,
            );
        }
    };
    // Warm up.
    for _ in 0..5 {
        draw(&mut surface, 0, &mut paint);
        bridge.present(&mut surface).unwrap();
        app.render_at(0.0).unwrap();
    }
    let frames = 60;
    let (mut t_draw, mut t_push, mut t_render) = (0.0, 0.0, 0.0);
    for frame in 0..frames {
        let start = Instant::now();
        draw(&mut surface, frame, &mut paint);
        let drawn = Instant::now();
        bridge.present(&mut surface).unwrap();
        let pushed = Instant::now();
        app.render_at(0.0).unwrap();
        let rendered = Instant::now();
        t_draw += (drawn - start).as_secs_f64();
        t_push += (pushed - drawn).as_secs_f64();
        t_render += (rendered - pushed).as_secs_f64();
    }
    let idle_start = Instant::now();
    for _ in 0..frames {
        app.render_at(0.0).unwrap();
    }
    let idle = idle_start.elapsed().as_secs_f64();
    let ms = |s: f64| s * 1000.0 / frames as f64;
    println!(
        "P7 {cw}x{ch} canvas in {vw}x{vh} viewport, {} frames pushed: \
         draw {:.2} ms, readback+register+set {:.2} ms, Div render {:.2} ms per frame; \
         unchanged-document render {:.3} ms",
        bridge.frames(),
        ms(t_draw),
        ms(t_push),
        ms(t_render),
        ms(idle)
    );
}
