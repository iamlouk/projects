#![feature(duration_constants)]

use std::fmt::Debug;

// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
use rand::Rng;

pub type VReg = u8;

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum Instr {
    SYS { addr: u16 },
    CLS,
    RET,
    JP { addr: u16 },
    JP_V0 { offset: u16 },
    CALL { addr: u16 },
    MV { dst: VReg, src: VReg },
    LD_IMM { dst: VReg, imm: u8 },
    LD_I { addr: u16 },
    LD_DT { dst: VReg },
    LD_K { dst: VReg },
    LD_SPRITE { digit: VReg },
    LD_BCD { num: VReg },
    LD_REGS_TO_I { upto: VReg },
    LD_I_TO_REGS { upto: VReg },
    SET_DT { x: VReg },
    SET_ST { x: VReg },
    SE_IMM { x: VReg, imm: u8 },
    SNE_IMM { x: VReg, imm: u8 },
    SE_REG { x: VReg, y: VReg },
    SNE_REG { x: VReg, y: VReg },
    OR { x: VReg, y: VReg },
    AND { x: VReg, y: VReg },
    XOR { x: VReg, y: VReg },
    ADD { x: VReg, y: VReg },
    ADD_IMM { dst: VReg, imm: u8 },
    ADD_I { x: VReg },
    SUB { x: VReg, y: VReg },
    SHR { x: VReg, y: VReg },
    SUBN { x: VReg, y: VReg },
    SHL { x: VReg, y: VReg },
    RND { dst: VReg, mask: u8 },
    DRW { x: VReg, y: VReg, n: u8 },
    SKP { x: VReg },
    SKNP { x: VReg },
}

impl Instr {
    #[allow(dead_code)]
    fn decode(addr: usize, mem: &[u8]) -> Result<Instr, &'static str> {
        let b0 = mem[addr];
        let b1 = mem[addr + 1];
        let nnn = (((b0 as u16) & 0xf) << 8) | (b1 as u16);
        let x = (b0 & 0xf) as VReg;
        let y = ((b1 >> 4) & 0xf) as VReg;
        Ok(match b0 >> 4 {
            0x0 => match nnn {
                0x0E0 => Instr::CLS,
                0x0EE => Instr::RET,
                nnn => Instr::SYS { addr: nnn },
            },
            0x1 => Instr::JP { addr: nnn },
            0x2 => Instr::CALL { addr: nnn },
            0x3 => Instr::SE_IMM { x, imm: b1 },
            0x4 => Instr::SNE_IMM { x, imm: b1 },
            0x5 => Instr::SE_REG { x, y },
            0x6 => Instr::LD_IMM { dst: x, imm: b1 },
            0x7 => Instr::ADD_IMM { dst: x, imm: b1 },
            0x8 => match b1 & 0xf {
                0x0 => Instr::MV { dst: x, src: y },
                0x1 => Instr::OR { x, y },
                0x2 => Instr::AND { x, y },
                0x3 => Instr::XOR { x, y },
                0x4 => Instr::ADD { x, y },
                0x5 => Instr::SUB { x, y },
                0x6 => Instr::SHR { x, y },
                0x7 => Instr::SUBN { x, y },
                0xE => Instr::SHL { x, y },
                _ => return Err("invalid instruction with 0x8??? encoding."),
            },
            0x9 => match b1 & 0xf {
                0x0 => Instr::SNE_REG { x, y },
                _ => return Err("invalid instruction with 0x9??? encoding."),
            },
            0xA => Instr::LD_I { addr: nnn },
            0xB => Instr::JP_V0 { offset: nnn },
            0xC => Instr::RND { dst: x, mask: b1 },
            0xD => Instr::DRW { x, y, n: b1 & 0xF },
            0xE => match b1 {
                0x9E => Instr::SKP { x },
                0xA1 => Instr::SKNP { x },
                _ => return Err("invalid instruction with encoding 0xE???"),
            },
            0xF => match b1 {
                0x07 => Instr::LD_DT { dst: x },
                0x0A => Instr::LD_K { dst: x },
                0x15 => Instr::SET_DT { x },
                0x18 => Instr::SET_ST { x },
                0x1E => Instr::ADD_I { x },
                0x29 => Instr::LD_SPRITE { digit: x },
                0x33 => Instr::LD_BCD { num: x },
                0x55 => Instr::LD_REGS_TO_I { upto: x },
                0x65 => Instr::LD_REGS_TO_I { upto: x },
                _ => return Err("invalid instruction with encoding 0xF???"),
            },
            _ => return Err("WTF? Impossible!"),
        })
    }

    #[allow(dead_code)]
    fn fmt(vreg: VReg, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match vreg {
            0x0 => "v0",
            0x1 => "v1",
            0x2 => "v2",
            0x3 => "v3",
            0x4 => "v4",
            0x5 => "v5",
            0x6 => "v6",
            0x7 => "v7",
            0x8 => "v8",
            0x9 => "v9",
            0xA => "vA",
            0xB => "vB",
            0xC => "vC",
            0xD => "vD",
            0xE => "vE",
            0xF => "F",
            _ => "<invalid V reg.>",
        })
    }
}

#[allow(dead_code)]
pub trait UI {
    fn is_key_pressed(&mut self, key: u8) -> bool;
    fn clear_screen(&mut self);
    fn draw_pixel(&mut self, x: usize, y: usize, val: bool);
    fn update(&mut self, cycle: u64, dt: std::time::Duration) -> Result<bool, &'static str>;
    fn rnd(&mut self) -> u8 {
        rand::thread_rng().gen()
    }
}

#[allow(dead_code)]
pub struct Chip8State {
    v_regs: [u8; 16],
    memory: [u8; 0x1000],
    display: [[bool; 64]; 32],
    pub ui: Box<dyn UI>,
    pc: u16,
    idx_reg: u16,
    sound_timer: u8,
    delay_timer: u8,
    stack: Vec<u16>,
    digit_sprites: [u16; 16],
    last_frame: std::time::Duration,
    cycles: u64,
}

#[allow(dead_code)]
impl Chip8State {
    pub fn new(ui: Box<dyn UI>, rom: &[u8]) -> Box<Chip8State> {
        let mut ch8 = Box::new(Chip8State {
            v_regs: [0; 16],
            memory: [0; 0x1000],
            display: [[false; 64]; 32],
            ui,
            idx_reg: 0,
            sound_timer: 0,
            delay_timer: 0,
            pc: 0x200,
            stack: Vec::with_capacity(16),
            digit_sprites: [0; 16],
            last_frame: if cfg!(feature = "time") {
                std::time::SystemTime::UNIX_EPOCH.elapsed().unwrap()
            } else {
                std::time::Duration::from_secs(0)
            },
            cycles: 0,
        });
        fn store_digit(pos: usize, sprite: &[u8], memory: &mut [u8]) -> usize {
            memory[pos..(sprite.len() + pos)].copy_from_slice(sprite);
            pos + sprite.len()
        }
        let mut pos: usize = 0x050;
        ch8.digit_sprites[0] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x90, 0x90, 0x90, 0xF0], &mut ch8.memory);
        ch8.digit_sprites[1] = pos as u16;
        pos = store_digit(pos, &[0x20, 0x60, 0x20, 0x20, 0x70], &mut ch8.memory);
        ch8.digit_sprites[2] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x10, 0xF0, 0x80, 0xF0], &mut ch8.memory);
        ch8.digit_sprites[3] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x10, 0xF0, 0x10, 0xF0], &mut ch8.memory);
        ch8.digit_sprites[4] = pos as u16;
        pos = store_digit(pos, &[0x90, 0x90, 0xF0, 0x10, 0x10], &mut ch8.memory);
        ch8.digit_sprites[5] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0xF0, 0x10, 0xF0], &mut ch8.memory);
        ch8.digit_sprites[6] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0xF0, 0x90, 0xF0], &mut ch8.memory);
        ch8.digit_sprites[7] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x10, 0x20, 0x40, 0x40], &mut ch8.memory);
        ch8.digit_sprites[8] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x90, 0xF0, 0x90, 0xF0], &mut ch8.memory);
        ch8.digit_sprites[9] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x90, 0xF0, 0x10, 0xF0], &mut ch8.memory);
        ch8.digit_sprites[10] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x90, 0xF0, 0x90, 0x90], &mut ch8.memory);
        ch8.digit_sprites[11] = pos as u16;
        pos = store_digit(pos, &[0xE0, 0x90, 0xE0, 0x90, 0xE0], &mut ch8.memory);
        ch8.digit_sprites[12] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0x80, 0x80, 0xF0], &mut ch8.memory);
        ch8.digit_sprites[13] = pos as u16;
        pos = store_digit(pos, &[0xE0, 0x90, 0x90, 0x90, 0xE0], &mut ch8.memory);
        ch8.digit_sprites[14] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0xF0, 0x80, 0xF0], &mut ch8.memory);
        ch8.digit_sprites[15] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0xF0, 0x80, 0x80], &mut ch8.memory);
        _ = pos;
        for (i, b) in rom.iter().enumerate() {
            ch8.memory[0x200 + i] = *b;
        }
        ch8
    }

    pub fn cycle(&mut self) -> Result<std::time::Duration, &'static str> {
        self.exec()?;
        self.cycles = self.cycles.wrapping_add(1);
        let dt = if cfg!(feature = "time") {
            let now = std::time::SystemTime::UNIX_EPOCH.elapsed().unwrap();
            let dt = now - self.last_frame;
            self.last_frame = now;
            dt
        } else {
            std::time::Duration::from_secs(0)
        };
        let timer_tick = self.ui.update(self.cycles, dt)?;
        if timer_tick {
            self.delay_timer = self.delay_timer.saturating_sub(1);
            self.sound_timer = self.sound_timer.saturating_sub(1);
        }
        Ok(dt)
    }

    fn exec(&mut self) -> Result<u16, &'static str> {
        let instr = Instr::decode(self.pc as usize, &self.memory)?;
        // eprintln!("PC={:#08x}: INST={:?}", self.pc, &instr);
        self.pc += 2;
        match instr {
            Instr::SYS { addr: _ } => return Err("unimplemented SYS instr."),
            Instr::CLS => {
                for i in 0..self.display.len() {
                    let row = &mut self.display[i];
                    for j in 0..row.len() {
                        self.display[i][j] = false;
                    }
                }
                self.ui.clear_screen()
            }
            Instr::RET => match self.stack.pop() {
                Some(addr) => self.pc = addr,
                None => return Err("ret from empty stack"),
            },
            Instr::JP { addr } => {
                if self.pc - 2 == addr {
                    return Err("busy wait");
                } else {
                    self.pc = addr
                }
            }
            Instr::JP_V0 { offset } => self.pc = self.v_regs[0] as u16 + offset,
            Instr::CALL { addr } => {
                self.stack.push(self.pc);
                self.pc = addr;
            }
            Instr::MV { dst, src } => self.v_regs[dst as usize] = self.v_regs[src as usize],
            Instr::LD_IMM { dst, imm } => self.v_regs[dst as usize] = imm,
            Instr::LD_I { addr } => self.idx_reg = addr,
            Instr::LD_DT { dst } => self.v_regs[dst as usize] = self.delay_timer,
            Instr::LD_K { dst } => {
                for key in 0..0xF {
                    if self.ui.is_key_pressed(key) {
                        self.v_regs[dst as usize] = key;
                    }
                }
                // Retry later...
                self.pc -= 2;
            }
            Instr::LD_SPRITE { digit } => {
                if digit < 16 {
                    self.idx_reg = self.digit_sprites[self.v_regs[digit as usize] as usize];
                } else {
                    return Err("'LD F, Vx' with a Vx out of bounds");
                }
            }
            Instr::LD_BCD { num } => {
                let num = self.v_regs[num as usize];
                self.memory[self.idx_reg as usize] = num / 100;
                self.memory[self.idx_reg as usize + 1] = num / 10;
                self.memory[self.idx_reg as usize + 2] = num % 10;
            }
            Instr::LD_REGS_TO_I { upto } => {
                let pos = self.idx_reg as usize;
                for i in 0..=(upto as usize) {
                    self.memory[pos + i] = self.v_regs[i];
                }
            }
            Instr::LD_I_TO_REGS { upto } => {
                let pos = self.idx_reg as usize;
                for i in 0..=(upto as usize) {
                    self.v_regs[i] = self.memory[pos + i];
                }
            }
            Instr::SET_DT { x } => self.delay_timer = self.v_regs[x as usize],
            Instr::SET_ST { x } => self.sound_timer = self.v_regs[x as usize],
            Instr::SE_IMM { x, imm } => {
                if self.v_regs[x as usize] == imm {
                    self.pc += 2
                }
            }
            Instr::SNE_IMM { x, imm } => {
                if self.v_regs[x as usize] != imm {
                    self.pc += 2
                }
            }
            Instr::SE_REG { x, y } => {
                if self.v_regs[x as usize] == self.v_regs[y as usize] {
                    self.pc += 2
                }
            }
            Instr::SNE_REG { x, y } => {
                if self.v_regs[x as usize] != self.v_regs[y as usize] {
                    self.pc += 2
                }
            }
            Instr::OR { x, y } => self.v_regs[x as usize] |= self.v_regs[y as usize],
            Instr::AND { x, y } => self.v_regs[x as usize] &= self.v_regs[y as usize],
            Instr::XOR { x, y } => self.v_regs[x as usize] ^= self.v_regs[y as usize],
            Instr::ADD { x, y } => {
                let v1 = self.v_regs[x as usize] as u16;
                let v2 = self.v_regs[y as usize] as u16;
                let res = v1 + v2;
                self.v_regs[x as usize] = (res & 0xFF) as u8;
                self.v_regs[0xF] = (res > 0xFF) as u8;
            }
            Instr::ADD_IMM { dst, imm } => self.v_regs[dst as usize] += imm,
            Instr::ADD_I { x } => self.idx_reg += self.v_regs[x as usize] as u16,
            Instr::SUB { x, y } => {
                let v1 = self.v_regs[x as usize] as u16;
                let v2 = self.v_regs[y as usize] as u16;
                self.v_regs[x as usize] = ((v1 - v2) & 0xFF) as u8;
                self.v_regs[0xF] = (v1 > v2) as u8;
            }
            Instr::SHR { x, y: _ } => {
                let v1 = self.v_regs[x as usize];
                self.v_regs[0xF] = v1 & 0x1;
                self.v_regs[x as usize] = v1 >> 1;
            }
            Instr::SUBN { x, y } => {
                let v1 = self.v_regs[x as usize] as u16;
                let v2 = self.v_regs[y as usize] as u16;
                self.v_regs[x as usize] = ((v2 - v1) & 0xFF) as u8;
                self.v_regs[0xF] = (v2 > v1) as u8;
            }
            Instr::SHL { x, y: _ } => {
                let v1 = self.v_regs[x as usize];
                self.v_regs[0xF] = ((v1 & 0x80) != 0) as u8;
                self.v_regs[x as usize] = v1 << 1;
            }
            Instr::RND { dst, mask } => {
                self.v_regs[dst as usize] = self.ui.rnd() & mask;
            }
            Instr::DRW { x, y, n } => {
                let x = self.v_regs[x as usize] as usize;
                let y = self.v_regs[y as usize] as usize;

                let mut pixel_erased = false;
                for row in 0..(n as usize) {
                    let b = self.memory[self.idx_reg as usize + row];
                    for col in 0..8 {
                        let x = (x + col) % 64;
                        let y = (y + row) % 32;
                        if (b & (0x80u8 >> col)) == 0 {
                            continue;
                        }

                        let prev_pixel = self.display[y][x];
                        let new_pixel = !prev_pixel;
                        pixel_erased |= !new_pixel;
                        self.display[y][x] = new_pixel;
                        self.ui.draw_pixel(x, y, new_pixel);
                    }
                }
                self.v_regs[0xF] = pixel_erased as u8;
            }
            Instr::SKP { x } => {
                if self.ui.is_key_pressed(self.v_regs[x as usize]) {
                    self.pc += 2
                }
            }
            Instr::SKNP { x } => {
                if !self.ui.is_key_pressed(self.v_regs[x as usize]) {
                    self.pc += 2
                }
            }
        }
        Ok(self.pc)
    }
}
