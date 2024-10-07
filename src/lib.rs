use getrandom::getrandom;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, window, CanvasRenderingContext2d, HtmlCanvasElement};

// Represents three points of a triangle in 2D space
type TrianglePoints = [(f64, f64); 3];
// ELI5: Represent color as u8 vs usize given it's 8bit 0-255 range
type Color = (u8, u8, u8);

// Constants related to HTML elements
struct HtmlConst;

impl HtmlConst {
    const CANVAS_ID: &'static str = "canvas";
    const CONTEXT_2D: &'static str = "2d";
}

// ELI5: Can't use static in an impl block ... here's why :
// - a) static items are associated with the entire program not just a type
// - b) impl blocks are for defining methods and associated functions for a
// specific type, NOT for declaring program-wide data
// - c) allowing statics in impl blocks adds confusion wrt scope and lifetime
// of these variables
// HACK: moving to the module level instead
static DEPTH: AtomicUsize = AtomicUsize::new(5);
// ELI5: f64.to_bits() is currently not stable as a const function
// static LENGTH: AtomicU64 = AtomicU64::new(600.0_f64.to_bits());
// - PHOTOSHOP terms - this unstable feature is an experimental filter that
// isn't supported in the current release, so our workaround is to
// add a note to manually apply filter on file open (init)
// - updated workaround, using once_cell instead of init
// TODO: Explain ... why didn't we have to use a js version of once_cell like
// we did with getrandom?
static LENGTH: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(600.0_f64.to_bits()));
const LENGTH_DEFAULT: f64 = 600.0;

// Constants and utility functions for triangle operations
struct TriangleConst;

impl TriangleConst {
    pub fn get_depth() -> usize {
        DEPTH.load(Ordering::Relaxed)
    }

    pub fn set_depth(depth: usize) {
        DEPTH.store(depth, Ordering::Relaxed)
    }

    pub fn get_length() -> f64 {
        let length = f64::from_bits(LENGTH.load(Ordering::Relaxed));
        if length <= 0.0 {
            LENGTH_DEFAULT
        } else {
            length
        }
    }

    pub fn set_length(length: f64) {
        LENGTH.store(length.to_bits(), Ordering::Relaxed)
    }
}

/// Gets the depth of the Sierpinski triangle
#[wasm_bindgen]
pub fn get_depth() -> usize {
    TriangleConst::get_depth()
}

/// Sets the depth of the Sierpinski triangle
#[wasm_bindgen]
pub fn set_depth(depth: usize) {
    TriangleConst::set_depth(depth);
}

/// Gets the length of the triangle's sides
#[wasm_bindgen]
pub fn get_length() -> f64 {
    TriangleConst::get_length()
}

/// Sets the length of the triangle's sides
/// - Only accepts positive values
#[wasm_bindgen]
pub fn set_length(length: f64) {
    // ensure only positive values
    if length > 0.0 {
        TriangleConst::set_length(length);
    } else {
        console::log_1(&JsValue::from_str("length must be positive"));
    }
}

/// Main entry for Webassembly module
/// - initializes canvas
/// - setups context
/// - starts drawing
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
    let tri_lod_0_0: TrianglePoints = compute_triangle_points(TriangleConst::get_length());
    console::log_1(&format!("[main_js] {:?}", tri_lod_0_0).into());
    sierpinski(
        &context,
        tri_lod_0_0,
        (0, 255, 255),
        TriangleConst::get_depth(),
    )?;

    Ok(())
}

fn sierpinski(
    context: &CanvasRenderingContext2d,
    points: TrianglePoints,
    color: Color,
    depth: usize,
) -> Result<(), JsValue> {
    if depth == 0 {
        return Ok(());
    }

    draw_triangle(context, points, color)?;
    if TriangleConst::get_depth() - depth == 1 {
        // debug draw each triangle point values
        debug_triangle_point_values(context, points)?;
    }

    let sub_triangles = compute_sub_triangles(points);
    for sub_triangle in sub_triangles.iter() {
        sierpinski(context, *sub_triangle, random_color(), depth - 1)?;
    }

    Ok(())
}

fn draw_triangle(
    context: &CanvasRenderingContext2d,
    points: TrianglePoints,
    color: Color,
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
    let color_string = format!("rgb({}, {}, {})", color.0, color.1, color.2);
    context.set_fill_style(&JsValue::from_str(&color_string));
    context.fill();

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

// TODO: is there a way to get brighter colors as depth increases?
fn random_color() -> Color {
    let mut buf = [0u8; 3];
    // getrandom is designed to fill a buffer with random bytes
    // - it's a low level function serves as a foundation for other random
    // number generation tasks
    // - it should be fast and non blocking
    getrandom(&mut buf).expect("Failed to generate random Color");
    // returns the buffer filled with random bytes
    (buf[0], buf[1], buf[2])
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
