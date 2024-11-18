use crate::browser;
use crate::engine;
use crate::engine::input::*;
use crate::engine::{Game, Image, Point, Rect, Renderer};
// browser > lib (root) > this crate
use self::red_hat_boy_states::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::join;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use web_sys::HtmlImageElement;

/// TABLE:
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
    // TODO: Explain why lifetime static is needed here???
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
                // TABLE:
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

    fn draw(&self, renderer: &Renderer) {
        if let WalkTheDog::Loaded(walk) = self {
            renderer.clear(&Rect {
                x: 0.0,
                y: 0.0,
                width: 600.0,
                height: 600.0,
            });
            // NOTE: Draw order matters : background -> foreground
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
///
/// Doesn't know about RedHatBoyStateMachine ... TODO: Explain why?
mod red_hat_boy_states {
    use crate::engine::Point;

    // animation timing/tick for playback
    pub const FRAME_TICK_RATE: u8 = 3;

    // physics consts
    const JUMP_SPEED: i16 = -25; // negative because top left is origin
    const GRAVITY: i16 = 1;
    const FLOOR: i16 = 475;
    const RUNNING_SPEED: i16 = 3;

    // sprite consts
    const IDLE_NAME: &str = "Idle";
    const RUN_NAME: &str = "Run";
    const SLIDE_NAME: &str = "Slide";
    const JUMP_NAME: &str = "Jump";
    // actual sprite count as defined by sheet json
    const IDLE_FRAME_COUNT: u8 = 10;
    const RUN_FRAME_COUNT: u8 = 8;
    const SLIDE_FRAME_COUNT: u8 = 5;
    const JUMP_FRAME_COUNT: u8 = 12;
    // sprite count formatted for animation timing/tick
    const IDLE_FRAMES: u8 = IDLE_FRAME_COUNT * FRAME_TICK_RATE - 1;
    const RUN_FRAMES: u8 = RUN_FRAME_COUNT * FRAME_TICK_RATE - 1;
    const SLIDE_FRAMES: u8 = SLIDE_FRAME_COUNT * FRAME_TICK_RATE - 1;
    const JUMP_FRAMES: u8 = JUMP_FRAME_COUNT * FRAME_TICK_RATE - 1;

    pub enum IsJumping {
        Done(RedHatBoyState<Running>),
        InProgress(RedHatBoyState<Jumping>),
    }

    pub enum IsSliding {
        Done(RedHatBoyState<Running>),
        InProgress(RedHatBoyState<Sliding>),
    }

    #[derive(Debug, Copy, Clone)]
    pub struct Idle;

    #[derive(Debug, Copy, Clone)]
    pub struct Running;

    #[derive(Debug, Copy, Clone)]
    pub struct Sliding;

    #[derive(Debug, Copy, Clone)]
    pub struct Jumping;

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

        pub fn frame_name(&self) -> &str {
            IDLE_NAME
        }

        // TODO: explain why we updated this to consume and return the SAME
        // state, given that it's not changing states???
        // - they somehow don't make unnecessary copies because they take
        // ownership of self when called and then return it???
        pub fn update(mut self) -> Self {
            self.context = self.context.update(IDLE_FRAMES);
            self
        }

        pub fn run(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.on_state_transition().run_right(),
                _state: Running {},
            }
        }
    }

    impl RedHatBoyState<Running> {
        pub fn frame_name(&self) -> &str {
            RUN_NAME
        }

        pub fn update(mut self) -> Self {
            self.context = self.context.update(RUN_FRAMES);
            self
        }

        pub fn slide(self) -> RedHatBoyState<Sliding> {
            RedHatBoyState {
                context: self.context.on_state_transition(),
                _state: Sliding {},
            }
        }

        pub fn jump(self) -> RedHatBoyState<Jumping> {
            RedHatBoyState {
                context: self
                    .context()
                    .set_vertical_velocity(JUMP_SPEED)
                    .on_state_transition(),
                _state: Jumping {},
            }
        }
    }

    impl RedHatBoyState<Sliding> {
        pub fn frame_name(&self) -> &str {
            SLIDE_NAME
        }

        // TODO: Explain why this update isn't returning another state here ...
        // Any additional options?
        pub fn update(mut self) -> IsSliding {
            self.context = self.context.update(SLIDE_FRAMES);
            // on every update we check if animation is complete
            if self.context.frame >= SLIDE_FRAMES {
                IsSliding::Done(self.stand())
            } else {
                IsSliding::InProgress(self)
            }
        }

        pub fn stand(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.on_state_transition(),
                _state: Running {},
            }
        }
    }

    impl RedHatBoyState<Jumping> {
        pub fn frame_name(&self) -> &str {
            JUMP_NAME
        }

        pub fn update(mut self) -> IsJumping {
            self.context = self.context.update(JUMP_FRAMES);
            if self.context.position.y >= FLOOR {
                IsJumping::Done(self.land())
            } else {
                IsJumping::InProgress(self)
            }
        }

        pub fn land(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.on_state_transition(),
                _state: Running {},
            }
        }
    }

    #[derive(Debug, Copy, Clone)]
    /// Shared data for :
    /// - physics : position + velocity
    /// - display : state + frame count
    pub struct RedHatBoyContext {
        pub frame: u8,
        pub position: Point,
        pub velocity: Point,
    }

    impl RedHatBoyContext {
        /// ::update per frame
        /// - set frame_count -> render frame
        /// - set velocity -> position
        pub fn update(mut self, frame_count: u8) -> Self {
            // add gravity
            self.velocity.y += GRAVITY;
            // update render frame
            if self.frame < frame_count {
                self.frame += 1;
            } else {
                self.frame = 0;
            }
            // update transform position
            self.position.x += self.velocity.x;
            self.position.y += self.velocity.y;

            // detect collision and resolve
            if self.position.y > FLOOR {
                self.position.y = FLOOR;
            }
            self
        }

        /// ::on_state_transition -> prevent RUNTIME ERROR
        /// Reset to frame 0 on transition :
        /// - because each state will variable frame count
        /// - else we risk accessing out of index frame => runtime ERROR
        fn on_state_transition(mut self) -> Self {
            self.frame = 0;
            self
        }

        fn run_right(mut self) -> Self {
            self.velocity.x = RUNNING_SPEED;
            self
        }

        fn set_vertical_velocity(mut self, y: i16) -> Self {
            self.velocity.y = y;
            self
        }
    }
}

pub enum Event {
    Run,
    Slide,
    Jump,
    Update,
}

#[derive(Debug, Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Sliding(RedHatBoyState<Sliding>),
    Jumping(RedHatBoyState<Jumping>),
}

impl From<RedHatBoyState<Idle>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Idle>) -> Self {
        RedHatBoyStateMachine::Idle(state)
    }
}

impl From<RedHatBoyState<Running>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Running>) -> Self {
        RedHatBoyStateMachine::Running(state)
    }
}

impl From<RedHatBoyState<Sliding>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Sliding>) -> Self {
        RedHatBoyStateMachine::Sliding(state)
    }
}

impl From<RedHatBoyState<Jumping>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Jumping>) -> Self {
        RedHatBoyStateMachine::Jumping(state)
    }
}

impl From<IsJumping> for RedHatBoyStateMachine {
    fn from(is_jumping: IsJumping) -> Self {
        use IsJumping::*;
        match is_jumping {
            Done(running_state) => running_state.into(),
            InProgress(jumping_state) => jumping_state.into(),
        }
    }
}

impl From<IsSliding> for RedHatBoyStateMachine {
    fn from(is_sliding: IsSliding) -> Self {
        use IsSliding::*;
        match is_sliding {
            // TODO: Explain how this code infers :
            // - Complete : RedHatBoyState<Running>
            // - Sliding : RedHatBoyState<Sliding>
            Done(running_state) => running_state.into(),
            InProgress(sliding_state) => sliding_state.into(),
        }
    }
}

impl RedHatBoyStateMachine {
    // CONSUMING self (state instance) and returning a new Self (state)
    // - the `self` passed in as an argument is moved -> no longer accessible
    // - &mut self would return a reference
    // TODO: Explain how to determine when to consume vs referencing
    fn transition(self, event: Event) -> Self {
        use RedHatBoyStateMachine::*;
        match (self, event) {
            (Idle(state), Event::Run) => state.run().into(),
            (Running(state), Event::Slide) => state.slide().into(),
            (Running(state), Event::Jump) => state.jump().into(),
            (Idle(state), Event::Update) => state.update().into(),
            (Running(state), Event::Update) => state.update().into(),
            (Sliding(state), Event::Update) => state.update().into(),
            (Jumping(state), Event::Update) => state.update().into(),
            // TODO: Explain why this doesn't just defeat the point of a well
            // defined match set if we gonna just default here?
            _ => self,
        }
    }

    // TODO: Explain why converting updates into a transition event
    fn update(self) -> Self {
        self.transition(Event::Update)
    }

    fn frame_name(&self) -> &str {
        use RedHatBoyStateMachine::*;
        match self {
            Idle(state) => state.frame_name(),
            Running(state) => state.frame_name(),
            Sliding(state) => state.frame_name(),
            Jumping(state) => state.frame_name(),
        }
    }

    // TODO: Find out if this can be simplified with a macro?
    fn context(&self) -> &RedHatBoyContext {
        use RedHatBoyStateMachine::*;
        match self {
            Idle(state) => state.context(),
            Running(state) => state.context(),
            Sliding(state) => state.context(),
            Jumping(state) => state.context(),
        }
    }
}

pub struct RedHatBoy {
    state: RedHatBoyStateMachine,
    sheet: Sheet,
    image: HtmlImageElement,
}

/// RedHatBoy
/// - update() -> statemachine::update()
/// - handle state transition -> RedHatBoyStateMachine::transition()
///     - run_right() ...
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

    fn slide(&mut self) {
        self.state = self.state.transition(Event::Slide);
    }

    fn jump(&mut self) {
        self.state = self.state.transition(Event::Jump);
    }

    fn draw(&self, renderer: &Renderer) {
        let frame_name = format!(
            "{} ({}).png",
            self.state.frame_name(),
            (self.state.context().frame / red_hat_boy_states::FRAME_TICK_RATE) + 1
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
