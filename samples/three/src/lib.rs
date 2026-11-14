//! A port of three.js 0.187.0's camera math and `OrbitControls` to Rust:
//! the part of a three.js example that turns pointer, wheel and key input
//! into a camera's view and projection matrices.
//!
//! - `math`: `Vector2`, `Vector3`, `Quaternion`, `Euler`, `Matrix4`,
//!   `Spherical`;
//! - `camera`: `PerspectiveCamera`, with `Object3D`'s `lookAt` and world
//!   matrices and the WebGL and WebGPU clip spaces;
//! - `orbit`: `OrbitControls` for the mouse, the wheel and the keys;
//! - `jsmath`: `Math.sin`, `Math.cos` and `Math.pow` rounded as Chromium
//!   rounds them.
//!
//! `tests/oracle.rs` compares every result with three.js itself in pinned
//! Chromium on the same scripts. Rendering the scene needs WebGPU on a `<canvas>`
//! (W17), which Div doesn't have yet.

pub mod camera;
pub mod jsmath;
pub mod math;
pub mod orbit;

pub use camera::{PerspectiveCamera, View};
pub use math::{
    CoordinateSystem, Euler, EulerOrder, Matrix4, Quaternion, Spherical, Vector2, Vector3,
};
pub use orbit::{
    ControlsEvent, Element, Modifiers, MouseAction, OrbitControls, PointerEvent, State, WheelEvent,
};
