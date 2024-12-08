// ==================== Imports ====================
use crate::engine::GameLoop;
use crate::game::WalkTheDog;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[macro_use]
mod browser;
mod engine;
mod game;
// TABLE:
// ┌──────────────────────────────────────────────────────────────────────────┐
// │                 Rust Module Organization Patterns                        │
// ├─────────────────┬────────────────────────┬───────────────────────────────┤
// │     Pattern     │    File Structure      │           Explanation         │
// ├─────────────────┼────────────────────────┼───────────────────────────────┤
// │                 │ src/                   │                               │
// │                 │ ├── lib.rs             │ Single file holds all sprite  │
// │  Single File    │ ├── game.rs            │ related code. Simpler, less   │
// │                 │ └── sprite.rs          │ organized for larger modules  │
// ├─────────────────┼────────────────────────┼───────────────────────────────┤
// │                 │ src/                   │                               │
// │                 │ ├── lib.rs             │ mod.rs serves as sprite.rs but│
// │  Directory with │ ├── game.rs            │ organizes related code        │
// │    mod.rs       │ └── sprite/            │ into submodules within the    │
// │                 │     ├── mod.rs         │ sprite/ directory             │
// │                 │     ├── states.rs      │                               │
// │                 │     └── red_hat_boy.rs │                               │
// ├─────────────────┴────────────────────────┴───────────────────────────────┤
// │                              Key Points                                  │
// ├──────────────────────────────────────────────────────────────────────────┤
// │ • mod.rs and sprite.rs are equivalent - just different org patterns      │
// │ • Directory approach better for modules with multiple related components │
// │ • Single file approach simpler for small, self-contained modules         │
// │ • Both approaches are valid and commonly used in Rust project            │
// └──────────────────────────────────────────────────────────────────────────┘
//   - mod.rs IS sprite.rs wrt to where the code goes
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
