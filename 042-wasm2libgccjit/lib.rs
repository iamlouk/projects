#![feature(buf_read_has_data_left)]

#[allow(clippy::upper_case_acronyms)]
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum SectionID {
	CUSTOM     = 0,
	TYPE       = 1,
	IMPORT     = 2,
	FUNCTION   = 3,
	TABLE      = 4,
	MEMORY     = 5,
	GLOBAL     = 6,
	EXPORT     = 7,
	START      = 8,
	ELEMENT    = 9,
	CODE       = 10,
	DATA       = 11,
	DATACOUNT  = 12,
}

impl std::fmt::Display for SectionID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SectionID::*;
        let s = match self {
            CUSTOM =>    "custom", TYPE =>    "type",    IMPORT => "import", FUNCTION => "function",
            TABLE =>     "table",  MEMORY =>  "memory",  GLOBAL => "global", EXPORT =>   "export",
            START =>     "start",  ELEMENT => "element", CODE =>   "code",   DATA =>     "data",
            DATACOUNT => "count",
        };
        f.write_str(s)
    }
}

impl std::convert::TryFrom<u8> for SectionID {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 12 {
            return Err(());
        }
        unsafe { std::mem::transmute(value) }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum Type {
    I32,
    I64,
    F32,
    F64,
    Func(Vec<Type>, Vec<Type>)
}

impl Type {
    fn parse(r: &mut dyn std::io::BufRead) -> std::io::Result<Type> {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        match buf[0] {
            0x60 => {
                let mut pair: [Option<Vec<Type>>; 2] = [None, None];
                for i in 0..2 {
                    let n = read_unsigned_leb128(r)?.0 as usize;
                    let mut v = vec![];
                    for _ in 0..n {
                        v.push(Self::parse(r)?);
                    }
                    pair[i] = Some(v);
                }
                Ok(Type::Func(pair[0].take().unwrap(), pair[1].take().unwrap()))
            }
            0x7F => Ok(Type::I32),
            0x7E => Ok(Type::I64),
            0x7D => Ok(Type::F32),
            0x7C => Ok(Type::F64),
            b => Err(std::io::Error::other(format!("unknown type: {:?}", b)))
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
struct Function {
    index: usize,
    size: usize,
    arguments: Vec<Type>,
    returns: Vec<Type>,
    locals: Vec<(usize, Type)>,
    body: Vec<Instr>
}

#[allow(dead_code)]
#[derive(Debug)]
enum Export { Func, Table, Memory, Global }

#[allow(dead_code)]
#[derive(Debug)]
pub struct Module {
    version: u32,
    function_types: Vec<(Vec<Type>, Vec<Type>)>,
    function_type_indexes: Vec<usize>,
    memory_ranges: Vec<(usize, Option<usize>)>,
    custom_sections: Vec<(usize, String, Vec<u8>)>,
    globals: Vec<(Type, bool, Vec<Instr>)>,
    exports: Vec<(String, Export, usize)>,
    functions: Vec<Function>
}

impl Module {
    pub fn parse(r: &mut dyn std::io::BufRead) -> std::io::Result<Module> {
        let mut buf = [0x0u8; 8];
        r.read_exact(&mut buf)?;
        if buf != [b'\0', b'a', b's', b'm', 1u8, 0u8, 0u8, 0u8] {
            return Err(std::io::Error::other("bad magic number or version"))
        }

        let mut m = Module {
            version: 1,
            function_types: vec![],
            function_type_indexes: vec![],
            memory_ranges: vec![],
            custom_sections: vec![],
            globals: vec![],
            exports: vec![],
            functions: vec![]
        };

        while r.has_data_left()? {
            m.parse_section(r)?
        }

        Ok(m)
    }

    fn parse_section(&mut self, r: &mut dyn std::io::BufRead) -> std::io::Result<()> {
        let mut buf = [0x0u8; 1];
        r.read_exact(&mut buf[0..1])?;
        let size = read_unsigned_leb128(r)?.0 as usize;

        if buf[0] == (SectionID::CUSTOM as u8) {
            let (key_len, n) = read_unsigned_leb128(r)?;
            let mut key_buf = vec![0u8; key_len as usize];
            r.read_exact(&mut key_buf[0..key_len as usize])?;
            let val_len = size - (n + key_len as usize);
            let mut val_buf = vec![0u8; val_len];
            r.read_exact(&mut val_buf[0..val_len])?;
            self.custom_sections.push((size,
                String::from_utf8(key_buf)
                    .map_err(std::io::Error::other)?, val_buf));
            return Ok(())
        }

        if buf[0] == (SectionID::TYPE as u8) {
            assert!(self.function_types.is_empty());
            let n = read_unsigned_leb128(r)?.0 as usize;
            for _ in 0 ..n {
                match Type::parse(r)? {
                    Type::Func(a, b) => 
                        self.function_types.push((a, b)),
                    t => return Err(std::io::Error::other(
                        format!("expected a function type, not {:?}", t)))
                }
            }
            return Ok(())
        }

        if buf[0] == (SectionID::FUNCTION as u8) {
            assert!(self.function_type_indexes.is_empty());
            let n = read_unsigned_leb128(r)?.0 as usize;
            for _ in 0 ..n {
                let idx = read_unsigned_leb128(r)?.0 as usize;
                assert!(idx < self.function_types.len());
                self.function_type_indexes.push(idx);
            }
            return Ok(())
        }

        if buf[0] == (SectionID::MEMORY as u8) {

        }

        Err(std::io::Error::other(format!("unknown section ID {:?}", buf[0])))
    }
}

#[allow(dead_code)]
#[derive(Clone, PartialEq)]
pub enum Instr {
    Unreachable,
    NOp,
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
    End
}

impl std::fmt::Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Instr::*;
        match self {
            Unreachable => f.write_str("(unreachable)"),
            NOp => f.write_str("(nop)"),
            LocalGet(idx) => write!(f, "(local.get {})", idx),
            LocalSet(idx) => write!(f, "(local.set {})", idx),
            LocalTee(idx) => write!(f, "(local.tree {})", idx),
            GlobalGet(idx) => write!(f, "(global.get {})", idx),
            GlobalSet(idx) => write!(f, "(global.set {})", idx),
            I32Const(c) => write!(f, "(i32.const {:x})", c),
            I64Const(c) => write!(f, "(i64.const {:x})", c),
            F32Const(c) => write!(f, "(f32.const {})", c),
            F64Const(c) => write!(f, "(f64.const {})", c),
            End => f.write_str("(end)"),
        }
    }
}

impl std::fmt::Debug for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl Instr {
    fn parse(r: &mut dyn std::io::BufRead) -> std::io::Result<Instr> {
        let mut buf: [u8; 1] = [0];
        r.read_exact(&mut buf)?;
        Ok(match buf[0] {
            0x00 => Instr::Unreachable,
            0x01 => Instr::NOp,

            0x20 => Instr::LocalGet(read_unsigned_leb128(r)?.0 as u32),
            0x21 => Instr::LocalSet(read_unsigned_leb128(r)?.0 as u32),
            0x22 => Instr::LocalTee(read_unsigned_leb128(r)?.0 as u32),
            0x23 => Instr::GlobalGet(read_unsigned_leb128(r)?.0 as u32),
            0x24 => Instr::GlobalSet(read_unsigned_leb128(r)?.0 as u32),

            0x41 => Instr::I32Const(read_signed_leb128(r, 32)? as i32),
            0x42 => Instr::I64Const(read_signed_leb128(r, 64)? as i64),

            0x0b => Instr::End,

            opc => return Err(std::io::Error::other(format!("unimplemented instruction opcode: {:x}", opc)))
        })
    }
}

fn read_unsigned_leb128(r: &mut dyn std::io::BufRead) -> std::io::Result<(u64, usize)> {
    let mut res: u64 = 0;
    let mut shift: u64 = 0;
    let mut bytes: usize = 0;
    loop {
        let mut buf: [u8; 1] = [0];
        r.read_exact(&mut buf)?;
        bytes += 1;
        res |= (buf[0] as u64 & 0x7f) << shift;
        if (buf[0] & 0x80) == 0 {
            return Ok((res, bytes))
        }
        shift += 7;
    }
}

fn read_signed_leb128(r: &mut dyn std::io::BufRead, bits: i64) -> std::io::Result<i64> {
    let mut res: i64 = 0;
    let mut shift: i64 = 0;
    loop {
        let mut buf: [u8; 1] = [0];
        r.read_exact(&mut buf)?;
        res |= (buf[0] as i64 & 0x7f) << shift;
        if (buf[0] & 0x80) == 0 {
            return Ok(if buf[0] & 0x40 != 0 && shift < bits {
                res | (-1 << shift)
            } else {
                res
            })
        }
        shift += 7;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_id_module() {
        let p = std::path::Path::new("./tests/id.wasm");
        let f = std::fs::File::open(p).unwrap();
        let mut buffered = std::io::BufReader::new(f);
        let m = Module::parse(&mut buffered).unwrap();

        println!("{:?}", m);
    }
}
