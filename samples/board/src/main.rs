//! The board in a window. The canvas stays blank until Div has a 2D
//! context (W2); the toolbar, style panel, layers list, gestures and
//! shortcuts all work.

use board::shell::{Board, HEIGHT, WIDTH};
use div::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::builder()
        .title("Board")
        .size(LogicalSize::new(f64::from(WIDTH), f64::from(HEIGHT)))
        .build()?;
    app.mount(|| view! { <div id="host"></div> })?;
    let document = app.document().clone();
    let host = document.element_by_id("host")?.expect("the host");
    Board::mount(&document, &host);
    app.run_document()?;
    Ok(())
}
