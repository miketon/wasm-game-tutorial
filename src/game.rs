use crate::browser;
use crate::engine;
use crate::engine::KeyState;
use crate::engine::{Game, Point, Rect, Renderer};
use crate::log;
use anyhow::Context;
// browser > lib (root) > this crate
use self::red_hat_boy_states::*;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use web_sys::HtmlImageElement;

const ANIMATION_FRAME_DURATION: u8 = 3;
const TOTAL_ANIMATION_FRAMES: u8 = 23;
const MOVEMENT_SPEED: i16 = 3;
const CANVAS_WIDTH: f32 = 600.0;
const CANVAS_HEIGHT: f32 = 600.0;

/// Walk The Dog : Game Trait implementation
/// - initialize, update and draw
pub struct WalkTheDog {
    image: Option<HtmlImageElement>,
    sheet: Option<Sheet>,
    frame: u8,
    position: Point,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog {
            image: None,
            sheet: None,
            frame: 0,
            position: Point { x: 0, y: 0 },
        }
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        // TODO: Explain how come we are taking self and throwing it a way? :
        // - replacing it with WalkTheDog
        // - thrown on the heap?

        let sheet = browser::fetch_json::<Sheet>("rhb.json")
            .await
            .context("Failed to fetch rhb.json")?;
        let image = engine::load_image("rhb.png")
            .await
            .context("Failed to load rhb.png")?;

        log!("[game.rs::WalkTheDog] initialize");

        Ok(Box::new(WalkTheDog {
            image: Some(image),
            sheet: Some(sheet),
            frame: self.frame,
            position: self.position,
        }))
    }

    fn update(&mut self, keystate: &KeyState) {
        self.frame = (self.frame + 1) % (TOTAL_ANIMATION_FRAMES + 1);

        let mut velocity = Point { x: 0, y: 0 };
        if keystate.is_pressed("ArrowDown") {
            velocity.y += MOVEMENT_SPEED;
        }
        if keystate.is_pressed("ArrowUp") {
            velocity.y -= MOVEMENT_SPEED;
        }
        if keystate.is_pressed("ArrowRight") {
            velocity.x += MOVEMENT_SPEED;
        }
        if keystate.is_pressed("ArrowLeft") {
            velocity.x -= MOVEMENT_SPEED;
        }

        self.position.x += velocity.x;
        self.position.y += velocity.y;
    }

    fn draw(&self, renderer: &Renderer) {
        let current_sprite = (self.frame / ANIMATION_FRAME_DURATION) + 1;
        let frame_name = format!("Run ({}).png", current_sprite);
        let sprite = match self
            .sheet // start with self.sheet (Option<Sheet>)
            .as_ref() // Convert Option<Sheet> to Option<&Sheet>
            // if sheet exists, try to get frame
            .and_then(|sheet| sheet.frames.get(&frame_name))
        {
            Some(sprite) => sprite,
            None => {
                log!("Warning : Sprite not found: {}", frame_name);
                return;
            }
        };
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: CANVAS_WIDTH,
            height: CANVAS_HEIGHT,
        });

        if let Some(image) = self.image.as_ref() {
            renderer.draw_image(
                image,
                // sets frame from sprite to draw
                &Rect {
                    x: sprite.frame.x.into(),
                    y: sprite.frame.y.into(),
                    width: sprite.frame.w.into(),
                    height: sprite.frame.h.into(),
                },
                // sets frame where to draw on canvax
                &Rect {
                    x: self.position.x.into(),
                    y: self.position.y.into(),
                    width: sprite.frame.w.into(),
                    height: sprite.frame.h.into(),
                },
            );
        };
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Cell {
    frame: SheetRect,
}

#[derive(Debug, Deserialize, Serialize)]
struct SheetRect {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}

// #region StateMachines

/// All code relating to individual states are behind this mod block and will
/// enforce unrepresentable states, by making it impossible to reach a state
/// transition without using ONLY the methods provided :
/// - PUBLIC  : RedHatBoyState and RedHatBoyContext struct are public
/// - PRIVATE : internal members are private
mod red_hat_boy_states {
    use crate::engine::Point;

    #[derive(Debug, Copy, Clone)]
    pub struct Idle;

    #[derive(Debug, Copy, Clone)]
    pub struct Running;

    #[derive(Debug, Copy, Clone)]
    pub struct RedHatBoyState<S> {
        context: RedHatBoyContext,
        // TODO: this is never read ... explain why?
        _state: S,
    }

    impl RedHatBoyState<Idle> {
        // TODO: Explain how this taking mut self consumes the current state?
        pub fn run(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context,
                _state: Running {},
            }
        }
    }

    #[derive(Debug, Copy, Clone)]
    /// Context data COMMON to all RedHatBoyState(s)
    pub struct RedHatBoyContext {
        frame: u8,
        position: Point,
        velocity: Point,
    }
}

#[derive(Debug, Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
}

impl From<RedHatBoyState<Running>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Running>) -> Self {
        RedHatBoyStateMachine::Running(state)
    }
}

pub enum Event {
    Run,
}

impl RedHatBoyStateMachine {
    fn transition(self, event: Event) -> Self {
        match (self, event) {
            (RedHatBoyStateMachine::Idle(state), Event::Run) => state.run().into(),
            _ => self,
        }
    }
}

struct RedHatBoy {
    statemachine: RedHatBoyStateMachine,
    sprite_sheet: Sheet,
    image: HtmlImageElement,
}

// #endregion
