use wasm_bindgen::prelude::*;
use web_sys::{
    CanvasRenderingContext2d,
};
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub fn main_js()-> Result<(), JsValue> {

    // setup better panic messages for debugging
    console_error_panic_hook::set_once();

    // get context
    let window = web_sys::window().unwrap();
    // TODO-done: unwrap -> expect for more explicit error messages
    let document = window.document().expect("document on window required");
    let canvas = document
        .get_element_by_id("canvas")
        .expect("canvas element required")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("HtmlCanvasElement required");
    let context = canvas.get_context("2d")
        .expect("2d context required")
        .expect("context should exist")
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .expect("context should be a CanvasRenderingContext2d");

    // draw @context
    // - add '?' end of function if function returns a Result<>
    // - web assembly code interacting with javascript can fail silently,
    // returning Result<> fails loudly as a forcing factor to fix 
    draw_triangle(&context)?;

    Ok(())
}

fn draw_triangle(context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
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
    
    Ok(())
}
