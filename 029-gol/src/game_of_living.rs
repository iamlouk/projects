use crate::CA;
use rand::Rng;

pub struct GameOfLiving;

impl GameOfLiving {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Clone, Copy, Default)]
pub enum Cell {
    #[default]
    Dead,
    Alive,
}

impl CA<Cell> for GameOfLiving {
    fn initialize(&self, rng: &mut rand::prelude::ThreadRng, _x: i64, _y: i64) -> Cell {
        match rng.gen_range(0..3) {
            0 => Cell::Alive,
            _ => Cell::Dead,
        }
    }

    fn render(&self, cell: Cell) -> sdl2::pixels::Color {
        match cell {
            Cell::Alive => sdl2::pixels::Color::BLACK,
            Cell::Dead => sdl2::pixels::Color::WHITE,
        }
    }

    fn update(
        &self,
        x: i64,
        y: i64,
        cell: Cell,
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

        match (cell, living_neighbours) {
            (Cell::Alive, n) if n < 2 || n > 3 => Cell::Dead,
            (Cell::Dead, 3) => Cell::Alive,
            _ => cell,
        }
    }
}
