//! A port of [rough.js](https://roughjs.com) 4.6.6 to Rust.
//!
//! rough.js draws shapes that look hand-drawn. This crate ports its
//! generator, which turns shapes into drawing operations, together with the
//! four packages it depends on: `hachure-fill`, `path-data-parser`,
//! `points-on-curve` and `points-on-path`. The generator is pure geometry and
//! needs no web platform feature.
//!
//! It also ports rough.js's canvas renderer, `RoughCanvas`, which draws
//! those operations with a `CanvasRenderingContext2D`. Div has no 2D context
//! yet (W2), so the renderer drives the `RoughCanvasContext` trait, whose only
//! implementation records the calls; see `canvas`.
//!
//! The port follows the JavaScript closely, including its evaluation order
//! and JavaScript number semantics, so that a seeded drawing matches the
//! original. `tests/oracle.rs` compares it with rough.js 4.6.6 itself.

pub mod canvas;
pub mod core;
pub mod fillers;
pub mod generator;
pub mod hachure_fill;
pub mod js;
pub mod path_data;
pub mod points_on_curve;
pub mod points_on_path;
pub mod renderer;

pub use crate::canvas::{
    CallRecorder, CanvasCall, CanvasFillRule, RoughCanvas, RoughCanvasContext,
};
pub use crate::core::{
    Drawable, Op, OpSet, OpSetType, OpType, Options, PathInfo, Point, ResolvedOptions,
};
pub use crate::generator::RoughGenerator;
