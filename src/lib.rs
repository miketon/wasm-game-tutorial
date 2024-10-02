use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, window, CanvasRenderingContext2d, HtmlCanvasElement};

struct HtmlConst;

impl HtmlConst {
    const CANVAS_ID: &'static str = "canvas";
    const CONTEXT_2D: &'static str = "2d";
}

struct TriangleConst;

// TODO: expose these values to web interface button for users to change
impl TriangleConst {
    const DEPTH: u8 = 5;
    const LENGTH: f64 = 600.0;
}

// TODO: Define a type alias for a triangle points
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

    // generate and draw triangles
    let tri_lod_0_0: TrianglePoints = compute_triangle_points(TriangleConst::LENGTH);
    console::log_1(&format!("[main_js] {:?}", tri_lod_0_0).into());
    sierpinkis(&context, tri_lod_0_0, TriangleConst::DEPTH)?;

    Ok(())
}

fn sierpinkis(
    context: &CanvasRenderingContext2d,
    points: TrianglePoints,
    depth: u8,
) -> Result<(), JsValue> {
    // TODO: figure out why this doesn't print, but main_js log does print
    // console::log_1(&format!("[sierpinkis] {:?}", depth).into());
    // console::log_1(&JsValue::from_str(&format!("[sierpinkis] depth: {}", depth)));
    // this prevents infinite recursion where u8 0 - 1 = 255
    // because u8 is unsigned
    // TODO: use usize instead of u8
    if depth == 0 {
        return Ok(());
    }

    draw_triangle(context, points)?;
    if TriangleConst::DEPTH - depth == 1 {
        // debug draw each triangle point values
        debug_triangle_point_values(context, points)?;
    }

    let sub_triangles = compute_sub_triangles(points);
    for sub_triangle in sub_triangles.iter() {
        sierpinkis(context, *sub_triangle, depth - 1)?;
    }

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
    // TODO: Fill with a random color at each depth
    // context.fill();

    Ok(())
}

/// return 3 points of equilateral triangle given length
fn compute_triangle_points(length: f64) -> TrianglePoints {
    [
        (length / 2.0, 0.0), // top
        (0.0, length),       // bottom-left
        (length, length),    // botttom-right
    ]
}

fn compute_sub_triangles(points: TrianglePoints) -> [TrianglePoints; 3] {
    let [top, left, right] = points;
    let mid_left = midpoint(top, left);
    let mid_right = midpoint(top, right);
    let mid_bottom = midpoint(left, right);

    [
        [top, mid_left, mid_right],
        [mid_left, left, mid_bottom],
        [mid_right, mid_bottom, right],
    ]
}

fn midpoint(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5)
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
