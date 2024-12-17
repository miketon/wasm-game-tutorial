#[cfg(debug_assertions)]
use crate::engine::DebugDraw;
use crate::engine::{Point, Rect, Renderer, Sheet, Size};
use crate::sprite;
use crate::sprite::state::{IsJumping, IsSliding, RedHatBoyContext, RedHatBoyState};
use crate::sprite::{Idle, Jumping, Running, Sliding, SpriteState};
use std::rc::Rc;
use web_sys::HtmlImageElement;

/// ELI5:
/// ┌──────────────── State Transition Flow ──────────────────┐
/// │  From State  →  Event   →  To State                     │
/// ├─────────────────────────────────────────────────────────┤
/// │  Idle        →  Run     →  Running                      │
/// │  Running     →  Slide   →  Sliding                      │
/// │  Running     →  Jump    →  Jumping                      │
/// │  -------        ------                                  │
/// │  Sliding     →  Update  →  Running (when complete)      │
/// │  Jumping     →  Update  →  Running (when landed)        │
/// └─────────────────────────────────────────────────────────┘
pub enum Event {
    Run,
    Slide,
    Jump,
    Update,
}

// PHOTOCOPIER ANALOGY
// +------------------+---------------------------+------------------------+
// | Trait            | Real World                | Rust Example           |
// +------------------+---------------------------+------------------------+
// | Copy             | Carbon Copy Paper         | let x = 5;             |
// |                  | - Automatic               | let y = x;             |
// |                  | - Cheap/Quick             | // Both still usable   |
// |                  | - Only works for          |                        |
// |                  |   simple documents        | // Simple types        |
// +------------------+---------------------------+------------------------+
// | Clone            | Photocopier Machine       | let s = String::new(); |
// |                  | - Manual process          | let c = s.clone();     |
// |                  | - More expensive          | // Explicit copying    |
// |                  | - Works for complex docs  |                        |
// +------------------+---------------------------+------------------------+
//
// FILING CABINET ANALOGY
// +----------------------+-------------------------+-------------------------+
// | Type                 | Real World              | Can implement Copy?     |
// +----------------------+-------------------------+-------------------------+
// | Simple (Copy-able)   | Single Index Card       | YES                     |
// | i32, f64, bool       | - One piece of paper    | Fixed, known size       |
// | char, &str           | - Fixed size            | Stored entirely on stack|
// | (i32, char)          | - Self-contained        |                         |
// +----------------------+-------------------------+-------------------------+
// | Complex (Clone-only) | Folder with References  | NO                      |
// | String, Vec<T>       | - Multiple pages        | Dynamic size            |
// | HashMap<K,V>         | - Can grow/shrink       | Has owned heap data     |
// | Box<T>               | - Has attachments       | Contains pointers       |
// +----------------------+-------------------------+-------------------------+
//
#[derive(Debug, Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Sliding(RedHatBoyState<Sliding>),
    Jumping(RedHatBoyState<Jumping>),
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
        match is_jumping {
            IsJumping::Done(running_state) => running_state.into(),
            IsJumping::InProgress(jumping_state) => jumping_state.into(),
        }
    }
}

impl From<IsSliding> for RedHatBoyStateMachine {
    fn from(is_sliding: IsSliding) -> Self {
        match is_sliding {
            // Type inference works because:
            // - Each variant has a specific type
            // - Into trait implementation exists
            IsSliding::Done(running_state) => running_state.into(),
            IsSliding::InProgress(sliding_state) => sliding_state.into(),
        }
    }
}

impl RedHatBoyStateMachine {
    // Consumption vs reference ----------------------------------------------
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
            // This default arm is necessary because :
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
    // ELI5:
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
    pub fn new(sheet: Sheet, image: HtmlImageElement) -> Self {
        let sheet = Rc::new(sheet);
        let bounding_box_size =
            RedHatBoyStateMachine::get_size_for_state::<crate::sprite::Idle>(&sheet);
        RedHatBoy {
            state: RedHatBoyStateMachine::Idle(RedHatBoyState::new(bounding_box_size)),
            sheet,
            image,
        }
    }

    pub fn update(&mut self) {
        // TODO: Explain why this forces us to derive the state machine as copy?
        // - somehow it consumes self via mut self ??? I don't get it
        self.state = self.state.update();
    }

    pub fn draw(&mut self, renderer: &Renderer) {
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

    pub fn run_right(&mut self) {
        self.state = self.state.transition(Event::Run, Some(&self.sheet));
    }

    pub fn slide(&mut self) {
        self.state = self.state.transition(Event::Slide, Some(&self.sheet));
    }

    pub fn jump(&mut self) {
        self.state = self.state.transition(Event::Jump, Some(&self.sheet));
    }

    // Addresses Law of Demeter
    // - OO style guideline where states should only access their direct
    // nodes, NOT children of those notes
    // - previously we manually called the full path at each entry
    pub fn position(&self) -> Point {
        self.state.context().position
    }

    pub fn bounding_box_size(&self) -> Size {
        self.state.context().bounding_box_size
    }

    pub fn get_current_frame_name(&self) -> String {
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
