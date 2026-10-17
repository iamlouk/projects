use rand::Rng;

use crate::CACell;

pub type Cell = f32;

lazy_static::lazy_static! {
    static ref NEIGHBOURS: [(i64, i64); 216] = {
        let mut result = [(0, 0); 216];
        let mut i = 0;
        for x in (-7)..8 {
            for y in (-7)..8 {
                if (x <= 1 && x >= -1) && (y <= 1 && y >= -1) {
                    continue;
                }
                result[i] = (x, y);
                i += 1;
            }
        }
        result
    };
}

impl CACell for Cell {
    fn init(&mut self, rng: &mut rand::prelude::ThreadRng, _x: i64, _y: i64) {
        *self = rng.gen_range(-1.0..1.0);
    }

    fn render(&self) -> sdl2::pixels::Color {
        let abs = self.abs();
        sdl2::pixels::Color::RGB(
            (abs * 255.0) as u8,
            (abs * 255.0) as u8,
            (abs * 255.0) as u8,
        )
    }

    fn update(
        &self,
        x: i64,
        y: i64,
        get_cell: impl Fn(i64, i64) -> Cell,
        _: &mut rand::prelude::ThreadRng,
    ) -> Cell {
        let sum: f32 = NEIGHBOURS
            .iter()
            .map(|(dx, dy)| get_cell(x + dx, y + dy))
            .sum();
        let avg = sum / NEIGHBOURS.len() as f32;
        let x = match (*self, avg) {
            (c, a) if c < 0. && a > -0.2 => -c + 0.025,

            (c, a) if c > 0. && a < 0. => -c - 0.025,
            (c, a) if c > 0. && a > 0.55 => -c - 0.025,

            (c, a) if c < a => c + 0.01,
            (c, a) if a < c => c - 0.01,
            (c, _) => c, // panic!() // (c * 6. + a) / 5.
                         // (c, a) => (c * 10. + a) / 10.
        };

        x.clamp(-1., 1.)
    }
}
