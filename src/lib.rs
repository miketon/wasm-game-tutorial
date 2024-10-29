// ==================== Imports ====================
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[macro_use]
mod browser;
mod engine;

// OOOF: include and activate sierpinksi triangle
// ==================== Structs ====================
#[derive(Deserialize, Serialize)]
struct Cell {
    frame: Rect,
}

#[derive(Deserialize, Serialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

// impl Rect {
//     // In code new() and center() are CHILDREN of the Rect impl block :
//     // Rect
//     // ├─ x
//     // ├─ y
//     // ├─ w
//     // ├─ h
//     // ├─ new()
//     // └─ center()
//     // NOTE: The document Symbols table will VISUALLY list them as SIBLINGS
//     // for navigating convenience
//     fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
//         Self { x, y, w, h }
//     }
//     fn center(&self) -> (f64, f64) {
//         (self.x + self.w * 0.5, self.y + self.h * 0.5)
//     }
// }

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
    let context = browser::context().expect("context should be a CanvasRenderingContext2d");

    // spawns a new asynchronous task in local thread, for web assembly
    // environment, using wasm_bindgen_futures
    browser::spawn_local(async move {
        let sheet: Sheet = browser::fetch_json::<Sheet>("rhb.json")
            .await
            .expect("Could not fetch rhb.json");

        let image = engine::load_image("rhb.png")
            .await
            .expect("Could not load rhb.png");

        let mut frame = -1;
    });

    Ok(())
}
