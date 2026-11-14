//! The Konva shell in a window. The stage stays blank until Div has a 2D
//! context (W2); the toolbar, layers panel, inspector and keys all work.

use div::prelude::*;
use konva_shell::shell::{Shell, HEIGHT, WIDTH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::builder()
        .title("Konva shell")
        .size(LogicalSize::new(f64::from(WIDTH), f64::from(HEIGHT)))
        .build()?;
    app.mount(|| view! { <div id="host"></div> })?;
    let document = app.document().clone();
    let host = document.element_by_id("host")?.expect("the host");
    Shell::mount(&document, &host);
    app.run_document()?;
    Ok(())
}
