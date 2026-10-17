#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_range_loop)]
#![feature(portable_simd)]

use clap::Parser;
use rayon::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

mod colors;
mod continous;
mod continuous_vectorized;
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

    fn update_frame(
        old_state: &[[Self; CELLS_Y as usize]; CELLS_X as usize],
        new_state: &mut [[Self; CELLS_Y as usize]; CELLS_X as usize],
    ) {
        new_state
            .par_iter_mut()
            .enumerate()
            .for_each(|(x, new_col)| {
                Self::update_row(&old_state[x], new_col, old_state, x);
            });
    }

    fn update_row(
        old_row: &[Self; CELLS_Y as usize],
        new_row: &mut [Self; CELLS_Y as usize],
        old_grid: &[[Self; CELLS_Y as usize]; CELLS_X as usize],
        y: usize,
    ) {
        for (x, cell) in old_row.iter().enumerate() {
            let mut rng = rand::thread_rng();
            new_row[x] = cell.update(
                x as i64,
                y as i64,
                |i, j| {
                    let (i, j) = wrap_idxs(i, j);
                    old_grid[j][i]
                },
                &mut rng,
            );
        }
    }
}

fn create_grid<Cell: CACell + Default>() -> Box<[[Cell; CELLS_Y as usize]; CELLS_X as usize]> {
    unsafe {
        let mut boxed = Box::<[[std::mem::MaybeUninit<Cell>; CELLS_Y as usize]; CELLS_X as usize]>::new_uninit();
        let ptr = boxed.as_mut_ptr();
        for i in 0..(CELLS_X as usize) {
            for j in 0..(CELLS_Y as usize) {
                (*ptr)[i][j] = std::mem::MaybeUninit::new(Cell::default());
            }
        }
        std::mem::transmute(boxed)
    }
}

const CELLS_X: u32 = 750;
const CELLS_Y: u32 = 750;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CAType {
    GameOfLiving,
    ForestFire,
    Colors,
    Continuous,
    ContinuousVectorized,
}

impl Default for CAType {
    fn default() -> Self {
        CAType::ContinuousVectorized
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
    #[arg(long, value_enum, default_value_t = CAType::ContinuousVectorized)]
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

pub fn run<Cell: CACell>(ca_name: &str) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window(&format!("Rust CA ({})", ca_name), CELLS_X * 1, CELLS_Y * 1)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::ARGB8888, CELLS_X, CELLS_Y)
        .unwrap();

    let mut cells_prev = Some(create_grid::<Cell>());
    let mut cells_next = Some(create_grid::<Cell>());

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

        texture
            .with_lock(None, |buffer, pitch| {
                let cells = cells_next.as_ref().unwrap();
                for y in 0..(CELLS_Y as usize) {
                    let row = y * pitch;
                    for x in 0..(CELLS_X as usize) {
                        let c = cells[x][y].render();
                        let i = row + x * 4;
                        // ARGB8888 little-endian byte order: B, G, R, A
                        buffer[i] = c.b;
                        buffer[i + 1] = c.g;
                        buffer[i + 2] = c.r;
                        buffer[i + 3] = 0xFF;
                    }
                }
            })
            .unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        // update (whole-frame hook, uses the optimized path where available)
        let old_state: Box<_> = cells_next.take().unwrap();
        let mut new_state: Box<_> = cells_prev.take().unwrap();

        Cell::update_frame(&old_state, &mut new_state);
        cells_prev = Some(old_state);
        cells_next = Some(new_state);

        frames += 1;

        // Update FPS display once per second
        if frame_start.duration_since(last_fps_update) >= std::time::Duration::from_secs(1) {
            let elapsed_seconds = frame_start.duration_since(t0).as_secs_f32();
            let fps = frames as f32 / elapsed_seconds;
            canvas
                .window_mut()
                .set_title(&format!("Rust CA ({}) - {:.1} FPS", ca_name, fps))
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
    let mut cells_prev: Box<[[Cell; CELLS_Y as usize]; CELLS_X as usize]> = create_grid();
    let mut cells_next: Box<[[Cell; CELLS_Y as usize]; CELLS_X as usize]> = create_grid();

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

        Cell::update_frame(&old_state, &mut new_state);

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
                run::<game_of_living::Cell>("game-of-living");
            }
        }
        CAType::ForestFire => {
            if args.headless {
                let fps = run_headless::<forestfire::Cell>(args.frames);
                println!("Forest Fire - avg. FPS: {:.1}", fps);
            } else {
                run::<forestfire::Cell>("forest-fire");
            }
        }
        CAType::Colors => {
            if args.headless {
                let fps = run_headless::<colors::Cell>(args.frames);
                println!("Colors - avg. FPS: {:.1}", fps);
            } else {
                run::<colors::Cell>("colors");
            }
        }
        CAType::Continuous => {
            if args.headless {
                let fps = run_headless::<continous::Cell>(args.frames);
                println!("Continuous - avg. FPS: {:.1}", fps);
            } else {
                run::<continous::Cell>("continuous");
            }
        }
        CAType::ContinuousVectorized => {
            if args.headless {
                let fps = run_headless::<continuous_vectorized::Cell>(args.frames);
                println!("Continuous (SIMD) - avg. FPS: {:.1}", fps);
            } else {
                run::<continuous_vectorized::Cell>("continuous-vectorized");
            }
        }
    }
}
