mod utils;

use chip8;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WebIU {
    width: u32,
    height: u32,
    pixel_size: f64,
    ctx: web_sys::CanvasRenderingContext2d,
}

impl chip8::UI for WebIU {
    fn clear_screen(&mut self) {
        self.ctx
            .clear_rect(0., 0., self.width as f64, self.height as f64);
    }

    fn draw_pixel(&mut self, x: usize, y: usize, val: bool) {
        if val {
            self.ctx.fill_rect(
                (x as f64) * self.pixel_size,
                (y as f64) * self.pixel_size,
                self.pixel_size,
                self.pixel_size,
            );
        } else {
            self.ctx.clear_rect(
                (x as f64) * self.pixel_size,
                (y as f64) * self.pixel_size,
                self.pixel_size,
                self.pixel_size,
            );
        }
    }

    fn update(&mut self, _cycle: u64, _dt: std::time::Duration) -> Result<bool, &'static str> {
        Ok(true)
    }

    // TODO:...
    fn is_key_pressed(&mut self, _key: u8) -> bool {
        false
    }
}

#[wasm_bindgen]
pub struct Chip8JSWrapper {
    c8: Box<chip8::Chip8State>,
}

#[wasm_bindgen]
impl Chip8JSWrapper {
    pub fn tick(&mut self) -> Result<(), String> {
        self.c8.cycle().map(|_| ()).map_err(|e| e.to_string())
    }
}

#[wasm_bindgen]
pub fn init_chip8(pixel_size: f64, rom: &[u8]) -> Chip8JSWrapper {
    utils::set_panic_hook();

    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("chip8").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap();
    canvas.set_width(canvas.client_width() as u32);
    canvas.set_height(canvas.client_height() as u32);

    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    ctx.set_fill_style(&JsValue::from_str("white"));

    let ui = Box::new(WebIU {
        pixel_size,
        width: canvas.width(),
        height: canvas.height(),
        ctx,
    });
    let c8 = chip8::Chip8State::new(ui, rom);
    Chip8JSWrapper { c8 }
}
