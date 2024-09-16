#![feature(duration_constants)]

use crossterm::{execute, queue, QueueableCommand};
use std::io::Write;

struct TerminalUI {
    stdout: std::io::Stdout,
    keys: [u32; 16],
}

impl TerminalUI {
    fn draw_border(&mut self) -> Result<(), std::io::Error> {
        use crossterm::cursor::*;
        use crossterm::style::*;
        let (cols, rows) = (64u16, 32u16);
        queue!(
            self.stdout,
            SetForegroundColor(Color::Cyan),
            MoveTo(0, 0),
            Print('╔'),
        )?;
        for _ in 0..(cols * 2) {
            self.stdout.queue(Print('═'))?;
        }
        self.stdout.queue(Print('╗'))?;
        for row in 0..rows {
            queue!(
                self.stdout,
                MoveTo(0, row + 1),
                Print('║'),
                MoveTo(cols * 2 + 1, row + 1),
                Print('║')
            )?;
        }
        self.stdout.queue(MoveTo(0, 1 + rows))?;
        self.stdout.queue(Print('╚'))?;
        for _ in 0..(cols * 2) {
            self.stdout.queue(Print('═'))?;
        }
        self.stdout.queue(Print('╝'))?;
        self.stdout.queue(SetForegroundColor(Color::Reset))?;
        self.stdout.flush()?;
        Ok(())
    }
}

impl chip8::UI for TerminalUI {
    fn clear_screen(&mut self) {
        queue!(
            self.stdout,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        )
        .unwrap();
        self.draw_border().unwrap();
    }

    fn draw_pixel(&mut self, x: usize, y: usize, val: bool) {
        queue!(
            self.stdout,
            crossterm::cursor::MoveTo((x * 2) as u16 + 1, y as u16 + 1)
        )
        .unwrap();
        queue!(
            self.stdout,
            crossterm::style::Print(if val { "██" } else { "  " })
        )
        .unwrap();
    }

    fn is_key_pressed(&mut self, key: u8) -> bool {
        self.keys[(key & 0xF) as usize] > 0
    }

    fn update(&mut self, cycle: u64, dt: std::time::Duration) -> Result<bool, &'static str> {
        const F: u32 = 4;
        const KEY_PRESSED_FOR: u32 = 10;
        self.stdout.flush().unwrap();
        for i in 0..self.keys.len() {
            self.keys[i] = self.keys[i].saturating_sub(1)
        }

        let stime = std::time::Duration::SECOND / (60 * F) - dt;
        if crossterm::event::poll(stime).unwrap() {
            use crossterm::event::{Event, KeyCode, ModifierKeyCode};
            match crossterm::event::read().unwrap() {
                Event::Key(e) => match e.code {
                    KeyCode::Char('1') => self.keys[0x1] = KEY_PRESSED_FOR,
                    KeyCode::Char('2') => self.keys[0x2] = KEY_PRESSED_FOR,
                    KeyCode::Char('3') => self.keys[0x3] = KEY_PRESSED_FOR,
                    KeyCode::Char('4') => self.keys[0xC] = KEY_PRESSED_FOR,
                    KeyCode::Char('q') => self.keys[0x4] = KEY_PRESSED_FOR,
                    KeyCode::Char('w') => self.keys[0x5] = KEY_PRESSED_FOR,
                    KeyCode::Char('e') => self.keys[0x6] = KEY_PRESSED_FOR,
                    KeyCode::Char('r') => self.keys[0xD] = KEY_PRESSED_FOR,
                    KeyCode::Char('a') => self.keys[0x7] = KEY_PRESSED_FOR,
                    KeyCode::Char('s') => self.keys[0x8] = KEY_PRESSED_FOR,
                    KeyCode::Char('d') => self.keys[0x9] = KEY_PRESSED_FOR,
                    KeyCode::Char('f') => self.keys[0xE] = KEY_PRESSED_FOR,

                    // Here the german and US keyboard differ...
                    KeyCode::Char('y') => self.keys[0xA] = KEY_PRESSED_FOR,
                    KeyCode::Char('x') => self.keys[0x0] = KEY_PRESSED_FOR,
                    KeyCode::Char('c') => self.keys[0xB] = KEY_PRESSED_FOR,
                    KeyCode::Char('v') => self.keys[0xF] = KEY_PRESSED_FOR,

                    KeyCode::Esc
                    | KeyCode::Backspace
                    | KeyCode::Delete
                    | KeyCode::Modifier(ModifierKeyCode::LeftControl)
                    | KeyCode::Modifier(ModifierKeyCode::RightControl)
                    | KeyCode::Enter => return Err("Bye!"),
                    _ => {}
                },
                Event::Resize(_, _) => return Err("Terminal resized!"),
                _ => {}
            }
        }

        Ok(cycle % (F as u64) == 0)
    }
}

fn main() {
    let ws = crossterm::terminal::window_size().unwrap();
    if ws.rows <= 32 || ws.columns <= 64 * 2 {
        eprintln!("terminal too small! rows={}, cols={}", ws.rows, ws.columns);
    }

    let rom_file = std::env::args().nth(1).unwrap_or("rom.ch8".to_string());
    println!(
        "hi! (terminal-size={}x{}, rom={:?})",
        ws.columns, ws.rows, rom_file
    );
    let rom: Vec<u8> = match std::fs::read(&rom_file) {
        Ok(buf) => buf,
        Err(e) => {
            eprintln!("{}: {}", &rom_file, e);
            std::process::exit(1);
        }
    };

    let err = {
        let ui = Box::new(TerminalUI {
            stdout: std::io::stdout(),
            keys: [0; 16],
        });
        let mut c8 = chip8::Chip8State::new(ui, &rom);
        crossterm::terminal::enable_raw_mode().unwrap();
        c8.ui.clear_screen();
        loop {
            match c8.cycle() {
                Ok(_) => continue,
                Err(e) => break e,
            }
        }
    };

    execute!(std::io::stdout(), crossterm::cursor::MoveTo(0, 34)).unwrap();
    crossterm::terminal::disable_raw_mode().unwrap();
    println!("{} (terminal-size={}x{})", err, ws.columns, ws.rows);
}
