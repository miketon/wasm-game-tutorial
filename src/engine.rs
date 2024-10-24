use crate::browser;
use futures::channel::oneshot::channel;
use std::rc::Rc;
use std::sync::Mutex;
#[rustfmt::skip]
use web_sys::HtmlImageElement;
use anyhow::{anyhow, Error, Result};
#[rustfmt::skip]
use wasm_bindgen::{
    JsCast, 
    JsValue, // NOTE: Explain how come unchecked_ref needs this?
};

pub async fn load_image(source: &str) -> Result<HtmlImageElement> {
    let image = browser::new_image()?;
    let (tx, rx) = channel::<Result<(), Error>>();
    let success_tx = Rc::new(Mutex::new(Some(tx)));
    let error_tx = success_tx.clone();

    let success_callback = browser::closure_once(move || {
        if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
            let _ = success_tx.send(Ok(()));
        }
    });

    let error_callback = browser::closure_once(move |err: JsValue| {
        if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
            let _ = error_tx.send(Err(anyhow!("Error loading image : {:#?}", err)));
        }
    });

    image.set_onload(Some(success_callback.as_ref().unchecked_ref()));
    image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));
    image.set_src(source);

    // keep callback alive until image is loaded or errors
    success_callback.forget();
    error_callback.forget();

    // NOTE: Explain double ? unwrap
    rx.await??;

    Ok(image)
}
