use crate::browser;
use crate::engine;
use crate::engine::{Game, Rect, Renderer};
use crate::log; // browser > lib (root) > this crate
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use web_sys::HtmlImageElement;

pub struct WalkTheDog {
    image: Option<HtmlImageElement>,
    sheet: Option<Sheet>,
    frame: u8,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog {
            image: None,
            sheet: None,
            frame: 0,
        }
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        // TODO: Explain how come we are taking self and throwing it a way? :
        // - replacing it with WalkTheDog
        // - thrown on the heap?

        let sheet = browser::fetch_json::<Sheet>("rhb.json")
            .await
            .expect("Could not fetch rhb.json");
        let image = engine::load_image("rhb.png")
            .await
            .expect("Could not load rhb.png");

        log!("[initialize] WalkTheDog");

        Ok(Box::new(WalkTheDog {
            image: Some(image),
            sheet: Some(sheet),
            frame: self.frame,
        }))
    }

    fn update(&mut self) {
        if self.frame < 23 {
            self.frame += 1;
        } else {
            self.frame = 0;
        }
    }

    fn draw(&self, renderer: &Renderer) {
        let current_sprite = (self.frame / 3) + 1;
        let frame_name = format!("Run ({}).png", current_sprite);
        let sprite = self
            .sheet // start with self.sheet (Option<Sheet>)
            .as_ref() // Convert Option<Sheet> to Option<&Sheet>
            // if sheet exists, try to get frame
            .and_then(|sheet| sheet.frames.get(&frame_name))
            // panic if cell for frame not found
            .unwrap_or_else(|| panic!("Cell not found : [{}]", frame_name.as_str()));
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 600.0,
        });

        if let Some(image) = self.image.as_ref() {
            renderer.draw_image(
                image,
                &Rect {
                    x: sprite.frame.x.into(),
                    y: sprite.frame.y.into(),
                    width: sprite.frame.w.into(),
                    height: sprite.frame.h.into(),
                },
                &Rect {
                    x: 300.0,
                    y: 300.0,
                    width: sprite.frame.w.into(),
                    height: sprite.frame.h.into(),
                },
            );
        };
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Cell {
    frame: SheetRect,
}

#[derive(Debug, Deserialize, Serialize)]
struct SheetRect {
    x: i16,
    y: i16,
    w: i16,
    h: i16,
}
