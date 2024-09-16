#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_range_loop)]

use rand::Rng;
use rayon::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

static mut NEIGHBOURS: [(isize, isize); 216] = [(0, 0); 216];

const CELLS_X: u32 = 600;
const CELLS_Y: u32 = 600;
const PIXEL_SIZE: u32 = 1;

type Cell = f32;

struct App {
    canvas: sdl2::render::WindowCanvas,
    cells_prev: Option<Box<[[Cell; CELLS_Y as usize]; CELLS_X as usize]>>,
    cells_curr: Option<Box<[[Cell; CELLS_Y as usize]; CELLS_X as usize]>>,

    iter: usize,
}

impl App {
    fn render(&mut self) {
        let cells = self.cells_curr.take().unwrap();
        self.canvas.set_draw_color(Color::BLACK);
        self.canvas.clear();

        let (window_width, window_height) = self.canvas.window().size();
        let (cell_width, cell_height) = (window_width / CELLS_X, window_height / CELLS_Y);

        for i in 0..(CELLS_X as usize) {
            for j in 0..(CELLS_Y as usize) {
                // let cell = cells[i][j];
                let abs = cells[i][j].abs();
                /*
                let color = match cell > 0. {
                    false => Color::RGB((abs * 255.0) as u8, 0, 0),
                    true => Color::RGB(0, (abs * 255.0) as u8, 0),
                };
                */
                let color = Color::RGB(
                    (abs * 255.0) as u8,
                    (abs * 255.0) as u8,
                    (abs * 255.0) as u8,
                );

                let x = (i as i32) * (cell_width as i32);
                let y = (j as i32) * (cell_height as i32);

                self.canvas.set_draw_color(color);
                self.canvas
                    .fill_rect(Rect::new(x, y, cell_width, cell_height))
                    .ok()
                    .unwrap();
            }
        }

        self.canvas.present();
        self.cells_curr = Some(cells);
    }

    fn update(&mut self) {
        fn wrap_idxs(mut i: isize, mut j: isize, di: isize, dj: isize) -> (usize, usize) {
            i += di;
            j += dj;

            if i < 0 {
                i += CELLS_X as isize;
            } else if i >= CELLS_X as isize {
                i -= CELLS_X as isize;
            }
            if j < 0 {
                j += CELLS_Y as isize;
            } else if j >= CELLS_Y as isize {
                j -= CELLS_Y as isize;
            }

            (i as usize, j as usize)
        }

        let cells_prev: Box<_> = self.cells_curr.take().unwrap();
        let mut cells_curr: Box<_> = self.cells_prev.take().unwrap();

        cells_curr.par_iter_mut().enumerate().for_each(|(i, row)| {
            for j in 0..(CELLS_Y as usize) {
                let cell = cells_prev[i][j];
                let sum = unsafe { NEIGHBOURS.iter() }
                    .map(|(di, dj)| wrap_idxs(i as isize, j as isize, *di, *dj))
                    .fold(0., |sum, (x, y)| cells_prev[x][y] + sum);
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

                row[j] = if x > 1. {
                    1.
                } else if x < -1. {
                    -1.
                } else {
                    x
                };
                // row[j] = x / (1. + x * x).sqrt();
            }
        });

        self.cells_prev = Some(cells_prev);
        self.cells_curr = Some(cells_curr);

        self.iter += 1;
    }
}

fn init_neighbours() {
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
}

pub fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()
        .unwrap();

    let window = video_subsystem
        .window("Rust CA", CELLS_X * PIXEL_SIZE, CELLS_Y * PIXEL_SIZE)
        .position_centered()
        .build()
        .unwrap();

    let canvas = window.into_canvas().build().unwrap();

    let mut app = App {
        canvas,
        cells_prev: Some(Box::new([[0.; CELLS_Y as usize]; CELLS_X as usize])),
        cells_curr: Some(Box::new([[0.; CELLS_Y as usize]; CELLS_X as usize])),

        iter: 0,
    };

    init_neighbours();

    let mut rng = rand::thread_rng();
    let cells = &mut app.cells_curr.as_mut().unwrap();
    for i in 0..(CELLS_X as usize) {
        for j in 0..(CELLS_Y as usize) {
            cells[i][j] = rng.gen_range((-1.)..1.);
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

        app.render();
        frames += 1;
        app.update();
    }

    let duration = std::time::Instant::now() - t0;
    eprintln!("avg. FPS: {}", (frames as f64) / duration.as_secs_f64());
}
