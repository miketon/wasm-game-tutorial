use self::red_hat_boy_states::{IsJumping, IsSliding, RedHatBoyContext, RedHatBoyState};
use crate::browser;
use crate::engine;
use crate::engine::input::*;
#[cfg(debug_assertions)]
use crate::engine::DebugDraw;
use crate::engine::{Game, Image, Point, Rect, Renderer, Size};
use crate::sprite::{self, SpriteState};
use ::std::rc::Rc;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::join;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// All code relating to individual states are behind this mod block and will
/// enforce unrepresentable states, by making it impossible to reach a state
/// transition without using ONLY the methods provided :
/// - PUBLIC  : RedHatBoyState and RedHatBoyContext struct are public
/// - PRIVATE : internal members are private
///
/// Doesn't know about RedHatBoyStateMachine ... TODO: Explain why?
mod red_hat_boy_states {
    use crate::engine::{Point, Size};
    use crate::sprite::{self, SpriteState};

    // physics consts
    const JUMP_SPEED: i16 = -25; // negative because top left is origin
    const GRAVITY: i16 = 1;
    const FLOOR: i16 = 475;
    const RUNNING_SPEED: i16 = 3;

    pub enum IsJumping {
        Done(RedHatBoyState<sprite::Running>),
        InProgress(RedHatBoyState<sprite::Jumping>),
    }

    pub enum IsSliding {
        Done(RedHatBoyState<sprite::Running>),
        InProgress(RedHatBoyState<sprite::Sliding>),
    }

    #[derive(Debug, Copy, Clone)]
    pub struct RedHatBoyState<S> {
        context: RedHatBoyContext,
        // _state is used for type-level tracking (phantom type)
        // - it's only purpose is to differentiate between states at compile
        // time, preventing invalid state transitions
        // - it's never read, so we underscored _state
        _state: S,
    }

    /// generic methods shared between all states
    /// - context() -> RedHatBoyContext
    impl<S> RedHatBoyState<S> {
        pub fn context(&self) -> &RedHatBoyContext {
            &self.context
        }
    }

    impl RedHatBoyState<sprite::Idle> {
        pub fn new(bounding_box_size: Size) -> Self {
            let position = Point { x: 0, y: FLOOR };
            RedHatBoyState {
                context: RedHatBoyContext {
                    frame: 0,
                    position,
                    velocity: Point { x: 0, y: 0 },
                    bounding_box_size,
                },
                _state: sprite::Idle {},
            }
        }

        pub fn update(mut self) -> Self {
            self.context = self.context.update(sprite::Idle::total_frames());
            self
        }

        pub fn run(self, size: Size) -> RedHatBoyState<sprite::Running> {
            RedHatBoyState {
                context: self
                    .context
                    .on_state_transition()
                    .run_right()
                    .with_bounding_box_size(size),
                _state: sprite::Running {},
            }
        }
    }

    impl RedHatBoyState<sprite::Running> {
        pub fn update(mut self) -> Self {
            self.context = self.context.update(sprite::Running::total_frames());
            self
        }

        pub fn slide(self, size: Size) -> RedHatBoyState<sprite::Sliding> {
            RedHatBoyState {
                context: self
                    .context()
                    .on_state_transition()
                    .with_bounding_box_size(size),
                _state: sprite::Sliding {},
            }
        }

        pub fn jump(self, size: Size) -> RedHatBoyState<sprite::Jumping> {
            RedHatBoyState {
                context: self
                    .context()
                    .set_vertical_velocity(JUMP_SPEED)
                    .on_state_transition()
                    .with_bounding_box_size(size),
                _state: sprite::Jumping {},
            }
        }
    }

    impl RedHatBoyState<sprite::Sliding> {
        /// Returns an enum because Sliding can:
        /// - End      (Done)
        /// - Continue (InProgress)
        pub fn update(mut self) -> IsSliding {
            self.context = self.context.update(sprite::Sliding::total_frames());
            // on every update we check if animation is complete
            if self.context.frame >= sprite::Sliding::total_frames() {
                IsSliding::Done(self.stand())
            } else {
                IsSliding::InProgress(self)
            }
        }

        pub fn stand(self) -> RedHatBoyState<sprite::Running> {
            RedHatBoyState {
                context: self.context.on_state_transition(),
                _state: sprite::Running {},
            }
        }
    }

    impl RedHatBoyState<sprite::Jumping> {
        pub fn update(mut self) -> IsJumping {
            self.context = self.context.update(sprite::Jumping::total_frames());
            if self.context.position.y >= FLOOR {
                IsJumping::Done(self.land())
            } else {
                IsJumping::InProgress(self)
            }
        }

        pub fn land(self) -> RedHatBoyState<sprite::Running> {
            RedHatBoyState {
                context: self.context.on_state_transition(),
                _state: sprite::Running {},
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
        pub bounding_box_size: Size,
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

        /// ::on_state_transition -> we must :
        /// - ELI5: prevent RUNTIME ERROR
        ///     - Reset to frame 0 on transition :
        ///         - because each state will variable frame count
        ///         - else we risk accessing out of index frame => runtime ERROR
        fn on_state_transition(mut self) -> Self {
            // reset frame
            self.frame = 0;
            self
        }

        /// update bounding box size field
        fn with_bounding_box_size(mut self, size: Size) -> Self {
            self.bounding_box_size = size;
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
    Idle(RedHatBoyState<sprite::Idle>),
    Running(RedHatBoyState<sprite::Running>),
    Sliding(RedHatBoyState<sprite::Sliding>),
    Jumping(RedHatBoyState<sprite::Jumping>),
}

impl From<RedHatBoyState<sprite::Idle>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<sprite::Idle>) -> Self {
        RedHatBoyStateMachine::Idle(state)
    }
}

impl From<RedHatBoyState<sprite::Running>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<sprite::Running>) -> Self {
        RedHatBoyStateMachine::Running(state)
    }
}

impl From<RedHatBoyState<sprite::Sliding>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<sprite::Sliding>) -> Self {
        RedHatBoyStateMachine::Sliding(state)
    }
}

impl From<RedHatBoyState<sprite::Jumping>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<sprite::Jumping>) -> Self {
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
            // Type inference works because:
            // - Each variant has a specific type
            // - Into trait implementation exists
            Done(running_state) => running_state.into(),
            InProgress(sliding_state) => sliding_state.into(),
        }
    }
}

impl RedHatBoyStateMachine {
    // ELI5: consumption vs reference
    // [Consume] when:
    // - operation fundamentally transforms the object (state transition)
    // - ensure old state can't be accessed
    // - operation needs exclusive access to all fields
    // [Reference] when:
    // - operation only needs to read data
    // - multiple references might be needed
    // - operation makes temporary modification
    //
    // CONSUMING self (state instance) and returning a new Self (state)
    // - the `self` passed in as an argument is moved -> no longer accessible
    // - &mut self would return a reference
    fn transition(self, event: Event, sheet: Option<&Sheet>) -> Self {
        use RedHatBoyStateMachine::*;
        match (self, event) {
            (Idle(state), Event::Run) => {
                let size = Self::get_size_for_state::<crate::sprite::Running>(
                    sheet.expect("Sheet not found"),
                );
                state.run(size).into()
            }
            (Running(state), Event::Slide) => {
                let size = Self::get_size_for_state::<crate::sprite::Sliding>(
                    sheet.expect("Sheet not found"),
                );
                state.slide(size).into()
            }
            (Running(state), Event::Jump) => {
                let size = Self::get_size_for_state::<crate::sprite::Jumping>(
                    sheet.expect("Sheet not found"),
                );
                state.jump(size).into()
            }
            (Idle(state), Event::Update) => state.update().into(),
            (Running(state), Event::Update) => state.update().into(),
            (Sliding(state), Event::Update) => state.update().into(),
            (Jumping(state), Event::Update) => state.update().into(),
            // ELI5: This default arm is necessary because :
            // - handles invalid state transitions(e.g. trying to Jump while Sliding)
            // - maintains the current state for unsupported transitions
            // - defensive programming practice for future state additions
            _ => self,
        }
    }

    fn get_size_for_state<S: SpriteState>(sheet: &Sheet) -> Size {
        let frame_key = S::frame_key(1);
        sheet
            .frames
            .get(&frame_key)
            .map(|cell| Size {
                width: cell.frame.w,
                height: cell.frame.h,
            })
            .unwrap_or_else(|| {
                log!("Warning: Missing sprite data for state: {}", S::name());
                S::metadata().default_size
            })
    }

    fn update(self) -> Self {
        // updates() are transitions(Event::Update,) because :
        // - unified state transition mechanism
        // - consistend handling of state changes
        // - simpler state machine logic
        self.transition(Event::Update, None)
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
    // update to reference to eliminate cloning for memory perf improvement :
    // TABLE:
    // ┌─────────────── Prevous Memory Impact ───────────────────────────┐
    // │ ┌──────────────┐      ┌─────────────┐     ┌────────────────┐    │
    // │ │  RedHatBoy   │      │    Sheet    │     │   HashMap      │    │
    // │ │              │ owns │ frames:     │owns │ String -> Cell │    │
    // │ │              ├─────►│ HashMap     ├────►│                │    │
    // │ └──────────────┘      └─────────────┘     └────────────────┘    │
    // │                                                                 │
    // │ Memory Overhead:                                                │
    // │ - Each Sheet instance contains a full copy of the HashMap       │
    // │ - HashMap cloning duplicates all String keys and Cell values    │
    // └─────────────────────────────────────────────────────────────────┘
    // ┌─────────────── Updated Memory Impact ───────────────────────────┐
    // │ ┌──────────────┐                                                │
    // │ │  RedHatBoy1  │      ┌─────────────┐                           │
    // │ │  Rc<Sheet>   │──┐   │    Sheet    │                           │
    // │ └──────────────┘  │   │ (Shared)    │                           │
    // │                   ├──►│             │                           │
    // │ ┌──────────────┐  │   │             │                           │
    // │ │  RedHatBoy2  │──┘   │             │                           │
    // │ │  Rc<Sheet>   │      │             │                           │
    // │ └──────────────┘      └─────────────┘                           │
    // └─────────────────────────────────────────────────────────────────┘
    sheet: Rc<Sheet>,
    image: HtmlImageElement,
}

/// RedHatBoy
/// - update() -> statemachine::update()
/// - handle state transition -> RedHatBoyStateMachine::transition()
///     - run_right() ...
impl RedHatBoy {
    fn new(sheet: Sheet, image: HtmlImageElement) -> Self {
        let sheet = Rc::new(sheet);
        let bounding_box_size =
            RedHatBoyStateMachine::get_size_for_state::<crate::sprite::Idle>(&sheet);
        RedHatBoy {
            state: RedHatBoyStateMachine::Idle(RedHatBoyState::new(bounding_box_size)),
            sheet,
            image,
        }
    }

    fn update(&mut self) {
        // TODO: Explain why this forces us to derive the state machine as copy?
        // - somehow it consumes self via mut self ??? I don't get it
        self.state = self.state.update();
    }

    fn draw(&mut self, renderer: &Renderer) {
        let frame_name = self.get_current_frame_name();
        let sprite = self.sheet.frames.get(&frame_name).expect("Cell not found");

        renderer.draw_sprite(
            &self.image,
            &Rect {
                position: Point {
                    x: sprite.frame.x,
                    y: sprite.frame.y,
                },
                size: Size {
                    width: sprite.frame.w,
                    height: sprite.frame.h,
                },
            },
            &Rect {
                position: Point {
                    x: self.position().x,
                    y: self.position().y,
                },
                size: Size {
                    width: sprite.frame.w,
                    height: sprite.frame.h,
                },
            },
        );

        #[cfg(debug_assertions)]
        {
            let bounding_box = Rect::new(self.position(), self.bounding_box_size());
            bounding_box.draw_debug(renderer);
        }
    }

    fn run_right(&mut self) {
        self.state = self.state.transition(Event::Run, Some(&self.sheet));
    }

    fn slide(&mut self) {
        self.state = self.state.transition(Event::Slide, Some(&self.sheet));
    }

    fn jump(&mut self) {
        self.state = self.state.transition(Event::Jump, Some(&self.sheet));
    }

    // Addresses Law of Demeter
    // - OO style guideline where states should only access their direct
    // nodes, NOT children of those notes
    // - previously we manually called the full path at each entry
    fn position(&self) -> Point {
        self.state.context().position
    }

    fn bounding_box_size(&self) -> Size {
        self.state.context().bounding_box_size
    }

    fn get_current_frame_name(&self) -> String {
        use RedHatBoyStateMachine::*;
        // Match state to the correct current SpriteState impl
        match self.state {
            Idle(_) => crate::sprite::Idle::current_frame_name(self.state.context().frame),
            Running(_) => crate::sprite::Running::current_frame_name(self.state.context().frame),
            Sliding(_) => crate::sprite::Sliding::current_frame_name(self.state.context().frame),
            Jumping(_) => crate::sprite::Jumping::current_frame_name(self.state.context().frame),
        }
    }
}
