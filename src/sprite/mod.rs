// TABLE:
// ┌──────────────────────────────────────────────────────────────────────────┐
// │                      Directory Structure Analogy                         │
// ├───────────────────┬──────────────────────────────────────────────────────┤
// │ Code Directory    │          Photoshop Equivalent                        │
// ├───────────────────┼──────────────────────────────────────────────────────┤
// │ src/              │ Project Root                                         │
// │ ├── lib.rs        │ Project Manager/Asset Organization                   │
// │ ├── game.rs       │ Main Composition Where Animations Are Used           │
// │ └── sprite/       │ Character Asset Library                              │
// │     ├── mod.rs    │ Master Sprite Sheet Settings (.psd)                  │
// │     ├── states.rs │ Animation Sequences (Layer Groups)                   │
// │     └── red_hat_  │ Character-Specific Settings (Layer Comps)            │
// │         boy.rs    │                                                      │
// └───────────────────┴──────────────────────────────────────────────────────┘
// - @src/ in addition to game.rs and lib.rs we have wasm related:
//   - browser.rs
//   - engine.rs
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

// TODO: explain what is happening here
// -- this mod.rs is sprite.rs but as a diretory?
pub mod red_hat_boy;
