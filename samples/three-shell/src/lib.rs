//! A three.js example shell on Div: a toolbar, an objects pane and a
//! viewport side by side with a splitter between them, and a settings
//! popover over the viewport's `<canvas>`, which OrbitControls (the port
//! in `samples/three`) drives with the mouse, the wheel and the keys.
//!
//! Div has no WebGPU canvas yet (W17), so the canvas shows nothing; the
//! camera it would render with is the port's. `tests/fidelity_shell.rs`
//! compares the shell with the same page on three.js itself in pinned
//! Chromium.

pub mod shell;

pub use shell::Shell;
