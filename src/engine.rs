use crate::browser;
use anyhow::{anyhow, Context, Error, Result};
// ELI5: web assembly is a single threaded environment, so Rc RefCell > Mutex
use async_trait::async_trait;
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::channel::oneshot::channel;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::{
    // unchecked_ref (unsafe) cast from Javascript type to Rust type
    // - because we control the closure creation and specify the expected type,
    // in principle this should be generally safe (unsafe) code
    JsCast,
    JsValue,
};
use web_sys::{CanvasRenderingContext2d, HtmlImageElement, KeyboardEvent};

// length of a frame in milliseconds
const FRAME_SIZE: f32 = 1.0 / 60.0 * 1000.0;

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
    fn new() -> Self {
        KeyState {
            pressed_keys: HashMap::new(),
        }
    }

    pub fn is_pressed(&self, code: &str) -> bool {
        self.pressed_keys.contains_key(code)
    }

    fn set_pressed(&mut self, code: &str, e: KeyboardEvent) {
        // TODO: Explain why .into() on insert, but not contains_key + remove?
        self.pressed_keys.insert(code.into(), e);
    }

    fn set_released(&mut self, code: &str) {
        self.pressed_keys.remove(code);
    }
}

#[async_trait(?Send)]
pub trait Game {
    async fn initialize(&self) -> Result<Box<dyn Game>>;
    fn update(&mut self, keystate: &KeyState);
    fn draw(&self, context: &Renderer);
}

pub struct GameLoop {
    last_frame: f64,
    accumulated_delta: f32,
}

type SharedLoopClosure = Rc<RefCell<Option<browser::LoopClosure>>>;

impl GameLoop {
    pub async fn start(game: impl Game + 'static) -> Result<()> {
        let mut keyevent_receiver = prepare_input()?;
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

        let mut keystate = KeyState::new();
        *g.borrow_mut() = Some(browser::create_raf_closure(move |perf: f64| {
            process_input(&mut keystate, &mut keyevent_receiver);
            game_loop.accumulated_delta += (perf - game_loop.last_frame) as f32;
            while game_loop.accumulated_delta > FRAME_SIZE {
                // TODO: clarify if we are able to also ref keystate here
                // because it's not mutable?
                game.update(&keystate);
                game_loop.accumulated_delta -= FRAME_SIZE;
            }
            game_loop.last_frame = perf;
            game.draw(&renderer);
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

pub struct Renderer {
    context: CanvasRenderingContext2d,
}

impl Renderer {
    pub fn clear(&self, rect: &Rect) {
        self.context.clear_rect(
            rect.x.into(),
            rect.y.into(),
            rect.width.into(),
            rect.height.into(),
        );
    }

    pub fn draw_image(&self, image: &HtmlImageElement, frame: &Rect, destination: &Rect) {
        self.context
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                image,
                frame.x.into(),
                frame.y.into(),
                frame.width.into(),
                frame.height.into(),
                destination.x.into(),
                destination.y.into(),
                destination.width.into(),
                destination.height.into(),
            )
            .expect("Drawing is throwing exceptions! Unrecoverable error");
    }
}

pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
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

/// Prepare Input :
/// - listens for key events (KeyPress)
/// - puts key events into a channel
fn prepare_input() -> Result<UnboundedReceiver<KeyPress>> {
    // TODO: explain unbounded channel
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

    Ok(keyevent_receiver)
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
