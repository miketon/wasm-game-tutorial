use std::future::Future;

use anyhow::{anyhow, Result};
use serde::de::DeserializeOwned;
use wasm_bindgen::closure::{Closure, WasmClosure, WasmClosureFnOnce};
use wasm_bindgen::{JsCast, JsValue}; // TODO: Explain why rustanalyzer can't auto import?
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CanvasRenderingContext2d, Document, HtmlCanvasElement, HtmlImageElement, Response, Window,
};

// ==================== Constants ====================
/// Constants related to browser elements
mod html {
    pub mod canvas {
        pub const ID: &str = "canvas";
        pub const CONTEXT_2D: &str = "2d";
    }
}

pub type LoopClosure = Closure<dyn FnMut(f64)>;

pub fn create_raf_closure(f: impl FnMut(f64) + 'static) -> LoopClosure {
    closure_wrap(Box::new(f))
}

fn closure_wrap<T: WasmClosure + ?Sized>(data: Box<T>) -> Closure<T> {
    Closure::wrap(data)
}

pub fn now() -> Result<f64> {
    Ok(window()?
        .performance()
        .ok_or_else(|| anyhow!("Performance object not found"))?
        .now())
}

pub fn request_animation_frame(callback: &LoopClosure) -> Result<i32> {
    window()?
        .request_animation_frame(callback.as_ref().unchecked_ref())
        .map_err(|err| anyhow!("Cannnot request animation frame {:#?}", err))
}

/// Creates an HTMLImageElement
/// # Errors
/// Returns an error if image element cannot be created
pub fn create_html_image_element() -> Result<HtmlImageElement> {
    HtmlImageElement::new().map_err(|err| anyhow!("Could not create image element : {:#?}", err))
}

pub fn context() -> Result<CanvasRenderingContext2d> {
    // 1) Retrieve the canvas eleement
    canvas()?
        // 2) Get the 2d rendering context from the canvas
        .get_context(html::canvas::CONTEXT_2D)
        // 3) Handle potential errors from 'get_context'
        .map_err(|js_value| anyhow!("Error getting context : {:#?}", js_value))?
        // 4) Handle case where context isn't found ('None')
        .ok_or_else(|| anyhow!("No 2d context found"))?
        // 5) Attempt to dynamically cast context to 'CanvasRenderingContext2d'
        .dyn_into::<CanvasRenderingContext2d>()
        // 6) Handle failed type conversion
        .map_err(|element| {
            anyhow!(
                "Error converting {:#?} to CanvasRenderingContext2d",
                element
            )
        })
}

fn canvas() -> Result<HtmlCanvasElement> {
    document()?
        .get_element_by_id(html::canvas::ID)
        .ok_or_else(|| {
            anyhow!(
                "No Canvas Element found with ID : '{:#?}'",
                html::canvas::ID
            )
        })?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|element| anyhow!("Error converting {:#?} to HtmlCanvasElement", element))
}

fn window() -> Result<Window> {
    web_sys::window().ok_or_else(|| anyhow!("Window not found"))
}

fn document() -> Result<Document> {
    window()?
        .document()
        .ok_or_else(|| anyhow!("No Document Found"))
}

pub fn closure_once<F, A, R>(f: F) -> Closure<F::FnMut>
where
    F: 'static + WasmClosureFnOnce<A, R>,
{
    Closure::once(f)
}

pub fn spawn_local<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

pub async fn fetch_json<T>(json_path: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let resp_value = fetch_with_str(json_path).await?;
    let resp: Response = resp_value
        .dyn_into()
        .map_err(|element| anyhow!("error converting [{:#?}] to Response", element))?;
    let json = resp
        .json()
        .map_err(|err| anyhow!("Could not get JSON from response [{:#?}]", err))?;

    let json_value = JsFuture::from(json)
        .await
        .map_err(|err| anyhow!("error fetching [{:#?}]", err))?;

    serde_wasm_bindgen::from_value(json_value)
        .map_err(|err| anyhow!("error converting response : {:#?}", err))
}

async fn fetch_with_str(resource: &str) -> Result<JsValue> {
    let resp = window()?.fetch_with_str(resource);

    JsFuture::from(resp)
        .await
        .map_err(|err| anyhow!("error fetching : {:#?}", err))
}

// macro_rules! log {
//     ($($t:tt)*) => {
//         web_sys::console::log_1(&format!($($t)*).into());
//     }
// }
