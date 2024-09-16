use rand::Rng;

use crate::CA;

pub struct Continous;

pub type Cell = f32;

static mut NEIGHBOURS: [(i64, i64); 216] = [(0, 0); 216];

impl Continous {
    pub fn new() -> Self {
        let mut i = 0;
        for x in (-7)..8 {
            for y in (-7)..8 {
                if (x <= 1 && x >= -1) && (y <= 1 && y >= -1) {
                    continue;
                }

                unsafe {
                    NEIGHBOURS[i] = (x, y);
                }
                i += 1;
            }
        }
        Self {}
    }
}

impl CA<Cell> for Continous {
    fn initialize(&self, rng: &mut rand::prelude::ThreadRng, _x: i64, _y: i64) -> Cell {
        rng.gen_range((-1.)..1.)
    }

    fn render(&self, cell: f32) -> sdl2::pixels::Color {
        let abs = cell.abs();
        sdl2::pixels::Color::RGB(
            (abs * 255.0) as u8,
            (abs * 255.0) as u8,
            (abs * 255.0) as u8,
        )
    }

    fn update(&self, x: i64, y: i64, cell: Cell, get_cell: impl Fn(i64, i64) -> Cell) -> Cell {
        let sum: f32 = unsafe { NEIGHBOURS.iter() }
            .map(|(dx, dy)| get_cell(x + dx, y + dy))
            .sum();
        let avg = sum / unsafe { NEIGHBOURS.len() as f32 };
        let x = match (cell, avg) {
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
