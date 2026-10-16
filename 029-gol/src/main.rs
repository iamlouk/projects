#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_range_loop)]

use std::i64;

use clap::Parser;
use rayon::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;

mod colors;
mod continous;
mod forestfire;
mod game_of_living;

pub trait CACell: Copy + Default + Send + Sync {
    fn init(&mut self, rng: &mut rand::prelude::ThreadRng, x: i64, y: i64);
    fn render(&self) -> sdl2::pixels::Color;
    fn update(
        &self,
        x: i64,
        y: i64,
        get_cell: impl Fn(i64, i64) -> Self,
        rng: &mut rand::prelude::ThreadRng,
    ) -> Self;
}

const CELLS_X: u32 = 450;
const CELLS_Y: u32 = 450;

/// Cellular Automaton type
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CAType {
    /// Game of Life variant
    GameOfLiving,
    /// Forest Fire simulation
    ForestFire,
    /// Color diffusion
    Colors,
    /// Continuous CA
    Continuous,
}

impl Default for CAType {
    fn default() -> Self {
        CAType::Continuous
    }
}

#[derive(Debug, Parser)]
#[command(name = "gol")]
#[command(about = "Cellular Automaton simulation")]
struct Args {
    /// Headless mode (no window, just compute and report FPS)
    #[arg(long)]
    headless: bool,

    /// Number of frames to run in headless mode
    #[arg(long, default_value_t = 1000)]
    frames: u64,

    /// Number of threads for Rayon
    #[arg(long, default_value_t = 4)]
    threads: usize,

    /// CA type to run
    #[arg(long, value_enum, default_value_t = CAType::Continuous)]
    ca: CAType,
}

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

pub fn run<Cell: CACell>() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Rust CA", CELLS_X * 1, CELLS_Y * 1)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut cells_prev = Some(Box::new(
        [[Cell::default(); CELLS_Y as usize]; CELLS_X as usize],
    ));
    let mut cells_next = Some(Box::new(
        [[Cell::default(); CELLS_Y as usize]; CELLS_X as usize],
    ));

    {
        let mut rng = rand::thread_rng();
        let cells = cells_next.as_mut().unwrap();
        for i in 0..(CELLS_X as i64) {
            for j in 0..(CELLS_Y as i64) {
                cells[i as usize][j as usize].init(&mut rng, i, j);
            }
        }
    }

    let t0 = std::time::Instant::now();
    let mut frames: u64 = 0;
    let mut last_fps_update = std::time::Instant::now();
    let mut events = sdl_context.event_pump().unwrap();
    let target_frame_duration = std::time::Duration::from_nanos(1_000_000_000 / 60);
    'mainloop: loop {
        let frame_start = std::time::Instant::now();

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

        // render
        canvas.clear();
        let cells = cells_next.as_ref().unwrap();
        for i in 0..(CELLS_X as usize) {
            for j in 0..(CELLS_Y as usize) {
                let c = cells[i][j].render();
                canvas.set_draw_color(c);
                canvas
                    .fill_rect(Rect::new(i as i32, j as i32, 1, 1))
                    .ok()
                    .unwrap();
            }
        }
        canvas.present();

        // update
        let old_state: Box<_> = cells_next.take().unwrap();
        let mut new_state: Box<_> = cells_prev.take().unwrap();

        new_state.par_iter_mut().enumerate().for_each(|(i, row)| {
            let i = i as i64;
            let mut rng = rand::thread_rng();
            for j in 0..(CELLS_Y as usize) {
                let old_cell = old_state[i as usize][j as usize];
                let new_cell = old_cell.update(
                    i,
                    j as i64,
                    |i, j| {
                        let (i, j) = wrap_idxs(i, j);
                        old_state[i][j]
                    },
                    &mut rng,
                );
                row[j] = new_cell;
            }
        });
        cells_prev = Some(old_state);
        cells_next = Some(new_state);

        frames += 1;

        // Update FPS display once per second
        if frame_start.duration_since(last_fps_update) >= std::time::Duration::from_secs(1) {
            let elapsed_seconds = frame_start.duration_since(t0).as_secs_f32();
            let fps = frames as f32 / elapsed_seconds;
            canvas
                .window_mut()
                .set_title(&format!("Rust CA - {:.1} FPS", fps))
                .unwrap();
            last_fps_update = frame_start;
        }

        // Limit to 60 FPS
        let elapsed = frame_start.elapsed();
        if elapsed < target_frame_duration {
            std::thread::sleep(target_frame_duration - elapsed);
        }
    }

    let duration = std::time::Instant::now() - t0;
    eprintln!("avg. FPS: {}", (frames as f64) / duration.as_secs_f64());
}

/// Headless simulation - just compute, no rendering
fn run_headless<Cell: CACell>(num_frames: u64) -> f64 {
    let mut cells_prev = Box::new([[Cell::default(); CELLS_Y as usize]; CELLS_X as usize]);
    let mut cells_next = Box::new([[Cell::default(); CELLS_Y as usize]; CELLS_X as usize]);

    // Initialize
    {
        let mut rng = rand::thread_rng();
        for i in 0..(CELLS_X as i64) {
            for j in 0..(CELLS_Y as i64) {
                cells_next[i as usize][j as usize].init(&mut rng, i, j);
            }
        }
    }

    let t0 = std::time::Instant::now();
    let mut frames: u64 = 0;

    while frames < num_frames {
        let old_state = cells_next;
        let mut new_state = cells_prev;

        new_state.par_iter_mut().enumerate().for_each(|(i, row)| {
            let i = i as i64;
            let mut rng = rand::thread_rng();
            for j in 0..(CELLS_Y as usize) {
                let old_cell = old_state[i as usize][j as usize];
                let new_cell = old_cell.update(
                    i,
                    j as i64,
                    |i, j| {
                        let (i, j) = wrap_idxs(i, j);
                        old_state[i][j]
                    },
                    &mut rng,
                );
                row[j] = new_cell;
            }
        });

        cells_prev = old_state;
        cells_next = new_state;
        frames += 1;
    }

    let duration = t0.elapsed();
    (frames as f64) / duration.as_secs_f64()
}

fn main() {
    let args = Args::parse();

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()
        .unwrap();

    match args.ca {
        CAType::GameOfLiving => {
            if args.headless {
                let fps = run_headless::<game_of_living::Cell>(args.frames);
                println!("Game of Living - avg. FPS: {:.1}", fps);
            } else {
                run::<game_of_living::Cell>();
            }
        }
        CAType::ForestFire => {
            if args.headless {
                let fps = run_headless::<forestfire::Cell>(args.frames);
                println!("Forest Fire - avg. FPS: {:.1}", fps);
            } else {
                run::<forestfire::Cell>();
            }
        }
        CAType::Colors => {
            if args.headless {
                let fps = run_headless::<colors::Cell>(args.frames);
                println!("Colors - avg. FPS: {:.1}", fps);
            } else {
                run::<colors::Cell>();
            }
        }
        CAType::Continuous => {
            if args.headless {
                let fps = run_headless::<continous::Cell>(args.frames);
                println!("Continuous - avg. FPS: {:.1}", fps);
            } else {
                run::<continous::Cell>();
            }
        }
    }
}
