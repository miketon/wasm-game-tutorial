// ==================== Imports ====================
use getrandom::getrandom;  // js shim because access to system entropy needed
use once_cell::sync::Lazy; // no js shim needed because it's pure Rust impl
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
// ELI5: Javascript properties are public and web_sys :
// - a) just generates setter and getter functions
// - b) these functions take JsValue objects that represent objects owned by
// Javascript
// - c) read documentation for corresponding functions to check what types are
// needed when translating from Rust to Javascript
#[rustfmt::skip]
use web_sys::{
    console, 
    window, 
    CanvasRenderingContext2d, 
    HtmlCanvasElement, 
    HtmlImageElement,
    Event,
};

// ==================== Constants ====================
// ELI5: Can't use static in an impl block ... here's why :
// - a) static items are associated with the entire program not just a type
// - b) impl blocks are for defining methods and associated functfions for a
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
const LENGTH_DEFAULT: f64 = 600.0;
// TODO: Explain why we are using AtomicU64 ?
static LENGTH: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(LENGTH_DEFAULT.to_bits()));

// ==================== Types ====================
// Represents three points of a triangle in 2D space
type TrianglePoints = [(f64, f64); 3];
// ELI5: Represent color as u8 vs usize given it's 8bit 0-255 range
type Color = (u8, u8, u8);

// ==================== Modules ====================
// ELI5: When to use modules vs structs :
// - No Self : managing CONSTANTS and STATELESS functions doesn't need instance
// - more idiomatic way to handle triangle GEOMETRY and html STANDARDS
// - control @function level public/private namespace

// Constants related to HTML elements
mod html {
    pub const CANVAS_ID: &str = "canvas";
    pub const CONTEXT_2D: &str = "2d";
}

// Constants and utility functions for triangle operations
mod triangle {
    use super::*; // brings outer surrounding scope's items into this mod

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

// ==================== Structs ====================
#[derive(Debug, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}


impl Rect {
    // In code new() and center() are CHILDREN of the Rect impl block :
    // Rect
    // ├─ x
    // ├─ y
    // ├─ width
    // ├─ height
    // ├─ new()
    // └─ center()
    // NOTE: The document Symbols table will VISUALLY list them as SIBLINGS
    // for navigating convenience
    fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    fn center(&self) -> (f64, f64) {
        (self.x + self.width * 0.5, self.y + self.height * 0.5)
    }
}

// ==================== Main Functions ====================
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
        .get_element_by_id(html::CANVAS_ID)
        .expect("canvas element required")
        .dyn_into::<HtmlCanvasElement>()
        .expect("HtmlCanvasElement required");
    let context = canvas
        .get_context(html::CONTEXT_2D)
        .expect("2d context required")
        .expect("context should exist")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("context should be a CanvasRenderingContext2d");

    // create a Rect representing the canvas
    let canvas_rect = Rect::new(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    // generate and draw triangles
    let base_triangle: TrianglePoints = compute_triangle_points(triangle::get_length());
    // center the triangle
    let centered_triangle = center_triangle(base_triangle, canvas_rect);

    // Load and Draw sprite
    // clone to move a context reference into the async block
    let context_clone = context.clone(); 
    // spawns a new asynchronous task in local thread, for web assembly 
    // environment, using wasm_bindgen_futures
    wasm_bindgen_futures::spawn_local(
        // starts an asynchronous closure
        // - 'move' keyword indicates that the closure will take ownership of 
        // any variables it uses from the surrounding scope
        async move {
            let image = HtmlImageElement::new().expect("HtmlImageElement required");

            // creates a one-shot channel : 
            // - it's a single-use channel between asynchronous tasks
            // ELI5: help me understand this
            // ANSWER : This helps coordinate actions between different parts 
            // of your program, especially when one part needs to wait for 
            // another part to finish something.
            // Here's how it works:
            // - The channel::<()>() function creates this special walkie-talkie set.
            // - It gives you two parts:
            //  - tx (transmitter): This is your part of the walkie-talkie. 
            //  You'll use this to send the message.
            //  - rx (receiver): This is your friend's part. They'll use this 
            //  to listen for your message.
            // - You can only use this walkie-talkie set once. After you send 
            // a message, it stops working.
            let (tx, rx) = futures::channel::oneshot::channel::<Result<(), JsValue>>();
            // NOTE: RefCell vs Mutex because we are ASSUMING single thread
            // - if not we can't use Rc anyways because that's NOT threadsafe
            let success_tx = Rc::new(RefCell::new(Some(tx)));
            // clone so we can move into the error callback closure
            let error_tx = success_tx.clone();

            // Callback closures that will be called once the load attempt done
            // - Closure::once() EXPLICITLY specifies a FnOnce closure, but tx 
            // send() call would have IMPLICITLY specified a FnOnce closure
            // at compile time
            let callback_success = Closure::once(move || {
                if let Some(tx) = success_tx.borrow_mut().take(){
                    let _ = tx.send(Ok(()));
                }
            });

            let callback_error = Closure::once(move |err: Event| {
                if let Some(tx) = error_tx.borrow_mut().take() {
                    let _ = tx.send(Err(err.into()));
                }
            });

            // Sets the callbacks
            image.set_onload(Some(callback_success.as_ref()
                // unchecked_ref is used to convert the call back to the 
                // correct type expected by 'set_onload'
                .unchecked_ref()));
            image.set_onerror(Some(callback_error.as_ref().unchecked_ref()));

            // start loading the image by setting the source to our file name
            // image.set_src("Idle (1).png");
            image.set_src("Rhb.png");
            // wait for image to load or fail - (invisible progress bar)
            // - pauses execution until signal is received in one-shot channel
            // - walkie talkie are cleared because it was one-shot
            // - forget() call no longer necessary
            match rx.await {
                Ok(Ok(()))=> {
                    // Image loaded successfully
                    console::log_1(&JsValue::from_str("[image] loading : DONE"));
                    // Drawing operations here
                    context_clone.draw_image_with_html_image_element(&image, 0.0, 0.0)
                        .expect("Failed to draw image");
                },
                Ok(Err(err))=>{
                    // Image failed to load
                    console::log_1(&JsValue::from_str(&format!("[image] loading : ERROR {:?}", err)));
                },
                Err(_)=>{
                    // Channel closed without sending a value
                    console::log_1(&JsValue::from_str("[image] loading : NO VALUE SENT"));
                }
            };

        }
    );

    // even if image doesn't load, program will still continue to draw triangle
    // as opposed to hanging on await because 
    // - we've added error handling
    // - so it can exit the await loop
    console::log_1(&format!("[main_js] {:?}", base_triangle).into());
    sierpinski(
        &context,
        centered_triangle,
        random_color(),
        triangle::get_depth(),
    )?;

    Ok(())
}

// ==================== WASM-bindgen Functions ====================
/// Gets the depth of the Sierpinski triangle
#[wasm_bindgen]
pub fn get_depth() -> usize {
    triangle::get_depth()
}

/// Sets the depth of the Sierpinski triangle
#[wasm_bindgen]
pub fn set_depth(depth: usize) {
    triangle::set_depth(depth);
}

/// Gets the length of the triangle's sides
#[wasm_bindgen]
pub fn get_length() -> f64 {
    triangle::get_length()
}

/// Sets the length of the triangle's sides
/// - Only accepts positive values
#[wasm_bindgen]
pub fn set_length(length: f64) {
    // ensure only positive values
    if length > 0.0 {
        triangle::set_length(length);
    } else {
        console::log_1(&JsValue::from_str("length must be positive"));
    }
}

// ==================== Utility Functions ====================
// TODO: current implementation is recursive, consider :
// - iterative implementation ... with VecDeque
// - memoization ... with Hashing ?
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
    if triangle::get_depth() - depth == 1 {
        // debug draw each triangle point values
        debug_triangle_point_values(context, points)?;
    }

    let sub_triangles = compute_sub_triangles(points);
    //we want a shared color for each sub-triangle
    let color_lod = random_color();
    for sub_triangle in sub_triangles.iter() {
        sierpinski(context, *sub_triangle, color_lod, depth - 1)?;
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
    // fill style expects a string of format "rgb(255, 0 ,255)"
    let color_str = format!("rgb({}, {}, {})", color.0, color.1, color.2);
    context.set_fill_style(&JsValue::from_str(&color_str));
    context.fill();

    Ok(())
}

/// return 3 points of equilateral triangle given length
fn compute_triangle_points(length: f64) -> TrianglePoints {
    // multi by 0.5 to avoid divide by zero
    let height = length * 3.0_f64.sqrt() * 0.5;
    [
        (0.0, height),       // bottom-left
        (length, height),    // botttom-right
        (length * 0.5, 0.0), // top
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

// TODO: Can be optimized by pre-calculating some values?
fn center_triangle(points: TrianglePoints, canvas: Rect) -> TrianglePoints {
    let [top, left, right] = points;

    // Calculate the bounding box of the triangle
    let min_x = left.0.min(right.0).min(top.0);
    let max_x = left.0.max(right.0).max(top.0);
    let min_y = top.1.min(left.1).min(right.1);
    let max_y = top.1.max(left.1).max(right.1);

    // Calculate the center of the triangle
    let triangle_center_x = (min_x + max_x) * 0.5;
    let triangle_center_y = (min_y + max_y) * 0.5;

    // Get the center of the canvas
    let (canvas_center_x, canvas_center_y) = canvas.center();

    // Calculate the offset to move the triangle to center of canvas
    let offset_x = canvas_center_x - triangle_center_x;
    let offset_y = canvas_center_y - triangle_center_y;

    // Apply the offset to all TrianglePoints
    [
        (top.0 + offset_x, top.1 + offset_y),
        (left.0 + offset_x, left.1 + offset_y),
        (right.0 + offset_x, right.1 + offset_y),
    ]
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
    let offset = 20.0;
    // destructuring for readability
    // - also rounding to whole number on print
    let [top, left, right] = points.map(|(x, y)| {
        (
            (x + offset).round() as i32,
            (y + offset * 0.75).round() as i32,
        )
    });
    // draw values as text for each point
    context.fill_text(&format!("{} {}", top.0, top.1), top.0 as f64, top.1 as f64)?;
    context.fill_text(
        &format!("{} {}", left.0, left.1),
        left.0 as f64,
        left.1 as f64,
    )?;
    context.fill_text(
        &format!("{} {}", right.0, right.1),
        right.0 as f64,
        right.1 as f64,
    )?;

    Ok(())
}

// ==================== Tests ====================
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_midpoint() {
        let a = (0.0, 0.0);
        let b = (10.0, 20.0);
        let mid = midpoint(a, b);

        assert_relative_eq!(mid.0, 5.0);
        assert_relative_eq!(mid.1, 10.0);
    }

    #[test]
    fn test_compute_triangle_points() {
        let length = 100.0;
        let points = compute_triangle_points(length);

        assert_relative_eq!(points[0].0, 0.0);
        assert_relative_eq!(points[0].1, 86.60254037844387);
        assert_relative_eq!(points[1].0, 100.0);
        assert_relative_eq!(points[1].1, 86.60254037844387);
        assert_relative_eq!(points[2].0, 50.0);
        assert_relative_eq!(points[2].1, 0.0);
    }

    #[test]
    fn test_compute_sub_triangles() {
        let parent = [(0.0, 100.0), (100.0, 100.0), (50.0, 13.397459621556151)];

        let sub_triangles = compute_sub_triangles(parent);

        // Check the first sub-triangle
        assert_relative_eq!(sub_triangles[0][0].0, 0.0);
        assert_relative_eq!(sub_triangles[0][0].1, 100.0);
        assert_relative_eq!(sub_triangles[0][1].0, 50.0);
        assert_relative_eq!(sub_triangles[0][1].1, 100.0);
        assert_relative_eq!(sub_triangles[0][2].0, 25.0);
        assert_relative_eq!(sub_triangles[0][2].1, 56.69872981077808);

        // You can add similar checks for the other two sub-triangles
    }
}
