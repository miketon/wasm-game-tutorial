use crate::browser;
use crate::engine;
use crate::engine::input::*;
use crate::engine::Sheet;
#[cfg(debug_assertions)]
use crate::engine::{Game, Image, Point, Rect, Renderer, Size};
use crate::sprite::RedHatBoy;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::join;
use web_sys::HtmlImageElement;

/// TABLE
/// ┌───────────────────── Game Architecture Overview ────────────────────────┐
/// │                                                                         │
/// │                              Update Flow                                │
/// │                                                                         │
/// │    ┌─────────────┐          ┌─────────────┐          ┌─────────────┐    │
/// │    │   lib.rs    │  update  │  engine.rs  │  update  │   game.rs   │    │
/// │    │  GameLoop   ├─────────►│             ├─────────►│  WalkTheDog │    │
/// │    │  update()   │          │  update()   │          │  update()   │    │
/// │    └─────────────┘          └──────┬──────┘          └──────┬──────┘    │
/// │                                    │                        │           │
/// │                              ┌─────┴──────┐            ┌────┴─────┐     │
/// │                              │  KeyState  │            │  Update  │     │
/// │                              │  Keyboard  ├────────────► Game     │     │
/// │                              │  Input     │            │ State    │     │
/// │                              └────────────┘            └──────────┘     │
/// │                                                                         │
/// ├──────────────────────── Call Sequence ──────────────────────────────────┤
/// │                                                                         │
/// │  1. Frame Update Cycle                                                  │
/// │     └─► GameLoop.update() initiates frame processing                    │
/// │                                                                         │
/// │  2. Input Processing                                                    │
/// │     └─► engine.update() captures and processes KeyState                 │
/// │                                                                         │
/// │  3. Game State Update                                                   │
/// │     └─► WalkTheDog.update() manages:                                    │
/// │         ├─► Input Processing: Handle keyboard events                    │
/// │         ├─► Character States: Update animations and positions           │
/// │         ├─► World Updates: Modify game environment                      │
/// │         └─► Collision Detection: Check for object interactions          │
/// │                                                                         │
/// └─────────────────────────────────────────────────────────────────────────┘
pub enum WalkTheDog {
    /// Initialize state while resources are being loaded
    /// Transition to `Loaded` once initialization is complete
    Loading,

    /// Active game state with initialized RedHatBoy assets
    Loaded(Walk),
}

impl WalkTheDog {
    // 'static lifetime is needed because these paths are needed for the entire
    // duration of the program
    // - string literals are implicitly static because they are stored in
    // read-only memory
    const SHEET_PATH: &'static str = "rhb.json";
    const IMAGE_PATH: &'static str = "rhb.png";

    pub fn new() -> Self {
        WalkTheDog::Loading
    }
    async fn load_sprite_sheet() -> Result<Sheet> {
        browser::fetch_json::<Sheet>(Self::SHEET_PATH)
            .await
            .with_context(|| format!("Failed to load sprite sheet from : {}", Self::SHEET_PATH))
    }

    async fn load_sprite_image() -> Result<HtmlImageElement> {
        engine::load_image(Self::IMAGE_PATH).await.with_context(|| {
            format!(
                "Failed to load sprite image resource from : {}",
                Self::IMAGE_PATH
            )
        })
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    // TODO: Explain how returning Game ensures initialized is called ONCE only
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        match self {
            // Key Benefits of Parallel Loading:
            // ┌────────────────────────────────────────────────┐
            // │ ✓ Independent resources load simultaneously    │
            // │ ✓ Total time determined by slowest resource    │
            // └────────────────────────────────────────────────┘
            WalkTheDog::Loading => {
                // ELI5:
                // +------------+----------------------------+----------------+
                // |   Method   |       Resource Time        |   Total Time   |
                // +------------+----------------------------+----------------+
                // |            | Image: 300ms, JSON: 200ms  |                |
                // +------------+----------------------------+----------------+
                // |  Serial    | Image → JSON               | 500ms          |
                // |  Loading   | (One after another)        | (300ms + 200ms)|
                // +------------+----------------------------+----------------+
                // |  Parallel  | Image || JSON              | 300ms          |
                // |  Loading   | (Simultaneous loading)     | (max time wins)|
                // +------------+----------------------------+----------------+
                let (sheet_result, image_result) =
                    join!(Self::load_sprite_sheet(), Self::load_sprite_image(),);
                let sheet = sheet_result?;
                let image = image_result?;
                let background = engine::load_image("BG.png").await?;
                let stone = engine::load_image("Stone.png").await?;
                let rhb = RedHatBoy::new(sheet, image);
                let walk = Walk {
                    boy: rhb,
                    background: Image::new(background, Point { x: 0, y: 0 }),
                    stone: Image::new(stone, Point { x: 150, y: 546 }),
                };
                Ok(Box::new(WalkTheDog::Loaded(walk)))
            }
            WalkTheDog::Loaded(_) => Err(anyhow!("Game is already initialized")),
        }
    }

    fn update(&mut self, keystate: &KeyState) {
        if let WalkTheDog::Loaded(walk) = self {
            // process input and trigger state changes
            if keystate.is_pressed("ArrowRight") {
                walk.boy.run_right();
            }
            if keystate.is_pressed("ArrowDown") {
                walk.boy.slide();
            }
            if keystate.is_pressed("Space") {
                walk.boy.jump();
            }
            walk.boy.update();
        }
    }

    fn draw(&mut self, renderer: &Renderer) {
        if let WalkTheDog::Loaded(walk) = self {
            renderer.clear(&Rect {
                position: Point { x: 0, y: 0 },
                size: Size {
                    width: 600,
                    height: 600,
                },
            });
            // Draw order matters : background -> foreground
            walk.background.draw(renderer);
            walk.boy.draw(renderer);
            walk.stone.draw(renderer);
        }
    }
}

pub struct Walk {
    boy: RedHatBoy,
    background: Image,
    stone: Image,
}
