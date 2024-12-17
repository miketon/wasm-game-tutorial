// This is a directory based mod structure
// mod.rs is the entry point for the sprite/ directory (sprite.rs)
// allowing organization of related code into submodules
mod red_hat_boy;
mod state;

// TABLE:
// ┌─────────────── File Structure ──────────────────
// │ sprite/
// ├─ mod.rs           // Public interface
// │  ├─ SpriteState trait
// │  └─ Basic sprite types
// │
// ├─ state.rs         // State management
// │  ├─ Constants
// │  ├─ State types
// │  └─ State implementations
// │
// └─ red_hat_boy.rs   // Character logic
//    ├─ Events
//    ├─ State machine
//    └─ Character implementation
// ┌──────────────────────────────────────────────────────────────────────────┐
// │                      Code Structure vs Photoshop Concepts                │
// ├────────────────┬──────────────────────┬──────────────────────────────────┤
// │   Code File    │   Code Component     │         Photoshop Equivalent     │
// ├────────────────┼──────────────────────┼──────────────────────────────────┤
// │                │ Main module file     │ Master .PSD file                 │
// │   mod.rs       │ SpriteState trait    │ Layer naming/organization rules  │
// │                │ FRAME_TICK_RATE      │ Timeline/Animation settings      │
// ├────────────────┼──────────────────────┼──────────────────────────────────┤
// │                │ struct Idle          │ "Standing_Pose" layer group      │
// │   states.rs    │ struct Running       │ "Running_Animation" layer group  │
// │                │ struct Sliding       │ "Slide_Animation" layer group    │
// │                │ struct Jumping       │ "Jump_Animation" layer group     │
// ├────────────────┼──────────────────────┼──────────────────────────────────┤
// │                │ SpriteMetadata       │ Layer Comp settings              │
// │ red_hat_boy.rs │ frame_count          │ Number of frames in Timeline     │
// │                │ animation_speed      │ Frame delay settings             │
// │                │ default_size         │ Canvas/Artboard dimensions       │
// ├────────────────┼──────────────────────┼──────────────────────────────────┤
// │    lib.rs      │ Project structure    │ Photoshop Project Manager        │
// ├────────────────┼──────────────────────┼──────────────────────────────────┤
// │   game.rs      │ Animation usage      │ Final composition/scene          │
// └────────────────┴──────────────────────┴──────────────────────────────────┘

use crate::engine::Size;
// TODO: Explain why we have to pub export here?
pub use red_hat_boy::RedHatBoy;
use std::num::NonZeroU8;

pub const FRAME_TICK_RATE: u8 = 3;
pub const DEFAULT_SPRITE_SIZE: Size = Size {
    width: 64,
    height: 64,
};

// ELI5:
// ┌─────────────── Animation Frame Sequences ─────────────────┐
// │  State      Frames    Description                         │
// ├────────────┬─────────┬────────────────────────────────────┤
// │  Idle      │ [1-10]  │ Idle stand cycle                   │
// ├────────────┼─────────┼────────────────────────────────────┤
// │  Running   │ [1-8]   │ Looping run cycle                  │
// ├────────────┼─────────┼────────────────────────────────────┤
// │  Sliding   │ [1-5]   │ Slide animation, returns to run    │
// ├────────────┼─────────┼────────────────────────────────────┤
// │  Jumping   │ [1-12]  │ Jump animation, returns to run     │
// └────────────┴─────────┴────────────────────────────────────┘
const IDLE_FRAMES: u8 = 10;
const RUN_FRAMES: u8 = 8;
const SLIDE_FRAMES: u8 = 5;
const JUMP_FRAMES: u8 = 12;

/// SpriteMetaData
/// - frame_count - private initialization via new(frame_count)
/// - animation_speed
/// - default_size (bounding box)
pub struct SpriteMetaData {
    frame_count: NonZeroU8, // private, must be init with new()
    pub animation_speed: u8,
    pub default_size: Size,
}

impl SpriteMetaData {
    pub fn new(frame_count: u8) -> Self {
        Self {
            frame_count: NonZeroU8::new(frame_count).expect("frame_count must be > 0"),
            animation_speed: FRAME_TICK_RATE,
            default_size: DEFAULT_SPRITE_SIZE,
        }
    }
}

pub trait SpriteState {
    // Required methods - must be implemented
    // TODO: Explain is it because we left these blank that they MUST be impl?
    fn name() -> &'static str;
    fn metadata() -> SpriteMetaData;

    // Default methods - shared implementation
    fn frame_key(frame: u8) -> String {
        format!("{} ({}).png", Self::name(), frame)
    }

    fn total_frames() -> u8 {
        // Convert NonZeroU8 to u8 before mathing
        Self::metadata().frame_count.get() * Self::metadata().animation_speed - 1
    }

    fn current_frame_name(frame: u8) -> String {
        format!("{} ({}).png", Self::name(), (frame / FRAME_TICK_RATE + 1))
    }
}

// State specific unit structs can be declared in two ways:
// - pub struct Idle;  // Preferred for marker types, implicit no fields EVER
// - pub struct Idle{} // More explicit, use when fields will be added later
#[derive(Debug, Copy, Clone)]
pub struct Idle;
#[derive(Debug, Copy, Clone)]
pub struct Running;
#[derive(Debug, Copy, Clone)]
pub struct Sliding;
#[derive(Debug, Copy, Clone)]
pub struct Jumping;

impl SpriteState for Idle {
    fn name() -> &'static str {
        "Idle"
    }

    fn metadata() -> SpriteMetaData {
        SpriteMetaData::new(IDLE_FRAMES)
    }
}

impl SpriteState for Running {
    fn name() -> &'static str {
        "Run"
    }

    fn metadata() -> SpriteMetaData {
        SpriteMetaData::new(RUN_FRAMES)
    }
}

impl SpriteState for Sliding {
    fn name() -> &'static str {
        "Slide"
    }

    fn metadata() -> SpriteMetaData {
        SpriteMetaData::new(SLIDE_FRAMES)
    }
}

impl SpriteState for Jumping {
    fn name() -> &'static str {
        "Jump"
    }

    fn metadata() -> SpriteMetaData {
        SpriteMetaData::new(JUMP_FRAMES)
    }
}
