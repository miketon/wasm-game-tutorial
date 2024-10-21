// ==================== Imports ====================
use serde::Deserialize;
use serde_wasm_bindgen::from_value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// OOOF: include and activate sierpinksi triangle
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
// Constants related to HTML elements
mod html {
    pub const CANVAS_ID: &str = "canvas";
    pub const CONTEXT_2D: &str = "2d";
}

// ==================== Structs ====================
#[derive(Deserialize)]
struct Cell {
    frame: Rect,
}

#[derive(Deserialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    // In code new() and center() are CHILDREN of the Rect impl block :
    // Rect
    // ├─ x
    // ├─ y
    // ├─ w
    // ├─ h
    // ├─ new()
    // └─ center()
    // NOTE: The document Symbols table will VISUALLY list them as SIBLINGS
    // for navigating convenience
    fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }
    fn center(&self) -> (f64, f64) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
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
    let document = window.document().expect("document on window required");
    let canvas = document
        .get_element_by_id(html::CANVAS_ID)
        .expect("canvas element required")
        .dyn_into::<HtmlCanvasElement>()
        .expect("HtmlCanvasElement required");
    let context = canvas
        .get_context(html::CONTEXT_2D)
        // NOTE: Explian why we are unwrapping twice here?
        .expect("2d context required")
        .expect("context should exist")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("context should be a CanvasRenderingContext2d");

    // spawns a new asynchronous task in local thread, for web assembly
    // environment, using wasm_bindgen_futures
    wasm_bindgen_futures::spawn_local(
        // starts an asynchronous closure
        // - 'move' keyword indicates that the closure will take ownership of
        // any variables it uses from the surrounding scope
        async move {
            let image = HtmlImageElement::new().expect("HtmlImageElement required");
            let json = fetch_json("rhb.json")
                .await
                .expect("Could not fetch rhb.json");
            let sheet: Sheet =
                from_value(json).expect("Could not convert rhb.json into a Sheet structure");

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
            // - Closure::once() EXPLICITLY specifies a FnOnce closure, else ...
            // - NOTE: send() call IMPLICITLY have compiled to a FnOnce closure
            let callback_success = Closure::once(move || {
                if let Some(tx) = success_tx.borrow_mut().take() {
                    let _ = tx.send(Ok(()));
                }
            });
            let callback_error = Closure::once(move |err: Event| {
                if let Some(tx) = error_tx.borrow_mut().take() {
                    let _ = tx.send(Err(err.into()));
                }
            });

            // Sets the callbacks
            image.set_onload(Some(
                callback_success
                    .as_ref()
                    // unchecked_ref is used to convert the call back to the
                    // correct type expected by 'set_onload'
                    .unchecked_ref(),
            ));
            image.set_onerror(Some(callback_error.as_ref().unchecked_ref()));

            // start loading the image by setting the source to our file name
            image.set_src("rhb.png");
            match rx.await {
                Ok(Ok(())) => {
                    let mut frame = -1;
                    console::log_1(&JsValue::from_str("[json] loading : DONE"));
                    #[rustfmt::skip]
                    let interval_callback = Closure::wrap(
                        Box::new(move || {
                            // increments frame coounter and wraps 0-7
                            frame = (frame+1) % 8;
                            // OOOF: rhb.json - run frames start at 1-8
                            // - RuntimeError: unreachable : if outside range
                            // - so we MUST +1 to frame counter
                            let frame_name = format!("Run ({}).png", frame +1);
                            // clears the existing context
                            context.clear_rect(0.0, 0.0, 600.0, 600.0);
                            // get sprite data for the current frame
                            let sprite = sheet.frames.get(&frame_name).expect("Cell not found");
                            // draw the current frame to the cleared context
                            context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                &image,
                                sprite.frame.x,
                                sprite.frame.y,
                                sprite.frame.w,
                                sprite.frame.h,
                                300.0,
                                300.0,
                                sprite.frame.w,
                                sprite.frame.h,
                            ).expect("Failed to draw image");
                        }) as Box<dyn FnMut()>
                    );

                    // Sets up interval that calls the animation closure
                    // every 50ms
                    let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
                        interval_callback.as_ref().unchecked_ref(),
                        50,
                    );

                    // Prevents the closure from being dropped when it goes
                    // out of scope
                    // - effectively dropped from Rust borrow checking, and
                    // hands off ownership of closure to Javascript runtime
                    // - HACK: runtime memory error NOT caught by Rust's static
                    // analysis
                    interval_callback.forget();
                }
                Ok(Err(err)) => {
                    console::log_1(&JsValue::from_str(&format!(
                        "[json] loading : ERROR {:?}",
                        err
                    )));
                }
                Err(_) => {
                    console::log_1(&JsValue::from_str("[json] loading : NO VALUE SENT"));
                }
            };
        },
    );
    Ok(())
}

async fn fetch_json(json_path: &str) -> Result<JsValue, JsValue> {
    let window = window().unwrap();
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(json_path)).await?;

    let resp: web_sys::Response = resp_value.dyn_into()?;
    wasm_bindgen_futures::JsFuture::from(resp.json()?).await
}
