//! A port of [rough.js](https://roughjs.com) 4.6.6 to Rust.
//!
//! rough.js draws shapes that look hand-drawn. This crate ports its
//! generator, which turns shapes into drawing operations, together with the
//! four packages it depends on: `hachure-fill`, `path-data-parser`,
//! `points-on-curve` and `points-on-path`. The generator is pure geometry and
//! needs no web platform feature.
//!
//! rough.js's canvas renderer, which draws those operations with a
//! `CanvasRenderingContext2D`, is a later milestone; it needs the 2D context
//! Div does not have yet.
//!
//! The port follows the JavaScript closely, including its evaluation order
//! and JavaScript number semantics, so that a seeded drawing matches the
//! original. `tests/oracle.rs` compares it with rough.js 4.6.6 itself.

pub mod core;
pub mod fillers;
pub mod generator;
pub mod hachure_fill;
pub mod js;
pub mod path_data;
pub mod points_on_curve;
pub mod points_on_path;
pub mod renderer;

pub use crate::core::{
    Drawable, Op, OpSet, OpSetType, OpType, Options, PathInfo, Point, ResolvedOptions,
};
pub use crate::generator::RoughGenerator;
