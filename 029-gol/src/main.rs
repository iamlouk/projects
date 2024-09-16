#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_range_loop)]

use std::i64;

use rayon::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

mod colors;
mod continous;
mod forestfire;
mod game_of_living;

pub trait CA<Cell: Clone + Default> {
    fn initialize(&self, rng: &mut rand::prelude::ThreadRng, x: i64, y: i64) -> Cell;
    fn render(&self, cell: Cell) -> sdl2::pixels::Color;
    fn update(
        &self,
        x: i64,
        y: i64,
        cell: Cell,
        get_cell: impl Fn(i64, i64) -> Cell,
        rng: &mut rand::prelude::ThreadRng,
    ) -> Cell;
}

const CELLS_X: u32 = 300;
const CELLS_Y: u32 = 300;
const PIXEL_SIZE: u32 = 1;

pub fn run<CACell: Clone + Copy + Default + Send + Sync, CAImpl: CA<CACell> + Sync>(ca: CAImpl) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Rust CA", CELLS_X * PIXEL_SIZE, CELLS_Y * PIXEL_SIZE)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut cells_prev = Some(Box::new(
        [[CACell::default(); CELLS_Y as usize]; CELLS_X as usize],
    ));
    let mut cells_next = Some(Box::new(
        [[CACell::default(); CELLS_Y as usize]; CELLS_X as usize],
    ));

    {
        let mut rng = rand::thread_rng();
        let cells = cells_next.as_mut().unwrap();
        for i in 0..(CELLS_X as i64) {
            for j in 0..(CELLS_Y as i64) {
                cells[i as usize][j as usize] = ca.initialize(&mut rng, i, j);
            }
        }
    }

    let t0 = std::time::Instant::now();
    let mut frames: u64 = 0;
    let mut events = sdl_context.event_pump().unwrap();
    'mainloop: loop {
        for e in events.poll_iter() {
            match e {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'mainloop;
                }
                _ => {}
            }
        }

        // render:
        {
            canvas.set_draw_color(Color::BLACK);
            let mut prev_col = Color::BLACK;
            let cells = cells_next.as_ref().unwrap();
            for i in 0..(CELLS_X as usize) {
                for j in 0..(CELLS_Y as usize) {
                    let c = ca.render(cells[i][j]);
                    if prev_col != c {
                        canvas.set_draw_color(c);
                        prev_col = c;
                    }
                    canvas
                        .fill_rect(Rect::new(
                            (i * PIXEL_SIZE as usize) as i32,
                            (j * PIXEL_SIZE as usize) as i32,
                            PIXEL_SIZE,
                            PIXEL_SIZE,
                        ))
                        .ok()
                        .unwrap();
                }
            }

            canvas.present();
        }

        // update:
        {
            fn wrap_idxs(mut i: i64, mut j: i64) -> (usize, usize) {
                if i < 0 {
                    i += CELLS_X as i64;
                } else if i >= CELLS_X as i64 {
                    i -= CELLS_X as i64;
                }
                if j < 0 {
                    j += CELLS_Y as i64;
                } else if j >= CELLS_Y as i64 {
                    j -= CELLS_Y as i64;
                }
                (i as usize, j as usize)
            }

            let old_state: Box<_> = cells_next.take().unwrap();
            let mut new_state: Box<_> = cells_prev.take().unwrap();

            new_state.par_iter_mut().enumerate().for_each(|(i, row)| {
                let i = i as i64;
                let mut rng = rand::thread_rng();
                for j in 0..(CELLS_Y as i64) {
                    let old_cell = old_state[i as usize][j as usize];
                    let new_cell = ca.update(
                        i,
                        j,
                        old_cell,
                        |i, j| {
                            let (i, j) = wrap_idxs(i, j);
                            old_state[i][j]
                        },
                        &mut rng,
                    );
                    row[j as usize] = new_cell;
                }
            });
            cells_prev = Some(old_state);
            cells_next = Some(new_state);
        }

        frames += 1;
        // app.update();
    }

    let duration = std::time::Instant::now() - t0;
    eprintln!("avg. FPS: {}", (frames as f64) / duration.as_secs_f64());
}

fn main() {
    /*
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()
        .unwrap();
    */

    run::<forestfire::Cell, forestfire::ForestFire>(forestfire::ForestFire);
    // run::<colors::Cell, colors::Colors>(colors::Colors);
    // run::<continous::Cell, continous::Continous>(continous::Continous);
    // run::<game_of_living::Cell, game_of_living::GameOfLiving>(game_of_living::GameOfLiving);
}
