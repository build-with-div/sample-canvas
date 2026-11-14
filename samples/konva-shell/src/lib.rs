//! A Konva demo shell on Div: a toolbar, a stage in its container, a layers
//! panel and an inspector, over the Konva port in `samples/konva`.
//!
//! `stage_dom` ports the part of Konva that touches the page (the stage's
//! content element and each layer's `<canvas>`); `shell` is the app. Div has
//! no 2D context yet (W1, W2), so the canvases show nothing: Konva's draws
//! go to the port's call recorder. `tests/fidelity_shell.rs` compares the
//! shell with the same app on Konva itself in pinned Chromium.

pub mod shell;
pub mod stage_dom;

pub use shell::Shell;
pub use stage_dom::StageDom;
