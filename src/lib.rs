// ==================== Imports ====================
use crate::engine::GameLoop;
use crate::game::WalkTheDog;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

// TABLE:
// ┌──────────────────────────────────────────────────────────────────────────┐
// │                      Directory Structure Analogy                         │
// ├───────────────────┬──────────────────────────────────────────────────────┤
// │ Code Directory    │          Photoshop Equivalent                        │
// ├───────────────────┼──────────────────────────────────────────────────────┤
// │ src/              │ Project Root                                         │
// │ ├── lib.rs        │ Project Manager/Asset Organization                   │
// │ ├── game.rs       │ Main Composition Where Animations Are Used           │
// │ └── sprite/       │ Character Asset Library                              │
// │     ├── mod.rs    │ Master Sprite Sheet Settings (.psd)                  │
// │     ├── states.rs │ Animation Sequences (Layer Groups)                   │
// │     └── red_hat_  │ Character-Specific Settings (Layer Comps)            │
// │         boy.rs    │                                                      │
// └───────────────────┴──────────────────────────────────────────────────────┘
// - @src/ in addition to game.rs and lib.rs we have wasm related:
//   - engine.rs  : Engine core + resource structures
//   - browser.rs : webassembly html + canvas bindings

#[macro_use]
mod browser;
mod engine;
mod game;
mod sprite;

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
