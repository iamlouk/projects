extern crate crossterm;
extern crate rand;

use std::{io::{stdout, Write}, time::Duration};
use crossterm::{style::Color, event::KeyEvent, QueueableCommand};
use crate::rand::distributions::Distribution;

const ROWS: isize = 25;
const COLS: isize = 11;

fn draw_border(w: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
    w.queue(crossterm::style::SetForegroundColor(Color::White))?;
    w.queue(crossterm::style::SetBackgroundColor(Color::Reset))?;
    w.queue(crossterm::cursor::MoveTo(0, 1))?;
    let mut line = String::with_capacity(COLS as usize * 2 + 2);
    line.push('╔');
    for _ in 0..(COLS * 2) { line.push('═'); }
    line.push('╗');
    w.write(line.as_bytes())?;
    line.clear();
    line.push('║');
    for _ in 0..(COLS * 2) { line.push(' '); }
    line.push('║');
    for row in 0..ROWS {
        w.queue(crossterm::cursor::MoveTo(0, (row + 2) as u16))?;
        w.write(line.as_bytes())?;
    }
    line.clear();
    line.push('╚');
    for _ in 0..(COLS * 2) { line.push('═'); }
    line.push('╝');
    w.queue(crossterm::cursor::MoveTo(0, (ROWS + 2) as u16))?;
    w.write(line.as_bytes())?;
    Ok(())
}

#[derive(Clone, Debug)]
struct Shape([[Color; 3]; 3]);

impl Shape {
    fn iter(&self) -> impl std::iter::Iterator<Item = (isize, isize, Color)> + '_ {
        self.0.iter()
            .enumerate()
            .flat_map(|(y, row)|
                row.iter().enumerate().map(move |(x, color)| (x as isize, y as isize, *color)))
            .filter(|(_, _, color)| *color != Color::Reset)
    }
}

struct Game {
    rng: rand::rngs::ThreadRng,
    color: Color,
    old_grid: Box<[[Color; COLS as usize]; ROWS as usize]>,
    new_grid: Box<[[Color; COLS as usize]; ROWS as usize]>,

    cursor_y: isize,
    cursor_x: isize,
}

impl Game {
    fn random_color(&mut self) -> Color {
        let colors = [
            Color::Red,     Color::DarkRed,
            Color::Green,   Color::DarkGreen,
            Color::Yellow,  Color::DarkYellow,
            Color::Blue,    Color::DarkBlue,
            Color::Magenta, Color::DarkMagenta,
            Color::Cyan,    Color::DarkCyan,
        ];
        colors[rand::distributions::Uniform::from(0..colors.len()).sample(&mut self.rng)]
    }

    fn random_shape(&mut self) -> Shape {
        let c = Color::Reset;
        let x = self.random_color();
        match rand::distributions::Uniform::from(0..7).sample(&mut self.rng) {
            0 => Shape([[c, c, c],
                        [c, x, x],
                        [c, x, x]]),
            1 => Shape([[c, c, c],
                        [x, x, x],
                        [c, c, c]]),
            2 => Shape([[c, x, c],
                        [c, x, c],
                        [c, x, x]]),
            3 => Shape([[c, x, c],
                        [c, x, x],
                        [c, c, x]]),
            4 => Shape([[c, c, c],
                        [c, x, x],
                        [c, x, x]]),
            5 => Shape([[c, x, c],
                        [c, x, c],
                        [x, x, x]]),
            6 => Shape([[c, x, c],
                        [c, x, c],
                        [x, x, c]]),
            _ => panic!()
        }
    }

    fn change_color(&mut self, w: &mut dyn std::io::Write, color: Color)
            -> Result<(), std::io::Error> {
        if self.color != color {
            w.queue(crossterm::style::SetForegroundColor(color))?;
            self.color = color;
        }
        Ok(())
    }

    #[allow(unused)]
    fn log(&mut self, w: &mut dyn std::io::Write, msg: &str) {
        w.queue(crossterm::cursor::MoveTo(0, (ROWS + 3) as u16)).unwrap();
        self.change_color(w, Color::Reset).unwrap();
        w.write(msg.as_bytes()).unwrap();
        w.write("            ".as_bytes()).unwrap();
        w.flush();
    }

    fn update(&mut self, w: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        for row in 0..ROWS {
            for col in 0..COLS {
                let old = self.old_grid[row as usize][col as usize];
                let new = self.new_grid[row as usize][col as usize];
                if old == new {
                    continue;
                }

                let x = (1 + (col * 2)) as u16;
                let y = (row + 2) as u16;
                w.queue(crossterm::cursor::MoveTo(x, y))?;
                match new {
                    Color::Reset => {
                        w.write("  ".as_bytes())?;
                    },
                    color => {
                        self.change_color(w, color)?;
                        w.write("██".as_bytes())?;
                    }
                }

                self.old_grid[row as usize][col as usize] = new;
            }
        }

        assert_eq!(self.old_grid, self.new_grid);
        Ok(())
    }

    fn move_rows_down(&mut self, above: usize) {
        for row in (0..(above+1)).rev() {
            for col in 0..COLS {
                let cell = if row == 0 {
                    Color::Reset
                } else {
                    self.new_grid[row - 1][col as usize]
                };
                self.new_grid[row][col as usize] = cell;
            }
        }
    }

    fn remove_rows(&mut self) -> bool {
        let mut changed = false;
        for row in (0..ROWS).rev() {
            let full = self.new_grid[row as usize].iter().all(|cell| *cell != Color::Reset);
            if full {
                self.move_rows_down(row as usize);
                changed = true;
            }
        }
        changed
    }

    fn check_shape_touchdown(&mut self, shape: &mut Shape) -> bool {
        let mut touchdown = false;
        for (c, r, _) in shape.iter() {
            let col = self.cursor_x + c;
            let row = self.cursor_y + r;
            touchdown = row + 1 >= ROWS ||
                self.new_grid[(row + 1) as usize][col as usize] != Color::Reset;
            if touchdown {
                break;
            }
        }

        if touchdown {
            for (c, r, color) in shape.iter() {
                let col = self.cursor_x + c;
                let row = self.cursor_y + r;
                self.new_grid[row as usize][col as usize] = color;
            }

            *shape = self.random_shape();
            self.cursor_x = COLS / 2 - 1;
            self.cursor_y = 0;
        }
        touchdown
    }

    fn valid(&self, shape: &Shape, dx: isize, dy: isize) -> bool {
        for (c, r, _) in shape.iter() {
            if self.cursor_x + c + dx < 0 || self.cursor_x + c + dx >= COLS ||
               self.cursor_y + r + dy < 0 || self.cursor_y + r + dy >= ROWS {
                return false
            }

            let col = (self.cursor_x + c + dx) as usize;
            let row = (self.cursor_y + r + dy) as usize;
            if self.new_grid[row][col] != Color::Reset {
                return false
            }
        }
        true
    }

    fn draw_shape(&mut self, shape: &Shape,
                  clear: bool, w: &mut dyn std::io::Write) -> Result<(), std::io::Error> {
        for (c, r, color) in shape.iter() {
            let col = self.cursor_x + c;
            let row = self.cursor_y + r;
            let x = (1 + (col * 2)) as u16;
            let y = (row + 2) as u16;
            w.queue(crossterm::cursor::MoveTo(x, y))?;
            if clear {
                w.write("  ".as_bytes())?;
            } else {
                self.change_color(w, color)?;
                w.write("██".as_bytes())?;
            }
        }
        Ok(())
    }

    fn rotate(&mut self, shape: &mut Shape) {
        let mut rotated = Shape([[Color::Reset; 3]; 3]);
        for x in 0..3 {
            for y in 0..3 {
                let (nx, ny) = match (x, y) {
                    (0, 0) => (0, 2),
                    (0, 1) => (1, 2),
                    (0, 2) => (2, 2),
                    (1, 0) => (0, 1),
                    (1, 1) => (1, 1),
                    (1, 2) => (2, 1),
                    (2, 0) => (0, 0),
                    (2, 1) => (1, 0),
                    (2, 2) => (2, 0),
                    (_, _) => panic!("wtf?")
                };
                rotated.0[nx][ny] = shape.0[x][y];
            }
        }

        if self.valid(&rotated, 0, 0) {
            *shape = rotated;
        }
    }

    fn update_shape(&mut self, shape: &mut Shape, dx: isize, dy: isize) {
        if !self.valid(shape, dx, dy) {
            return
        }

        self.cursor_x += dx;
        self.cursor_y += dy;
    }
}

fn check_terminal_size() -> bool {
    let (cols, rows) = match crossterm::terminal::size() {
        Ok((cols, rows)) => (cols as isize, rows as isize),
        Err(_) => return false
    };

    cols >= 2 + 2 * COLS && rows >= 5 + ROWS
}

fn main() -> Result<(), std::io::Error> {
    if !check_terminal_size() {
        eprintln!("terminal too small (or TTY is not a terminal)");
        return Ok(());
    }

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.queue(crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
    stdout.queue(crossterm::cursor::MoveTo(0, 0))?;
    stdout.write("Tetris!".as_bytes())?;
    draw_border(&mut stdout)?;
    stdout.flush()?;

    let mut game = Game {
        rng: rand::thread_rng(),
        color: Color::Reset,
        old_grid: Box::new([[Color::Reset; COLS as usize]; ROWS as usize]),
        new_grid: Box::new([[Color::Reset; COLS as usize]; ROWS as usize]),

        cursor_x: COLS / 2,
        cursor_y: 0,
    };

    let mut score = 0;
    let mut shape = game.random_shape();
    let mut sleep_time = Duration::from_millis(500);
    let mut goodbyemsg = String::new();

    'mainloop: for iter in 0.. {
        let mut update = true;
        game.draw_shape(&shape, true, &mut stdout)?;
        if crossterm::event::poll(sleep_time)? {
            use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
            match crossterm::event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('Q') | KeyCode::Char('q'),
                    kind: KeyEventKind::Press, .. }) |
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL, .. }) => {
                    goodbyemsg = format!("You wanted to quit...! Score: {}.", score);
                    break 'mainloop
                },

                Event::Key(KeyEvent {
                    code: KeyCode::Up, kind: KeyEventKind::Press, .. }) =>
                    { game.rotate(&mut shape); update = false; score += 1; },
                Event::Key(KeyEvent {
                    code: KeyCode::Down, kind: KeyEventKind::Press, .. }) =>
                    { game.update_shape(&mut shape,  0, 1); update = false },
                Event::Key(KeyEvent {
                    code: KeyCode::Left, kind: KeyEventKind::Press, .. }) =>
                    { game.update_shape(&mut shape, -1, 0); update = false },
                Event::Key(KeyEvent {
                    code: KeyCode::Right, kind: KeyEventKind::Press, .. }) =>
                    { game.update_shape(&mut shape,  1, 0); update = false },

                Event::Resize(_, _) => {
                    if !check_terminal_size() {
                        goodbyemsg = format!("Terminal too small!");
                        break 'mainloop;
                    }
                },

                _ => { continue; }
            }
        }

        if game.check_shape_touchdown(&mut shape) {
            if !game.valid(&shape, 0, 0) {
                goodbyemsg = format!("Game Over! Score: {}", score);
                break
            }
            update = false;
        }

        if update {
            game.update_shape(&mut shape, 0, 1);
            score += 1;
        }

        game.remove_rows();
        game.update(&mut stdout)?;
        game.draw_shape(&shape, false, &mut stdout)?;
        stdout.queue(crossterm::cursor::MoveTo(0, 0))?;
        stdout.flush()?;

        if iter % 25 == 0 {
            sleep_time = Duration::from_secs_f32(sleep_time.as_secs_f32() * 0.99f32);
            game.change_color(&mut stdout, Color::Reset)?;
            stdout.queue(crossterm::cursor::MoveTo(0, 0))?;
            write!(&mut stdout, "Tetris! {}, fps={:.1}", score,
                1. / sleep_time.as_secs_f32())?;
        }
    }

    stdout.queue(crossterm::style::SetForegroundColor(Color::Reset))?;
    stdout.queue(crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;
    stdout.queue(crossterm::cursor::MoveTo(0, 0))?;
    stdout.flush()?;
    crossterm::terminal::disable_raw_mode()?;
    if goodbyemsg.len() > 0 {
        println!("{}", goodbyemsg);
    }
    Ok(())
}

