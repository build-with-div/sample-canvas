//! A port of [Konva](https://konvajs.org) 10.9.1's scene graph to Rust.
//!
//! Konva is a 2D canvas framework: a stage holds layers, layers hold groups
//! and shapes, every node has a transform, and Konva dispatches its own
//! events through the tree. This crate ports that scene graph: `Transform`,
//! the node tree and its attributes, absolute transforms and positions,
//! client rects, and Konva's events with bubbling.
//!
//! Drawing, hit detection and dragging are not ported yet: they need
//! `CanvasRenderingContext2D` (W2) and Konva's color-key hit canvas (W7).
//! `tests/oracle.rs` compares the port with Konva 10.9.1 itself.

pub mod js;
pub mod node;
pub mod transform;

pub use node::{
    ClassName, ClientRectConfig, Handler, KonvaEvent, Node, NodeType, TransformsEnabled, Value,
};
pub use transform::{Decomposed, IRect, Transform, Vector2d};
