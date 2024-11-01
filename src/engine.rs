use crate::browser;
use anyhow::{anyhow, Error, Result};
// ELI5: web assembly is a single threaded environment, so Rc RefCell > Mutex
use async_trait::async_trait;
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

#[async_trait(?Send)]
pub trait Game {
    async fn initialize(&self) -> Result<Box<dyn Game>>;
    fn update(&mut self);
    fn draw(&self, context: &Renderer);
}

// length of a frame in milliseconds
const FRAME_SIZE: f32 = 1.0 / 60.0 * 1000.0;

pub struct GameLoop {
    last_frame: f64,
    accumulated_delta: f32,
}

type SharedLoopClosure = Rc<RefCell<Option<browser::LoopClosure>>>;

impl GameLoop {
    pub async fn start(game: impl Game + 'static) -> Result<()> {
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
            game_loop.accumulated_delta += (perf - game_loop.last_frame) as f32;
            while game_loop.accumulated_delta > FRAME_SIZE {
                game.update();
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

pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
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
