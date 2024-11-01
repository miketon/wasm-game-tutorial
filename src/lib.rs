// ==================== Imports ====================
use crate::engine::GameLoop;
use crate::game::WalkTheDog;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[macro_use]
mod browser;
mod engine;
mod game;

// ==================== Main Functions ====================
/// Main entry for Webassembly module
/// - initializes canvas
/// - setups context
/// - starts drawing
#[wasm_bindgen]
pub fn main_js() -> Result<(), JsValue> {
    // setup better panic messages for debugging
    console_error_panic_hook::set_once();

    browser::spawn_local(async move {
        let game = WalkTheDog::new();
        GameLoop::start(game)
            .await
            .expect("[lib.rs::main_js] Could not start game loop");
    });

    Ok(())
}
