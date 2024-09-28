use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement};

struct HtmlConst;

impl HtmlConst {
    const CANVAS_ID: &'static str = "canvas";
    const CONTEXT_2D: &'static str = "2d";
}

const TRIANGLE_LENGTH: f64 = 600.0;

// Define a type alias for a triangle points
type TrianglePoints = [(f64, f64); 3];

#[wasm_bindgen]
pub fn main_js() -> Result<(), JsValue> {
    // setup better panic messages for debugging
    console_error_panic_hook::set_once();

    // get context
    let window = window().expect("Failed to get window");
    // TODO-done: unwrap -> expect for more explicit error messages
    let document = window.document().expect("document on window required");
    let canvas = document
        .get_element_by_id(HtmlConst::CANVAS_ID)
        .expect("canvas element required")
        .dyn_into::<HtmlCanvasElement>()
        .expect("HtmlCanvasElement required");
    let context = canvas
        .get_context(HtmlConst::CONTEXT_2D)
        .expect("2d context required")
        .expect("context should exist")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("context should be a CanvasRenderingContext2d");

    // lod 0
    let length_0 = TRIANGLE_LENGTH;
    // lod 1
    let length_1 = length_0 * 0.5;
    // lod 2
    let length_2 = length_1 * 0.5;
    // draw @context
    // - add '?' end of function if function returns a Result<>
    // - web assembly code interacting with javascript can fail silently,
    // returning Result<> fails loudly as a forcing factor to fix
    let tri_lod_0_0: TrianglePoints = compute_triangle_points(length_0);
    // base for lod 1 triangle
    let tri_lod_1_base: TrianglePoints = compute_triangle_points(length_1);
    // we then offset position that base and draw
    let tri_lod_1_1: TrianglePoints = tri_lod_1_base.map(|(x, y)| (x + length_2, y));
    let tri_lod_1_2: TrianglePoints = tri_lod_1_base.map(|(x, y)| (x, y + length_1));
    let tri_lod_1_3: TrianglePoints = tri_lod_1_base.map(|(x, y)| (x + length_1, y + length_1));

    // lod_0
    draw_triangle(&context, tri_lod_0_0)?;
    // lod_1
    draw_triangle(&context, tri_lod_1_1)?;
    draw_triangle(&context, tri_lod_1_2)?;
    draw_triangle(&context, tri_lod_1_3)?;

    // debug draw each triangle point values
    debug_triangle_point_values(&context, tri_lod_1_1)?;
    debug_triangle_point_values(&context, tri_lod_1_2)?;
    debug_triangle_point_values(&context, tri_lod_1_3)?;

    Ok(())
}

fn draw_triangle(
    context: &CanvasRenderingContext2d,
    points: TrianglePoints,
) -> Result<(), JsValue> {
    // destructuring for readability
    let [top, left, right] = points;

    // path out triangle
    context.begin_path();
    context.move_to(top.0, top.1);
    context.line_to(left.0, left.1);
    context.line_to(right.0, right.1);
    context.close_path();

    context.stroke();
    // context.fill();

    Ok(())
}

/// return 3 points of equilateral triangle given length
fn compute_triangle_points(length: f64) -> TrianglePoints {
    [
        (length / 2.0, 0.0), // top
        (0.0, length),       // bottom-left
        (length, length),    // bottom-right
    ]
}

fn debug_triangle_point_values(
    context: &CanvasRenderingContext2d,
    points: TrianglePoints,
) -> Result<(), JsValue> {
    let offset = 10.0;
    // destructuring for readability
    let [top, left, right] = points.map(|(x, y)| (x, y + offset));
    // draw values as text for each point
    context.fill_text(&format!("{:?}", top), top.0, top.1)?;
    context.fill_text(&format!("{:?}", left), left.0, left.1)?;
    context.fill_text(&format!("{:?}", right), right.0, right.1)?;

    Ok(())
}
