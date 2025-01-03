use crate::browser;
use crate::engine::input::*;
use anyhow::{anyhow, Error, Result};
use async_trait::async_trait;
use futures::channel::oneshot::channel;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
// web assembly is a single threaded environment, so Rc RefCell > Mutex
use std::rc::Rc;
use wasm_bindgen::{
    // unchecked_ref (unsafe) cast from Javascript type to Rust type
    // - because we control the closure creation and specify the expected type,
    // in principle this should be generally safe (unsafe) code
    JsCast,
    JsValue,
};
use web_sys::{CanvasRenderingContext2d, HtmlImageElement};

// length of a frame in milliseconds
const FRAME_SIZE: f32 = 1.0 / 60.0 * 1000.0;

/// TABLE:
/// ┌──────────── Game Architecture Overview ──────────────┐
/// │                                                      │
/// │    Browser              Engine              Game     │
/// │  ┌─────────┐         ┌─────────┐          ┌───────┐  │
/// │  │HTML/JS  │◄────────┤GameLoop │◄─────────┤Game   │  │
/// │  │Canvas   │         │RAF      │          │Trait  │  │
/// │  └─────────┘         └─────────┘          └───────┘  │
/// │       ▲                   ▲                    ▲     │
/// │       │                   │                    │     │
/// │  ┌─────────┐         ┌─────────┐          ┌───────┐  │
/// │  │Events   │────────►│Input    │─────────►│Update │  │
/// │  │Keyboard │         │Handler  │          │State  │  │
/// │  └─────────┘         └─────────┘          └───────┘  │
/// └──────────────────────────────────────────────────────┘
#[async_trait(?Send)]
pub trait Game {
    async fn initialize(&self) -> Result<Box<dyn Game>>;
    /// TABLE:
    /// ┌────────────── Input Processing Flow ──────────────────┐
    /// │                                                       │
    /// │ KeyboardEvent                                         │
    /// │     │                                                 │
    /// │     ▼                                                 │
    /// │ KeyPress(enum)        UnboundedReceiver               │
    /// │  ├─KeyUp ─────────────────────┐                       │
    /// │  └─KeyDown                    │                       │
    /// │     │                         │                       │
    /// │     ▼                         ▼                       │
    /// │ InputHandler ──────────► KeyState(HashMap)            │
    /// │     │                    │                            │
    /// │     └──update()──────────┘                            │
    /// │                                                       │
    /// └───────────────────────────────────────────────────────┘
    fn update(&mut self, keystate: &KeyState);
    /// TABLE:
    /// ┌────────────── Animation Frame Flow ──────────────────┐
    /// │                                                      │
    /// │ RAF Closure                                          │
    /// │     │                                                │
    /// │     ▼                                                │
    /// │ Update Input                                         │
    /// │     │                                                │
    /// │     ▼                                                │
    /// │ While Loop                                           │
    /// │  └─► Update Physics (if accumulated_delta > FRAME)   │
    /// │     │                                                │
    /// │     ▼                                                │
    /// │ Draw Frame                                           │
    /// │     │                                                │
    /// │     ▼                                                │
    /// │ Schedule Next Frame                                  │
    /// │                                                      │
    /// └──────────────────────────────────────────────────────┘
    fn draw(&mut self, context: &Renderer);
}

#[derive(Debug)]
pub struct GameLoop {
    last_frame: f64,
    accumulated_delta: f32,
}

type SharedLoopClosure = Rc<RefCell<Option<browser::LoopClosure>>>;

impl GameLoop {
    pub async fn start(game: impl Game + 'static) -> Result<()> {
        let mut input_handler = InputHandler::new()?;

        let mut game = game.initialize().await?;
        let mut game_loop = GameLoop {
            last_frame: browser::now()?,
            accumulated_delta: 0.0,
        };
        let renderer = Renderer {
            // moving this outside of request_animation_frame closure no longer
            // requires us to use the expect() syntax ... nice
            context: browser::context()?,
        };
        let f: SharedLoopClosure = Rc::new(RefCell::new(None));
        let g = f.clone();

        *g.borrow_mut() = Some(browser::create_raf_closure(move |perf: f64| {
            input_handler.update();

            game_loop.accumulated_delta += (perf - game_loop.last_frame) as f32;
            // a) catch up on physics update
            // - multiple updates can occur in a single frame to catch up
            // - doesn't block browser responsiveness via requestAnimationFrame
            // ELI5: why did I think moving draw() inside is more performant?
            while game_loop.accumulated_delta > FRAME_SIZE {
                // TODO: clarify if we are able to also ref keystate here
                // because it's not mutable?
                game.update(input_handler.get_keystate());
                game_loop.accumulated_delta -= FRAME_SIZE;
            }
            // b) draw after while loop updates
            game.draw(&renderer);
            game_loop.last_frame = perf;
            let _ = browser::request_animation_frame(f.borrow().as_ref().unwrap());
        }));

        browser::request_animation_frame(
            g.borrow()
                .as_ref()
                .ok_or_else(|| anyhow!("GameLoop: Loop is None"))?,
        )?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Renderer {
    context: CanvasRenderingContext2d,
}

impl Renderer {
    pub fn clear(&self, rect: &Rect) {
        self.context.clear_rect(
            rect.position.x.into(),
            rect.position.y.into(),
            rect.size.width.into(),
            rect.size.height.into(),
        );
    }

    /// draw_sprite() method :
    /// - image_src: image sheet source to draw from
    /// - frame_id: rect of the current frame from src sheet to draw
    /// - destination : rect of where on canvas to draw image
    pub fn draw_sprite(&self, image_src: &HtmlImageElement, frame_id: &Rect, destination: &Rect) {
        self.context
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                image_src,
                frame_id.position.x.into(),
                frame_id.position.y.into(),
                frame_id.size.width.into(),
                frame_id.size.height.into(),
                destination.position.x.into(),
                destination.position.y.into(),
                destination.size.width.into(),
                destination.size.height.into(),
            )
            .expect("Drawing (draw_sprite) is throwing exceptions! Unrecoverable error");
    }

    pub fn draw_image(&self, image: &HtmlImageElement, position: &Point) {
        self.context
            .draw_image_with_html_image_element(image, position.x.into(), position.y.into())
            .expect("Drawing (draw_entire_image) is throwing exceptions! Unrecoverable error");
    }

    #[cfg(debug_assertions)]
    pub fn draw_bounding_box(&self, bbox: &Rect, color: &str) {
        // Save current context
        self.context.save();
        // Set debug visual style
        self.context.set_stroke_style(&JsValue::from_str(color));
        self.context.set_line_width(2.0);
        // Draw debug bounding box
        self.context.stroke_rect(
            bbox.position.x as f64,
            bbox.position.y as f64,
            bbox.size.width as f64,
            bbox.size.height as f64,
        );
        // Restore original context
        self.context.restore();
    }
}

pub struct Image {
    element: HtmlImageElement,
    position: Point,
    bounding_box: Rect,
}

impl Image {
    pub fn new(element: HtmlImageElement, position: Point) -> Self {
        // TODO: Explain why we couldn't into() and had to as i16 explicitly?
        let bounding_box = Rect::new(
            position,
            Size {
                width: element.width() as i16,
                height: element.height() as i16,
            },
        );
        Self {
            element,
            position,
            bounding_box,
        }
    }

    pub fn draw(&self, renderer: &Renderer) {
        renderer.draw_image(&self.element, &self.position);
        #[cfg(debug_assertions)]
        self.bounding_box.draw_debug(renderer);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: i16,
    pub height: i16,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub position: Point,
    pub size: Size,
}

// TODO: explain perf wise if new bounding box every frame is better than
// - update position on every update
// - width, height on every transition
impl Rect {
    pub fn new(position: Point, size: Size) -> Self {
        Self { position, size }
    }
}

#[cfg(debug_assertions)]
impl DebugDraw for Rect {
    fn draw_debug(&self, renderer: &Renderer) {
        renderer.draw_bounding_box(self, "#00ff00");
    }
}

/// Asynchronously load an image from a given source path
/// # Arguments
/// * `source` - string slice to path/url
/// # Returns
/// * `Ok(HtmlImageElement)` - on load success
/// * `Err` - on load fail
pub async fn load_image(source: &str) -> Result<HtmlImageElement> {
    let image = browser::create_html_image_element()?;
    let (tx, rx) = channel::<Result<(), Error>>();
    let success_tx = Rc::new(RefCell::new(Some(tx)));
    let error_tx = success_tx.clone();

    let success_callback = browser::closure_once(move || {
        if let Some(tx) = success_tx.borrow_mut().take() {
            let _ = tx.send(Ok(()));
        }
    });

    let error_callback = browser::closure_once(move |err: JsValue| {
        if let Some(tx) = error_tx.borrow_mut().take() {
            let _ = tx.send(Err(anyhow!(
                "[engine.rs::load_image] Error loading image: {:#?}",
                err
            )));
        }
    });

    image.set_onload(Some(success_callback.as_ref().unchecked_ref()));
    image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));
    image.set_src(source);

    // keep callback alive until image is loaded or errors
    success_callback.forget();
    error_callback.forget();

    // ?? - double unwrap because Result<Result<(), Error>, oneshot::Canceled>
    // - first unwrap yields channel result : Result<(), Error>
    // - second unwrap yields image load result : () or propagating Error
    rx.await??;

    Ok(image)
}

// ELI5: MEMORY LAYOUT
// ┌─ Sheet ─────────────────────────────────────────────────────────────────┐
// │                                                                         │
// │  frames: HashMap<String, Cell>                                          │
// │  ┌─ Key (String) ─┬─ Value (Cell) ────────────────────────────────┐     │
// │  │                │                                               │     │
// │  │   "idle"       │    frame: SheetRect                           │     │
// │  │                │    ┌────────────┐                             │     │
// │  │                │    │  x: i16    │                             │     │
// │  │                │    │  y: i16    │                             │     │
// │  │                │    │  w: i16    │                             │     │
// │  │                │    │  h: i16    │                             │     │
// │  │                │    └────────────┘                             │     │
// │  └────────────────┴───────────────────────────────────────────────┘     │
// │                                                                         │
// └─────────────────────────────────────────────────────────────────────────┘
//
// MEMORY SIZE BREAKDOWN
// ┌─ Type ─────────┬─ Size ─────┬─ Location ─┬─ Notes ───────────────────────┐
// │ Sheet          │ 24 bytes   │ Stack      │ Contains HashMap pointer      │
// ├────────────────┼────────────┼────────────┼───────────────────────────────┤
// │ HashMap        │ Variable   │ Heap       │ Grows with number of entries  │
// ├────────────────┼────────────┼────────────┼───────────────────────────────┤
// │ String (key)   │ 24 bytes   │ Heap       │ Per key + string content      │
// ├────────────────┼────────────┼────────────┼───────────────────────────────┤
// │ Cell           │ 8 bytes    │ Stack      │ Contains SheetRect            │
// ├────────────────┼────────────┼────────────┼───────────────────────────────┤
// │ SheetRect      │ 8 bytes    │ Stack      │ Four i16 values (2 bytes each)│
// └────────────────┴────────────┴────────────┴───────────────────────────────┘
//
#[derive(Debug, Deserialize, Serialize)]
pub struct Sheet {
    pub frames: HashMap<String, Cell>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Cell {
    pub frame: SheetRect,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SheetRect {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}

pub mod input {
    use crate::browser;
    use anyhow::{Context, Result};
    use futures::channel::mpsc::{unbounded, UnboundedReceiver};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use web_sys::KeyboardEvent;

    #[derive(Debug)]
    /// Because we can't determine what kind of KeyboardEvent is returned :
    /// - this enum wraps the event as a key up or key down
    /// - effectively let's us manage one channel (as opposed to two+)
    enum KeyPress {
        KeyUp(KeyboardEvent),
        KeyDown(KeyboardEvent),
    }

    #[derive(Debug)]
    /// HashMap values represent a generic physical keyboard as defined by :
    /// - https://developer.mozilla.org/en-US/docs/Web/API/UI_Events/Keyboard_event_code_values
    pub struct KeyState {
        pressed_keys: HashMap<String, KeyboardEvent>,
    }

    impl KeyState {
        pub fn new() -> Self {
            KeyState {
                pressed_keys: HashMap::new(),
            }
        }

        pub fn is_pressed(&self, code: &str) -> bool {
            self.pressed_keys.contains_key(code)
        }

        fn set_pressed(&mut self, code: &str, e: KeyboardEvent) {
            // Explain why .into() on insert, but not contains_key + remove?
            // - Hashmap `insert` takes ownership of the key, and into()
            // converts &str to String
            // - `contains_key` and `remove` only reference : into() is unneeded
            self.pressed_keys.insert(code.into(), e);
        }

        fn set_released(&mut self, code: &str) {
            self.pressed_keys.remove(code);
        }
    }

    /// TABLE:
    /// ┌────────────── Input Processing Flow ──────────────────┐
    /// │                                                       │
    /// │ KeyboardEvent                                         │
    /// │     │                                                 │
    /// │     ▼                                                 │
    /// │ KeyPress(enum)        UnboundedReceiver               │
    /// │  ├─KeyUp ─────────────────────┐                       │
    /// │  └─KeyDown                    │                       │
    /// │     │                         │                       │
    /// │     ▼                         ▼                       │
    /// │ InputHandler ──────────► KeyState(HashMap)            │
    /// │     │                    │                            │
    /// │     └──update()──────────┘                            │
    /// └───────────────────────────────────────────────────────┘
    ///
    /// InputHandler encapsulates both :
    /// - keystate: KeyState
    /// - receiver: UnboundedReceiver<KeyPress>
    ///
    /// Provides a cleaner interface and hides implemntation
    /// details of input processing
    pub struct InputHandler {
        keystate: KeyState,
        receiver: UnboundedReceiver<KeyPress>,
    }

    impl InputHandler {
        // a) Self (capital S) refers to the TYPE itself (InputHandler)
        //  - Self in new() is good practice, easier to maintain because it
        //  reduces change, like if the type name changes
        // b) self (lowercase s) refers to an INSTANCE of the type
        pub fn new() -> Result<Self> {
            let (keystate, receiver) = prepare_input()?;
            Ok(InputHandler { keystate, receiver })
        }

        pub fn update(&mut self) {
            process_input(&mut self.keystate, &mut self.receiver);
        }

        pub fn get_keystate(&self) -> &KeyState {
            &self.keystate
        }
    }

    /// Prepare Input :
    /// - listens for key events (KeyPress)
    /// - puts key events into a channel
    fn prepare_input() -> Result<(KeyState, UnboundedReceiver<KeyPress>)> {
        // unbounded() channels have no limits on it buffer size, used here:
        // - we don't expect keyboard events to overflow memory
        // - we process events quickly in each frame
        // - avoiding backpressure handling simplifies the code
        let (keydown_sender, keyevent_receiver) = unbounded();
        let keydown_sender = Rc::new(RefCell::new(keydown_sender));
        let keyup_sender = Rc::clone(&keydown_sender);

        let onkeydown = browser::closure_wrap(Box::new(move |keycode: KeyboardEvent| {
            log!("Key pressed: {}", keycode.key());
            let _ = keydown_sender
                .borrow_mut()
                .start_send(KeyPress::KeyDown(keycode));
        }) as Box<dyn FnMut(KeyboardEvent)>);
        let onkeyup = browser::closure_wrap(Box::new(move |keycode: KeyboardEvent| {
            log!("Key released: {}", keycode.key());
            let _ = keyup_sender
                .borrow_mut()
                .start_send(KeyPress::KeyUp(keycode));
        }) as Box<dyn FnMut(KeyboardEvent)>);

        let window = browser::window().context("Window element not found")?;

        window.set_onkeydown(Some(onkeydown.as_ref().unchecked_ref()));
        window.set_onkeyup(Some(onkeyup.as_ref().unchecked_ref()));

        onkeydown.forget();
        onkeyup.forget();

        Ok((KeyState::new(), keyevent_receiver))
    }

    /// Process Input :
    /// - Grab all events from key press channel
    /// - Reduce them to KeyState
    fn process_input(state: &mut KeyState, keyevent_receiver: &mut UnboundedReceiver<KeyPress>) {
        loop {
            match keyevent_receiver.try_next() {
                Ok(None) => break,
                Err(_err) => break,
                Ok(Some(e)) => match e {
                    KeyPress::KeyUp(e) => state.set_released(&e.code()),
                    KeyPress::KeyDown(e) => state.set_pressed(&e.code(), e),
                },
            };
        }
    }
}

#[cfg(debug_assertions)]
pub trait DebugDraw {
    fn draw_debug(&self, renderer: &Renderer);
}
