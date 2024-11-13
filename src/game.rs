use crate::browser;
use crate::engine;
use crate::engine::input::*;
use crate::engine::{Game, Point, Rect, Renderer};
use crate::log;
// browser > lib (root) > this crate
use self::red_hat_boy_states::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use web_sys::HtmlImageElement;

/// TABLE:
/// ┌───────────────────────────────────────────────────────────┐
/// │                   WalkTheDog Game Update                  │
/// │                                                           │
/// │  ┌─────────────┐        ┌─────────────┐      ┌────────┐   │
/// │  │   lib.rs    │        │  engine.rs  │      │game.rs │   │
/// │  │ GameLoop    ├───────►│   update()  ├─────►│WalkDog │   │
/// │  │  update()   │        │             │      │update()│   │
/// │  └─────────────┘        └─────────────┘      └───┬────┘   │
/// │                                                  │        │
/// │                              ┌──────────────────►│        │
/// │                              │                   │        │
/// │                        ┌─────┴─────┐             │        │
/// │                        │  KeyState │             │        │
/// │                        └───────────┘             ▼        │
/// │                                            Game State     │
/// └───────────────────────────────────────────────────────────┘
///
/// Call hiearchy for update:
/// 1. lib.rs: GameLoop::update() calls engine::update()
/// 2. engine.rs: update() calls game::update() with current KeyState
/// 3. game.rs: WalkTheDog::update() processes inputs and updates game state
pub struct WalkTheDog {
    rhb: Option<RedHatBoy>,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog { rhb: None }
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
            rhb: Some(RedHatBoy::new(
                sheet.clone().ok_or_else(|| anyhow!("No Sheet Present"))?,
                image.clone().ok_or_else(|| anyhow!("No Image Present"))?,
            )),
        }))
    }

    fn update(&mut self, keystate: &KeyState) {
        // RedHatBoy::update animation state
        self.rhb.as_mut().unwrap().update();

        // process input and trigger state changes
        if keystate.is_pressed("ArrowRight") {
            self.rhb.as_mut().unwrap().run_right();
        }

        if keystate.is_pressed("ArrowDown") {
            self.rhb.as_mut().unwrap().slide();
        }
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });
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
///
/// Doesn't know about RedHatBoyStateMachine ... TODO: Explain why?
mod red_hat_boy_states {
    use crate::engine::Point;

    // animation timing/tick for playback
    pub const FRAME_TICK_RATE: u8 = 3;

    // physics consts
    const FLOOR: i16 = 475;
    const RUNNING_SPEED: i16 = 3;

    // sprite consts
    const IDLE_NAME: &str = "Idle";
    const RUN_NAME: &str = "Run";
    const SLIDE_NAME: &str = "Slide";
    // actual sprite count as defined by sheet json
    const IDLE_FRAME_COUNT: u8 = 10;
    const RUN_FRAME_COUNT: u8 = 8;
    const SLIDE_FRAME_COUNT: u8 = 5;
    // sprite count formatted for animation timing/tick
    const IDLE_FRAMES: u8 = IDLE_FRAME_COUNT * FRAME_TICK_RATE - 1;
    const RUN_FRAMES: u8 = RUN_FRAME_COUNT * FRAME_TICK_RATE - 1;
    const SLIDE_FRAMES: u8 = SLIDE_FRAME_COUNT * FRAME_TICK_RATE - 1;

    #[derive(Debug, Copy, Clone)]
    pub struct Idle;

    #[derive(Debug, Copy, Clone)]
    pub struct Running;

    #[derive(Debug, Copy, Clone)]
    pub struct Sliding;

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
    }

    impl RedHatBoyState<Sliding> {
        pub fn frame_name(&self) -> &str {
            SLIDE_NAME
        }

        // TODO: Explain why this update isn't returning another state here ...
        // Any additional options?
        pub fn update(mut self) -> SlideToggled {
            self.context = self.context.update(SLIDE_FRAMES);
            // on every update we check if animation is complete
            if self.context.frame >= SLIDE_FRAMES {
                SlideToggled::Done(self.stand())
            } else {
                SlideToggled::InProgress(self)
            }
        }

        pub fn stand(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.on_state_transition(),
                _state: Running {},
            }
        }
    }

    pub enum SlideToggled {
        Done(RedHatBoyState<Running>),
        InProgress(RedHatBoyState<Sliding>),
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

pub enum Event {
    Run,
    Slide,
    Update,
}

#[derive(Debug, Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Sliding(RedHatBoyState<Sliding>),
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

impl From<SlideToggled> for RedHatBoyStateMachine {
    fn from(slide_state: SlideToggled) -> Self {
        use SlideToggled::*;
        match slide_state {
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
            (Idle(state), Event::Update) => state.update().into(),
            (Running(state), Event::Update) => state.update().into(),
            (Sliding(state), Event::Update) => state.update().into(),
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
        }
    }

    // TODO: Find out if this can be simplified with a macro?
    fn context(&self) -> &RedHatBoyContext {
        use RedHatBoyStateMachine::*;
        match self {
            Idle(state) => state.context(),
            Running(state) => state.context(),
            Sliding(state) => state.context(),
        }
    }
}

struct RedHatBoy {
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
