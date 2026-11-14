//! Konva's browser DOM, ported onto Div: what `Stage._buildDOM`,
//! `Stage._resizeDOM` and `Stage.add` (`Stage.ts`), the `Canvas` constructor
//! and `setSize` (`Canvas.ts`) and `Layer._checkVisibility` (`Layer.ts`) do
//! to the page.
//!
//! `samples/konva` ports Konva's scene graph and drawing and has no DOM: it
//! doesn't depend on Div. This is the other half, kept with the shell.
//! Konva makes a `<canvas>` element in each canvas's constructor; here the
//! element of a layer's scene canvas is made when the layer is added to the
//! stage, from the state Konva would have given it by then.

use div::prelude::*;
use div::typed_style::{Display, LengthValue};
use konva::{ClassName, KonvaEvent, Node, Value};
use std::rc::Rc;

/// A stage with its container's DOM.
#[derive(Clone)]
pub struct StageDom {
    document: Document,
    pub stage: Node,
    /// `stage.content`: the `div.konvajs-content` Konva puts in the
    /// container.
    pub content: Element,
}

fn px(value: f64) -> LengthValue {
    LengthValue::px(value as f32)
}

/// `layer.canvas._canvas`, made as the `Canvas` constructor makes it.
fn canvas_element(document: &Document) -> Element {
    let canvas = Element::create(document, "canvas").expect("a canvas");
    // div-gap: 17 the `background` shorthand. Konva assigns
    // `style.background = 'transparent'`; Div has no `background`
    // property, so the canvas gets `background-color: transparent`, which
    // computes the same (the other longhands keep their initial values).
    canvas
        .apply_style(&style!("padding: 0px; margin: 0px; border-top-width: 0; border-right-width: 0; border-bottom-width: 0; border-left-width: 0; border-top-style: none; border-right-style: none; border-bottom-style: none; border-left-style: none; background-color: transparent; position: absolute; top: 0px; left: 0px"))
        .expect("the canvas style applies");
    canvas
}

/// What `setSize` writes to the element: the bitmap size as the `width`
/// and `height` attributes, and the CSS size.
fn size_element(element: &Element, layer: &Node) {
    let canvases = layer.canvases();
    let canvas = &canvases[0];
    let (width, height) = canvas.logical_size();
    element
        .set_attribute("width", &konva::js::number_to_string(canvas.width))
        .unwrap();
    element
        .set_attribute("height", &konva::js::number_to_string(canvas.height))
        .unwrap();
    element.set_width(px(width)).unwrap();
    element.set_height(px(height)).unwrap();
}

/// `_checkVisibility()`.
fn check_visibility(element: &Element, layer: &Node) {
    let display = if layer.visible() {
        Display::Block
    } else {
        Display::None
    };
    element.set_display(display).unwrap();
}

impl StageDom {
    /// `new Konva.Stage({container, width, height})`: the stage, and its
    /// `konvajs-content` element in `container`.
    pub fn new(document: &Document, container: &Element, width: f64, height: f64) -> StageDom {
        let stage = Node::new(
            ClassName::Stage,
            &[
                ("width", Value::Number(width)),
                ("height", Value::Number(height)),
            ],
        );
        // `container.innerHTML = ''`.
        while let Some(child) = container.first_child().unwrap() {
            container.remove_child(&child).unwrap();
        }
        let content = Element::create(document, "div").expect("a div");
        // div-gap: 17 user-select. Konva also assigns
        // `style.userSelect = 'none'`; Div has no `user-select` property.
        content
            .apply_style(&style!("position: relative"))
            .expect("the content style applies");
        content.set_attribute("class", "konvajs-content").unwrap();
        content.set_attribute("role", "presentation").unwrap();
        container.append_child(&content).unwrap();
        let dom = StageDom {
            document: document.clone(),
            stage,
            content,
        };
        dom.resize_content();
        // `_resizeDOM` on `widthChange` and `heightChange`: the port's own
        // handler sizes and draws the layers; this one sizes the elements.
        let weak = dom.content.downgrade();
        let handler_document = document.clone();
        dom.stage.on(
            "widthChange.konva heightChange.konva",
            Rc::new(move |stage: &Node, _: &mut KonvaEvent| {
                if let Some(content) = weak.upgrade() {
                    let dom = StageDom {
                        document: handler_document.clone(),
                        stage: stage.clone(),
                        content,
                    };
                    dom.resize_content();
                    for (layer, element) in stage.children().iter().zip(dom.canvas_elements()) {
                        size_element(&element, layer);
                    }
                }
            }),
        );
        dom
    }

    fn resize_content(&self) {
        self.content.set_width(px(self.stage.width())).unwrap();
        self.content.set_height(px(self.stage.height())).unwrap();
    }

    /// The layers' canvas elements, in the stage's order.
    fn canvas_elements(&self) -> Vec<Element> {
        let mut elements = Vec::new();
        let mut child = self.content.first_child().unwrap();
        while let Some(current) = child {
            child = current.next_sibling().unwrap();
            elements.push(current);
        }
        elements
    }

    /// `stage.add(layer)`: sizes and draws the layer, then appends its
    /// canvas element to the content.
    pub fn add(&self, layer: &Node) {
        self.stage.add(layer);
        let element = canvas_element(&self.document);
        size_element(&element, layer);
        check_visibility(&element, layer);
        let weak = element.downgrade();
        layer.on(
            "visibleChange.konva",
            Rc::new(move |layer: &Node, _: &mut KonvaEvent| {
                if let Some(element) = weak.upgrade() {
                    check_visibility(&element, layer);
                }
            }),
        );
        self.content.append_child(&element).unwrap();
    }
}
