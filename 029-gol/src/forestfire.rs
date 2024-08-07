use rand::Rng;

use crate::CACell;

#[derive(Default, PartialEq, Clone, Copy)]
pub enum Cell {
    #[default]
    Empty,
    Sappling,
    Tree,
    Fire,
    Burning1,
    Burning2,
    Burning3,
}

impl CACell for Cell {
    fn init(&mut self, rng: &mut rand::prelude::ThreadRng, _x: i64, _y: i64) {
        *self = match rng.gen_bool(0.001) {
            true => Cell::Sappling,
            false => Cell::Empty,
        }
    }

    fn update(
        &self,
        x: i64,
        y: i64,
        get_cell: impl Fn(i64, i64) -> Cell,
        rng: &mut rand::prelude::ThreadRng,
    ) -> Cell {
        match self {
            Cell::Fire => Cell::Burning1,
            Cell::Sappling => Cell::Tree,
            Cell::Burning1 => Cell::Burning2,
            Cell::Burning2 => Cell::Burning3,
            Cell::Burning3 => Cell::Empty,
            Cell::Empty => match rng.gen_ratio(1, 500) {
                true => Cell::Sappling,
                false => Cell::Empty,
            },
            Cell::Tree => {
                let fire = (get_cell(x - 1, y) == Cell::Fire)
                    | (get_cell(x + 1, y) == Cell::Fire)
                    | (get_cell(x, y - 1) == Cell::Fire)
                    | (get_cell(x, y + 1) == Cell::Fire);
                if fire | rng.gen_ratio(1, 10000) {
                    Cell::Fire
                } else {
                    Cell::Tree
                }
            }
        }
    }

    fn render(&self) -> sdl2::pixels::Color {
        use sdl2::pixels::Color;
        match self {
            Cell::Empty => Color::RGB(0x33, 0x33, 0x00),
            Cell::Sappling => Color::RGB(0x33, 0x66, 0x00),
            Cell::Tree => Color::RGB(0x99, 0xbb, 0x00),
            Cell::Fire => Color::RGB(0xb3, 0x00, 0x00),
            Cell::Burning1 => Color::RGB(0xb3, 0x2d, 0x00),
            Cell::Burning2 => Color::RGB(0xb3, 0x59, 0x00),
            Cell::Burning3 => Color::RGB(0xb3, 0x86, 0x00),
        }
    }
}
