//! An Excalidraw-style board on Div: a toolbar, a style panel, a layers
//! list and pointer gestures over a `<canvas>`, drawing with the rough.js
//! port in `samples/roughjs`.
//!
//! `scene` is the board's model; `shell` is its Div DOM. Div has no 2D
//! context yet (W1, W2), so the canvas shows nothing: the scene's rough.js
//! calls go to a recorder. `tests/fidelity_board.rs` compares the shell
//! with the same board as a web page in pinned Chromium.

pub mod scene;
pub mod shell;

pub use scene::{BoardElement, ItemStyle, Scene, StyleChange, Tool};
pub use shell::Board;
