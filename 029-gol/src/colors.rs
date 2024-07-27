use rand::Rng;

use crate::CA;

pub type Cell = (bool, f32, f32, f32);

pub struct Colors;

impl Colors {
    pub fn new() -> Self {
        Self {}
    }
}

impl CA<Cell> for Colors {
    fn initialize(&self, rng: &mut rand::prelude::ThreadRng, _x: i64, _y: i64) -> Cell {
        if rng.gen_bool(0.999) {
            return (false, 0., 0., 0.);
        }

        (
            true,
            rng.gen_range(0f32..1.),
            rng.gen_range(0f32..1.),
            rng.gen_range(0f32..1.),
        )
    }

    fn render(&self, cell: Cell) -> sdl2::pixels::Color {
        sdl2::pixels::Color::RGB(
            (cell.1 * 255.) as u8,
            (cell.2 * 255.) as u8,
            (cell.3 * 255.) as u8,
        )
    }

    fn update(
        &self,
        x: i64,
        y: i64,
        cell: Cell,
        get_cell: impl Fn(i64, i64) -> Cell,
        _: &mut rand::prelude::ThreadRng,
    ) -> Cell {
        if cell.0 {
            return cell;
        }

        let (active1, r1, g1, b1) = get_cell(x, y - 1);
        let (active2, r2, g2, b2) = get_cell(x, y + 1);
        let (active3, r3, g3, b3) = get_cell(x - 1, y);
        let (active4, r4, g4, b4) = get_cell(x + 1, y);
        let num_active = active1 as i32 + active2 as i32 + active3 as i32 + active4 as i32;
        if num_active == 0 {
            return cell;
        }

        let f = (4 - num_active) as f32 * 0.25 * 1.01;
        let r = (r1 + r2 + r3 + r4) * f;
        let g = (g1 + g2 + g3 + g4) * f;
        let b = (b1 + b2 + b3 + b4) * f;
        (true, r, g, b)
    }
}
