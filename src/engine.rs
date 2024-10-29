use crate::browser;
use anyhow::{anyhow, Error, Result};
// ELI5: web assembly is a single threaded environment, so Rc RefCell > Mutex
use futures::channel::oneshot::channel;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{
    // unchecked_ref (unsafe) cast from Javascript type to Rust type
    // - because we control the closure creation and specify the expected type,
    // in principle this should be generally safe (unsafe) code
    JsCast,
    JsValue,
};
use web_sys::{CanvasRenderingContext2d, HtmlImageElement};

pub trait Game {
    fn update(&mut self);
    fn draw(&self, context: &CanvasRenderingContext2d);
}

// length of a frame in milliseconds
const FRAME_SIZE: f32 = 1.0 / 60.0 * 1000.0;

pub struct GameLoop {
    last_frame: f64,
    accumulated_delta: f32,
}

type SharedLoopClosure = Rc<RefCell<Option<browser::LoopClosure>>>;

impl GameLoop {
    pub async fn start(mut game: impl Game + 'static) -> Result<()> {
        let mut game_loop = GameLoop {
            last_frame: browser::now()?,
            accumulated_delta: 0.0,
        };
        let f: SharedLoopClosure = Rc::new(RefCell::new(None));
        let g = f.clone();
        *g.borrow_mut() = Some(browser::create_raf_closure(move |perf: f64| {
            game_loop.accumulated_delta += (perf - game_loop.last_frame) as f32;
            while game_loop.accumulated_delta > FRAME_SIZE {
                game.update();
                game_loop.accumulated_delta -= FRAME_SIZE;
            }
            game_loop.last_frame = perf;
            game.draw(&browser::context().expect("Context should exist"));
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
            let _ = tx.send(Err(anyhow!("Error loading image: {:#?}", err)));
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
