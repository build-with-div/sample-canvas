//! The three.js shell in a window. The viewport stays blank until Div has a
//! WebGPU canvas (W17); the panes, the popover and the camera controls all
//! work.

use div::prelude::*;
use three_shell::shell::{Shell, HEIGHT, WIDTH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::builder()
        .title("three.js shell")
        .size(LogicalSize::new(f64::from(WIDTH), f64::from(HEIGHT)))
        .build()?;
    app.mount(|| view! { <div id="host"></div> })?;
    let document = app.document().clone();
    let host = document.element_by_id("host")?.expect("the host");
    Shell::mount(&document, &host);
    app.run_document()?;
    Ok(())
}
