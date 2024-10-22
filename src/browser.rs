use anyhow::{anyhow, Result};
use wasm_bindgen::JsCast; // TODO: Explain why rustanalyzer can't auto import?
#[rustfmt::skip]
use web_sys::{
    Document, 
    Window,
    HtmlCanvasElement,
    CanvasRenderingContext2d,
};

// ==================== Constants ====================
// Constants related to HTML elements
mod html {
    pub const CANVAS_ID: &str = "canvas";
    pub const CONTEXT_2D: &str = "2d";
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

macro_rules! log {
    ($($t:tt)*) => {
        web_sys::console::log_1(&format!($($t)*).into());
    }
}
