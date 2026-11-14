//! A port of [Konva](https://konvajs.org) 10.9.1 to Rust: its scene graph
//! and its drawing of rects and circles.
//!
//! Konva is a 2D canvas framework: a stage holds layers, layers hold groups
//! and shapes, every node has a transform, and Konva dispatches its own
//! events through the tree. Each layer draws into a scene canvas and into a
//! hidden hit canvas, where every shape is filled in its own color key.
//!
//! Div has no `CanvasRenderingContext2D` yet (W2), so the canvases' contexts
//! record the calls Konva makes and draw nothing (`CallRecorder`). Hit
//! detection reads the hit canvas back (W7) and dragging needs Pointer
//! Events (W14); neither is ported yet. `tests/oracle.rs` compares the port
//! with Konva 10.9.1 itself.

pub mod canvas;
pub mod color;
mod colors;
pub mod context;
mod draw;
pub mod js;
pub mod node;
pub mod transform;

pub use canvas::{Canvas, CanvasKind};
pub use color::{clear_color_keys, color_to_rgba, set_math_random, Rgba};
pub use context::{Arg, CallRecorder, CanvasCall, Context2d};
pub use node::{
    ClassName, ClientRectConfig, Handler, KonvaEvent, Node, NodeType, TransformsEnabled, Value,
};
pub use transform::{Decomposed, IRect, Transform, Vector2d};
