use crate::browser;
use crate::engine;
use crate::engine::input::*;
use crate::engine::{Game, Point, Rect, Renderer};
use crate::log;
// browser > lib (root) > this crate
use self::constants::{animation, canvas, MOVEMENT_SPEED};
use self::red_hat_boy_states::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use web_sys::HtmlImageElement;

mod constants {

    pub mod animation {
        pub const FRAME_DURATION: u8 = 3;
        pub const TOTAL_FRAMES: u8 = 23;
    }

    pub mod canvas {
        pub const WIDTH: f32 = 600.0;
        pub const HEIGHT: f32 = 600.0;
    }

    pub const MOVEMENT_SPEED: i16 = 3;
}

/// Walk The Dog : Game Trait implementation
/// - initialize, update and draw
pub struct WalkTheDog {
    image: Option<HtmlImageElement>,
    sheet: Option<Sheet>,
    frame: u8,
    position: Point,
    rhb: Option<RedHatBoy>,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog {
            image: None,
            sheet: None,
            frame: 0,
            position: Point { x: 0, y: 0 },
            rhb: None,
        }
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        // TODO: Explain how come we are taking self and throwing it a way? :
        // - replacing it with WalkTheDog
        // - thrown on the heap?

        let sheet = Some(browser::fetch_json::<Sheet>("rhb.json").await?);
        let image = Some(engine::load_image("rhb.png").await?);

        log!("[game.rs::WalkTheDog] initialize");

        Ok(Box::new(WalkTheDog {
            image: image.clone(),
            sheet: sheet.clone(),
            frame: self.frame,
            position: self.position,
            rhb: Some(RedHatBoy::new(
                sheet.clone().ok_or_else(|| anyhow!("No Sheet Present"))?,
                image.clone().ok_or_else(|| anyhow!("No Image Present"))?,
            )),
        }))
    }

    // ELI5: Graph `update` delegate from lib.rs > engine.rs > game.rs
    fn update(&mut self, keystate: &KeyState) {
        self.frame = (self.frame + 1) % (animation::TOTAL_FRAMES + 1);
        self.rhb.as_mut().unwrap().update();

        let mut velocity = Point { x: 0, y: 0 };
        if keystate.is_pressed("ArrowDown") {
            velocity.y += MOVEMENT_SPEED;
        }
        if keystate.is_pressed("ArrowUp") {
            velocity.y -= MOVEMENT_SPEED;
        }
        if keystate.is_pressed("ArrowRight") {
            velocity.x += MOVEMENT_SPEED;
            self.rhb.as_mut().unwrap().run_right();
        }
        if keystate.is_pressed("ArrowLeft") {
            velocity.x -= MOVEMENT_SPEED;
        }

        self.position.x += velocity.x;
        self.position.y += velocity.y;
    }

    fn draw(&self, renderer: &Renderer) {
        let current_sprite = (self.frame / animation::FRAME_DURATION) + 1;
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
            width: canvas::WIDTH,
            height: canvas::HEIGHT,
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

        self.rhb.as_ref().unwrap().draw(renderer);
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Cell {
    frame: SheetRect,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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

    // physics
    const FLOOR: i16 = 475;
    const RUNNING_SPEED: i16 = 3;
    // rendering
    const IDLE_NAME: &str = "Idle";
    const RUN_NAME: &str = "Run";
    const IDLE_FRAMES: u8 = 29;
    const RUN_FRAMES: u8 = 23;

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

    /// generic methods shared between all states
    /// - context() -> RedHatBoyContext
    impl<S> RedHatBoyState<S> {
        pub fn context(&self) -> &RedHatBoyContext {
            &self.context
        }
    }

    impl RedHatBoyState<Idle> {
        pub fn new() -> Self {
            RedHatBoyState {
                context: RedHatBoyContext {
                    // ah instead of on_state_transition - explicit frame reset
                    // FIXME: find a way to use on_state_transition
                    frame: 0,
                    position: Point { x: 0, y: FLOOR },
                    velocity: Point { x: 0, y: 0 },
                },
                _state: Idle {},
            }
        }
        // TODO: Explain how this taking mut self consumes the current state?
        pub fn run(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.on_state_transition().run_right(),
                _state: Running {},
            }
        }

        pub fn frame_name(&self) -> &str {
            IDLE_NAME
        }

        pub fn update(&mut self) {
            self.context = self.context.update(IDLE_FRAMES);
        }
    }

    impl RedHatBoyState<Running> {
        pub fn frame_name(&self) -> &str {
            RUN_NAME
        }

        pub fn update(&mut self) {
            self.context = self.context.update(RUN_FRAMES);
        }
    }

    #[derive(Debug, Copy, Clone)]
    /// Shared data to track current :
    /// - frame::draw
    /// - rect::position
    pub struct RedHatBoyContext {
        pub frame: u8,
        pub position: Point,
        pub velocity: Point,
    }

    impl RedHatBoyContext {
        /// RedHadBoyContext::update(self, frame_count)
        /// - update frame_count -> render frame
        /// - update velocity -> position
        pub fn update(mut self, frame_count: u8) -> Self {
            // update render frame
            if self.frame < frame_count {
                self.frame += 1;
            } else {
                self.frame = 0;
            }
            // update transform position
            self.position.x += self.velocity.x;
            self.position.y += self.velocity.y;
            self
        }

        /// Handle state transition :: prevent RUNTIME ERROR
        /// TODO: explain is there a new Self being replaced???
        fn on_state_transition(mut self) -> Self {
            // Reset to frame 0 on transition to prevent runtime ERROR
            // - because each state will variable frame count
            // - else we risk accessing out of index frame => runtime ERROR
            self.frame = 0;
            self
        }

        fn run_right(mut self) -> Self {
            self.velocity.x = RUNNING_SPEED;
            self
        }
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
    fn update(self) -> Self {
        use RedHatBoyStateMachine::*;
        match self {
            Idle(mut state) => {
                state.update();
                Idle(state)
            }
            Running(mut state) => {
                state.update();
                Running(state)
            }
        }
    }

    fn transition(self, event: Event) -> Self {
        use RedHatBoyStateMachine::*;
        match (self, event) {
            (Idle(state), Event::Run) => state.run().into(),
            _ => self,
        }
    }

    fn frame_name(&self) -> &str {
        match self {
            RedHatBoyStateMachine::Idle(state) => state.frame_name(),
            RedHatBoyStateMachine::Running(state) => state.frame_name(),
        }
    }

    // TODO: Find out if this can be simplified with a macro?
    fn context(&self) -> &RedHatBoyContext {
        match self {
            RedHatBoyStateMachine::Idle(state) => state.context(),
            RedHatBoyStateMachine::Running(state) => state.context(),
        }
    }
}

struct RedHatBoy {
    state: RedHatBoyStateMachine,
    sheet: Sheet,
    image: HtmlImageElement,
}

impl RedHatBoy {
    fn new(sheet: Sheet, image: HtmlImageElement) -> Self {
        RedHatBoy {
            state: RedHatBoyStateMachine::Idle(RedHatBoyState::new()),
            sheet,
            image,
        }
    }

    fn update(&mut self) {
        // TODO: Explain why this forces us to derive the state machine as copy?
        // - somehow it consumes self via mut self ??? I don't get it
        self.state = self.state.update();
    }

    fn run_right(&mut self) {
        self.state = self.state.transition(Event::Run);
    }

    fn draw(&self, renderer: &Renderer) {
        let frame_name = format!(
            "{} ({}).png",
            self.state.frame_name(),
            (self.state.context().frame / 3) + 1
        );
        let sprite = self.sheet.frames.get(&frame_name).expect("Cell not found");

        renderer.draw_image(
            &self.image,
            &Rect {
                x: sprite.frame.x.into(),
                y: sprite.frame.y.into(),
                width: sprite.frame.w.into(),
                height: sprite.frame.h.into(),
            },
            &Rect {
                // TODO: Explain why it's ok to diverge from Law of Demeter here
                x: self.position().x.into(),
                y: self.position().y.into(),
                width: sprite.frame.w.into(),
                height: sprite.frame.h.into(),
            },
        );
    }

    // Addresses Law of Demeter
    // - OO style guideline where states should only access their direct
    // nodes, NOT children of those notes
    // - previously we manually called the full path at each entry
    fn position(&self) -> Point {
        self.state.context().position
    }
}

// #endregion
