use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, window, CanvasRenderingContext2d, HtmlCanvasElement};

struct HtmlConst;

impl HtmlConst {
    const CANVAS_ID: &'static str = "canvas";
    const CONTEXT_2D: &'static str = "2d";
}

// Can't use static in an impl block ... here's why :
// - a) static items are associated with the entire program not just a type
// - b) impl blocks are for defining methods and associated functions for a
// specific type, NOT for declaring program-wide data
// - c) allowing statics in impl blocks adds confusion wrt scope and lifetime
// of these variables
// HACK: moving to the module level instead
static DEPTH: AtomicUsize = AtomicUsize::new(5);
// ELI5: f64.to_bits() is currently not stable as a const function
// static LENGTH: AtomicU64 = AtomicU64::new(600.0_f64.to_bits());
// So as a workaround we will set to 0.0 and initialize on main
// PHOTOSHOP terms - this unstable feature is an experimental filter that
// isn't supported in the current release, so our workaround is to
// add a note to manually apply filter on file open (init)
static LENGTH: AtomicU64 = AtomicU64::new(0);
const LENGTH_DEFAULT: f64 = 600.0;

struct TriangleConst;

impl TriangleConst {
    // HACK: work around f_64.to_bits() not being stable
    pub fn init() {
        // Only if current length is invalid do we change length ...
        // Else we will interfere with values passed through html ui
        if Self::get_length() <= 0.0 {
            Self::set_length(LENGTH_DEFAULT);
        }
    }
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

// type alias for a triangle points
type TrianglePoints = [(f64, f64); 3];

#[wasm_bindgen]
pub fn get_depth() -> usize {
    TriangleConst::get_depth()
}

#[wasm_bindgen]
pub fn set_depth(depth: usize) {
    TriangleConst::set_depth(depth);
}

#[wasm_bindgen]
pub fn get_length() -> f64 {
    TriangleConst::get_length()
}

#[wasm_bindgen]
pub fn set_length(length: f64) {
    // ensure only positive values
    if length > 0.0 {
        TriangleConst::set_length(length);
    } else {
        console::log_1(&JsValue::from_str("length must be positive"));
    }
}

#[wasm_bindgen]
pub fn main_js() -> Result<(), JsValue> {
    // setup better panic messages for debugging
    console_error_panic_hook::set_once();
    // HACK: work around f_64.to_bits() not being stable
    TriangleConst::init();

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
    sierpinkis(&context, tri_lod_0_0, TriangleConst::get_depth())?;

    Ok(())
}

fn sierpinkis(
    context: &CanvasRenderingContext2d,
    points: TrianglePoints,
    depth: usize,
) -> Result<(), JsValue> {
    // TODO: figure out why this doesn't print, but main_js log does print
    // console::log_1(&format!("[sierpinkis] {:?}", depth).into());
    // console::log_1(&JsValue::from_str(&format!("[sierpinkis] depth: {}", depth)));
    if depth == 0 {
        return Ok(());
    }

    draw_triangle(context, points)?;
    if TriangleConst::get_depth() - depth == 1 {
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
