/// All code relating to individual states are behind this mod block and will
/// enforce unrepresentable states, by making it impossible to reach a state
/// transition without using ONLY the methods provided :
/// - PUBLIC  : RedHatBoyState and RedHatBoyContext struct are public
/// - PRIVATE : internal members are private
///
/// Doesn't know about RedHatBoyStateMachine ... TODO: Explain why?
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
/// Shared data for :
/// - physics : position + velocity
/// - display : state + frame count
pub struct RedHatBoyContext {
    pub frame: u8,
    pub position: Point,
    pub velocity: Point,
    pub bounding_box_size: Size,
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
    /// - WARN: prevent RUNTIME ERROR
    ///     - Reset to frame 0 on transition :
    ///         - because each state will likely have variable frame count
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
