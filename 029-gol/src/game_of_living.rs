use crate::CACell;
use rand::Rng;

#[derive(Clone, Copy, Default)]
pub enum Cell {
    #[default]
    Dead,
    Alive,
}

impl CACell for Cell {
    fn init(&mut self, rng: &mut rand::prelude::ThreadRng, _x: i64, _y: i64) {
        *self = match rng.gen_ratio(1, 3) {
            true => Cell::Alive,
            false => Cell::Dead,
        };
    }

    fn render(&self) -> sdl2::pixels::Color {
        match self {
            Cell::Alive => sdl2::pixels::Color::BLACK,
            Cell::Dead => sdl2::pixels::Color::WHITE,
        }
    }

    fn update(
        &self,
        x: i64,
        y: i64,
        get_cell: impl Fn(i64, i64) -> Cell,
        _: &mut rand::prelude::ThreadRng,
    ) -> Cell {
        const NEIGHBOURS: [(i64, i64); 8] = [
            (-2, 0),
            (-1, -1),
            (0, -2),
            (1, -1),
            (2, 0),
            (1, 1),
            (0, 2),
            (-1, 1),
        ];

        let living_neighbours: i64 = NEIGHBOURS
            .iter()
            .map(|(dx, dy)| match get_cell(x + dx, y + dy) {
                Cell::Alive => 1,
                Cell::Dead => 0,
            })
            .sum();

        match (self, living_neighbours) {
            (Cell::Alive, n) if n < 2 || n > 3 => Cell::Dead,
            (Cell::Dead, 3) => Cell::Alive,
            _ => *self,
        }
    }
}
