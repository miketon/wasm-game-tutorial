use wasm_bindgen::prelude::*;
use web_sys::{
    console,
    window,
    HtmlCanvasElement,
    CanvasRenderingContext2d,
};
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub fn main_js()-> Result<(), JsValue> {

    // TODO: Explain what this does
    console_error_panic_hook::set_once();

    // get context
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    // TODO: instead of 2 unwraps consider using expect : better error handling
    let context = canvas.get_context("2d").unwrap().unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>().unwrap();
    draw_triangle(&context);
    Ok(())
}

fn draw_triangle(context: &CanvasRenderingContext2d) {
    // get to start position
    context.move_to(600.0, 0.0);    // top of triangle

    // draw triangle
    context.begin_path();
    context.line_to(0.0, 600.0);    // bottom left of triangle
    context.line_to(600.0, 600.0);  // bottom right of triangle
    context.line_to(300.0, 0.0);    // back to top of triangle
    // close and fill shape
    context.close_path();
    context.stroke();
    context.fill();
}
