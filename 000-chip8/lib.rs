// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
use rand::Rng;

pub type VReg = u8;

#[allow(non_camel_case_types)]
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
    ADD_IMM { dst: VReg, imm: u16 },
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
        let b0 = mem[addr+0];
        let b1 = mem[addr+1];
        let nnn = (b0 as u16) & 0xf | (b1 as u16);
        let x = (b0 & 0xf) as VReg;
        let y = ((b1 >> 4) & 0xf) as VReg;
        Ok(match b0 >> 4 {
            0x0 => match nnn {
                0x0E0 => Instr::CLS,
                0x0EE => Instr::RET,
                nnn => Instr::SYS { addr: nnn }
            },
            0x1 => Instr::JP { addr: nnn },
            0x2 => Instr::CALL { addr: nnn },
            0x3 => Instr::SE_IMM { x, imm: b1 },
            0x4 => Instr::SNE_IMM { x, imm: b1 },
            0x5 => Instr::SE_REG { x, y },
            0x6 => Instr::LD_IMM { dst: x, imm: b1 },
            0x7 => Instr::ADD_IMM { dst: x, imm: b1 as u16 },
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
                _ => return Err("invalid instruction with 0x8??? encoding.")
            },
            0x9 => match b1 & 0xf {
                0x0 => Instr::SNE_REG { x, y },
                _ => return Err("invalid instruction with 0x9??? encoding.")
            },
            0xA => Instr::LD_I { addr: nnn },
            0xB => Instr::JP_V0 { offset: nnn },
            0xC => Instr::RND { dst: x, mask: b1 },
            0xD => Instr::DRW { x, y, n: b1 },
            0xE => match b1 {
                0x9E => Instr::SKP { x },
                0xA1 => Instr::SKNP { x },
                _ => return Err("invalid instruction with encoding 0xE???")
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
                _ => return Err("invalid instruction with encoding 0xF???")
            },
            _ => unreachable!()
        })
    }
}

#[allow(dead_code)]
trait UI {
    fn wait_for_key_press(&mut self) -> u16;
    fn is_key_pressed(&mut self, key: u16) -> bool;
    fn clear_screen(&mut self);
    fn draw_pixel(&mut self, x: usize, y: usize, val: bool);
    fn rnd(&mut self) -> u8 { rand::thread_rng().gen() }
}

#[allow(dead_code)]
struct Chip8State {
    v_regs: [u16; 16],
    memory: [u8; 0x1000],
    display: [[bool; 64]; 32],
    ui: Box<dyn UI>,
    i_reg: u16,
    sound_timer: u8,
    delay_timer: u8,
    pc: u16,
    stack: Vec<u16>,
    digit_sprites: [u16; 16],
}

#[allow(dead_code)]
impl Chip8State {
    fn new(ui: Box<dyn UI>) -> Box<Chip8State> {
        let mut b = Box::new(Chip8State {
            v_regs: [0; 16],
            memory: [0; 0x1000],
            display: [[false; 64]; 32],
            ui,
            i_reg: 0,
            sound_timer: 0,
            delay_timer: 0,
            pc: 0x200,
            stack: Vec::with_capacity(16),
            digit_sprites: [0; 16]
        });
        fn store_digit(pos: usize, sprite: &[u8], memory: &mut [u8]) -> usize {
            for i in 0..sprite.len() {
                memory[pos + i] = sprite[i];
            }
            return pos + sprite.len();
        }
        let mut pos: usize = 0;
        b.digit_sprites[0] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x90, 0x90, 0x90, 0xF0], &mut b.memory);
        b.digit_sprites[1] = pos as u16;
        pos = store_digit(pos, &[0x20, 0x60, 0x20, 0x20, 0x70], &mut b.memory);
        b.digit_sprites[2] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x10, 0xF0, 0x80, 0xF0], &mut b.memory);
        b.digit_sprites[3] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x10, 0xF0, 0x10, 0xF0], &mut b.memory);
        b.digit_sprites[4] = pos as u16;
        pos = store_digit(pos, &[0x90, 0x90, 0xF0, 0x10, 0x10], &mut b.memory);
        b.digit_sprites[5] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0xF0, 0x10, 0xF0], &mut b.memory);
        b.digit_sprites[6] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0xF0, 0x90, 0xF0], &mut b.memory);
        b.digit_sprites[7] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x10, 0x20, 0x40, 0x40], &mut b.memory);
        b.digit_sprites[8] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x90, 0xF0, 0x90, 0xF0], &mut b.memory);
        b.digit_sprites[9] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x90, 0xF0, 0x10, 0xF0], &mut b.memory);
        b.digit_sprites[10] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x90, 0xF0, 0x90, 0x90], &mut b.memory);
        b.digit_sprites[11] = pos as u16;
        pos = store_digit(pos, &[0xE0, 0x90, 0xE0, 0x90, 0xE0], &mut b.memory);
        b.digit_sprites[12] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0x80, 0x80, 0xF0], &mut b.memory);
        b.digit_sprites[13] = pos as u16;
        pos = store_digit(pos, &[0xE0, 0x90, 0x90, 0x90, 0xE0], &mut b.memory);
        b.digit_sprites[14] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0xF0, 0x80, 0xF0], &mut b.memory);
        b.digit_sprites[15] = pos as u16;
        pos = store_digit(pos, &[0xF0, 0x80, 0xF0, 0x80, 0x80], &mut b.memory);
        _ = pos;
        b
    }

    fn exec(&mut self) -> Result<u16, &'static str> {
        let instr = Instr::decode(self.pc as usize, &self.memory)?;
        self.pc += 2;
        match instr {
            Instr::SYS { addr: _ } => unimplemented!(),
            Instr::CLS => {
                for i in 0..self.display.len() {
                    let row = &mut self.display[i];
                    for j in 0..row.len() {
                        self.display[i][j] = false;
                    }
                }
                self.ui.clear_screen()
            },
            Instr::RET => match self.stack.pop() {
                Some(addr) => {
                    self.pc = addr;
                    return Ok(self.pc)
                }
                None => return Err("ret from empty stack")
            },
            Instr::JP { addr } => {
                self.pc = addr;
                return Ok(self.pc)
            },
            Instr::JP_V0 { offset } => {
                self.pc = self.v_regs[0] + offset;
                return Ok(self.pc)
            },
            Instr::CALL { addr } => {
                self.stack.push(self.pc);
                self.pc = addr;
                return Ok(self.pc)
            },
            Instr::MV { dst, src } => self.v_regs[dst as usize] = self.v_regs[src as usize],
            Instr::LD_IMM { dst, imm } => self.v_regs[dst as usize] = imm as u16,
            Instr::LD_I { addr } => self.i_reg = addr,
            Instr::LD_DT { dst } => self.v_regs[dst as usize] = self.delay_timer as u16,
            Instr::LD_K { dst } => self.v_regs[dst as usize] = self.ui.wait_for_key_press(),
            Instr::LD_SPRITE { digit } => if digit < 16 {
                self.i_reg = self.digit_sprites[self.v_regs[digit as usize] as usize];
            } else {
                return Err("'LD F, Vx' with a Vx out of bounds")
            },
            Instr::LD_BCD { num } => {
                let num = self.v_regs[num as usize];
                self.memory[self.i_reg as usize + 0] = (num / 100) as u8;
                self.memory[self.i_reg as usize + 1] = (num / 10) as u8;
                self.memory[self.i_reg as usize + 2] = (num % 10) as u8;
            },
            Instr::LD_REGS_TO_I { upto } => {
                let mut pos = self.i_reg as usize;
                for vreg in 0..=upto {
                    let v: u16 = self.v_regs[vreg as usize];
                    self.memory[pos+0] = (v >> 8) as u8;
                    self.memory[pos+1] = (v & 0xff) as u8;
                    pos += 2
                }
            },
            Instr::LD_I_TO_REGS { upto } => {
                let mut pos = self.i_reg as usize;
                for vreg in 0..=upto {
                    let v = ((self.memory[pos+0] as u16) << 8) | (self.memory[pos+1] as u16);
                    self.v_regs[vreg as usize] = v;
                    pos += 2
                }
            },
            Instr::SET_DT { x } => self.delay_timer = self.v_regs[x as usize] as u8,
            Instr::SET_ST { x } => self.sound_timer = self.v_regs[x as usize] as u8,
            Instr::SE_IMM { x, imm } => if self.v_regs[x as usize] == imm as u16 {
                self.pc += 2
            },
            Instr::SNE_IMM { x, imm } => if self.v_regs[x as usize] != imm as u16 {
                self.pc += 2
            },
            Instr::SE_REG { x, y } => if self.v_regs[x as usize] == self.v_regs[y as usize] {
                self.pc += 2
            },
            Instr::SNE_REG { x, y } => if self.v_regs[x as usize] != self.v_regs[y as usize] {
                self.pc += 2
            },
            Instr::OR { x, y } => self.v_regs[x as usize] |= self.v_regs[y as usize],
            Instr::AND { x, y } => self.v_regs[x as usize] &= self.v_regs[y as usize],
            Instr::XOR { x, y } => self.v_regs[x as usize] ^= self.v_regs[y as usize],
            Instr::ADD { x, y } => {
                let v1 = self.v_regs[x as usize];
                let v2 = self.v_regs[y as usize];
                let res = v1 + v2;
                self.v_regs[x as usize] = res & 0xFF;
                self.v_regs[0xF] = (res > 0xFF) as u16;
            },
            Instr::ADD_IMM { dst, imm } => self.v_regs[dst as usize] += imm,
            Instr::ADD_I { x } => self.i_reg += self.v_regs[x as usize],
            Instr::SUB { x, y } => {
                let v1 = self.v_regs[x as usize];
                let v2 = self.v_regs[y as usize];
                self.v_regs[x as usize] = v1 - v2;
                self.v_regs[0xF] = (v1 > v2) as u16;
            },
            Instr::SHR { x, y: _ } => {
                let v1 = self.v_regs[x as usize];
                self.v_regs[0xF] = (v1 & 0x1) as u16;
                self.v_regs[x as usize] = v1 >> 1;
            },
            Instr::SUBN { x, y } => {
                let v1 = self.v_regs[x as usize];
                let v2 = self.v_regs[y as usize];
                self.v_regs[x as usize] = v2 - v1;
                self.v_regs[0xF] = (v2 > v1) as u16;
            },
            Instr::SHL { x, y: _ } => {
                let v1 = self.v_regs[x as usize];
                self.v_regs[0xF] = ((v1 & 0x80) != 0) as u16;
                self.v_regs[x as usize] = v1 << 1;
            },
            Instr::RND { dst, mask } => {
                self.v_regs[dst as usize] = (self.ui.rnd() & mask) as u16;
            },
            Instr::DRW { x, y, n } => {
                let x = self.v_regs[x as usize] as usize;
                let y = self.v_regs[y as usize] as usize;

                fn wrap(x: usize, dx: usize, n: usize) -> usize {
                    if x + dx > n { (x + dx) - n } else { x + dx }
                }

                let mut pixel_erased = false;
                for i in 0..(n as usize) {
                    let addr = self.i_reg as usize + i;
                    let b = self.memory[addr];
                    for j in 0..8 {
                        let x = wrap(x, j, 64);
                        let y = wrap(y, i, 32);
                        if (b & (1 << j)) == 0 {
                            continue
                        }

                        let prev_pixel = self.display[y][x];
                        let new_pixel = !prev_pixel;
                        pixel_erased |= !new_pixel;
                        self.display[y][x] = new_pixel;
                        self.ui.draw_pixel(x, y, new_pixel);
                    }
                }
                self.v_regs[0xF] = pixel_erased as u16;
            },
            Instr::SKP { x } => if self.ui.is_key_pressed(self.v_regs[x as usize]) {
                self.pc += 2
            }
            Instr::SKNP { x } => if !self.ui.is_key_pressed(self.v_regs[x as usize]) {
                self.pc += 2
            }
        }
        Ok(self.pc)
    }
}
