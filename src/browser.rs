use anyhow::{anyhow, Result};
use serde::de::DeserializeOwned;
use std::future::Future;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::{
    JsCast,
    JsValue,
}; // TODO: Explain why rustanalyzer can't auto import?
use wasm_bindgen::closure::{
    Closure,
    WasmClosureFnOnce,
};

#[rustfmt::skip]
use web_sys::{
    Document, 
    Window,
    CanvasRenderingContext2d,
    HtmlCanvasElement,
    HtmlImageElement,
    Response,
};

// ==================== Constants ====================
// Constants related to HTML elements
mod html {
    pub const CANVAS_ID: &str = "canvas";
    pub const CONTEXT_2D: &str = "2d";
}

pub fn new_image() -> Result<HtmlImageElement> {
    HtmlImageElement::new()
        .map_err(|err| 
            anyhow!("Could not create image element : {:#?}", err)
        ) 
}

pub fn context() -> Result<CanvasRenderingContext2d> {
    canvas()?
        .get_context(html::CONTEXT_2D)
        // Because return is Result<Option<Object>,JsValue>
        // - we map error(JsValue) to Error (anyhow)
        // - take the inner Option and map the None case to a value
        // NOTE: I'm still confused, explain this in detail please ^
        .map_err(|js_value| anyhow!("Error getting context : {:#?}", js_value))?
        .ok_or_else(|| anyhow!("No 2d context found"))?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|element| {
            anyhow!(
                "Error converting {:#?} to CanvasRenderingContext2d",
                element
            )
        })
}

pub fn canvas() -> Result<HtmlCanvasElement> {
    document()?
        .get_element_by_id(html::CANVAS_ID)
        .ok_or_else(|| anyhow!("No Canvas Element found with ID : '{:#?}'", html::CANVAS_ID))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|element| anyhow!("Error converting {:#?} to HtmlCanvasElement", element))
}

pub fn window() -> Result<Window> {
    web_sys::window().ok_or_else(|| anyhow!("Window not found"))
}

pub fn document() -> Result<Document> {
    window()?
        .document()
        .ok_or_else(|| anyhow!("No Document Found"))
}

pub fn closure_once<F, A, R>(f: F) -> 
    Closure<F::FnMut>
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
    let json = 
    resp.json()
        .map_err(|err| anyhow!("Could not get JSON from response [{:#?}]", err))?;

    let json_value = JsFuture::from(json)
        .await
        .map_err(|err| anyhow!("error fetching [{:#?}]", err))?;

    serde_wasm_bindgen::from_value(json_value)
        .map_err(|err| anyhow!("errro converting response : {:#?}", err))
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
