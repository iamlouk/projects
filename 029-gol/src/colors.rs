use crate::CACell;
use rand::Rng;

pub type Cell = (bool, f32, f32, f32);

impl CACell for Cell {
    fn init(&mut self, rng: &mut rand::prelude::ThreadRng, _x: i64, _y: i64) {
        if rng.gen_bool(0.999) {
            *self = (false, 0., 0., 0.);
            return;
        }

        *self = (
            true,
            rng.gen_range(0f32..1.),
            rng.gen_range(0f32..1.),
            rng.gen_range(0f32..1.),
        )
    }

    fn render(&self) -> sdl2::pixels::Color {
        sdl2::pixels::Color::RGB(
            (self.1 * 255.) as u8,
            (self.2 * 255.) as u8,
            (self.3 * 255.) as u8,
        )
    }

    fn update(
        &self,
        x: i64,
        y: i64,
        get_cell: impl Fn(i64, i64) -> Cell,
        _: &mut rand::prelude::ThreadRng,
    ) -> Cell {
        if self.0 {
            return *self;
        }

        let (active1, r1, g1, b1) = get_cell(x, y - 1);
        let (active2, r2, g2, b2) = get_cell(x, y + 1);
        let (active3, r3, g3, b3) = get_cell(x - 1, y);
        let (active4, r4, g4, b4) = get_cell(x + 1, y);
        let num_active = active1 as i32 + active2 as i32 + active3 as i32 + active4 as i32;
        if num_active == 0 {
            return *self;
        }

        let f = (4 - num_active) as f32 * 0.25 * 1.01;
        let r = (r1 + r2 + r3 + r4) * f;
        let g = (g1 + g2 + g3 + g4) * f;
        let b = (b1 + b2 + b3 + b4) * f;
        (true, r, g, b)
    }
}
