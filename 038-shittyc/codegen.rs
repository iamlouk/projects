use std::{collections::HashMap, fmt::Display, rc::Rc};

use crate::ast::{BinOp, Decl, Expr, Function, Stmt};

#[derive(Clone, Copy, PartialEq, PartialOrd, Hash)]
pub enum Reg {
    Zero,
    RA,
    SP,
    GP,
    TP,
    T0, T1, T2,
    FP,
    S1,
    A0, A1, A2, A3, A4, A5, A6, A7,
    S2, S3, S4, S5, S6, S7, S8, S9, S10, S11,
    T3, T4, T5, T6
}

impl Reg {
    fn as_str(&self) -> &'static str {
        use Reg::*;
        match self {
            Zero => "zero",
            RA => "ra", SP => "sp", GP => "gp", TP => "tp",
            T0 => "t0", T1 => "t1", T2 => "t2",
            FP => "fp", S1 => "s1",
            A0 => "a0", A1 => "a1", A2 => "a2", A3 => "a3",
            A4 => "a4", A5 => "a5", A6 => "a6", A7 => "a7",
            S2 => "s2", S3 => "s3", S4 => "s4", S5 => "s5",
            S6 => "s6", S7 => "s7", S8 => "s8", S9 => "s9",
            S10 => "s10", S11 => "s11",
            T3 => "t3", T4 => "t4", T5 => "t5", T6 => "t6"
        }
    }

    fn scratch_regs() -> &'static [Reg] { use Reg::*; return &[T0, T1]; }

    fn caller_save() -> &'static [Reg] {
        use Reg::*;
        return &[T2, /*A0, A1, A2, A3, A4, A5, A6, A7,*/ T3, T4, T5, T6];
    }

    fn callee_save() -> &'static [Reg] {
        use Reg::*;
        return &[S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11];
    }

    fn argument_regs() -> &'static [Reg] {
        use Reg::*;
        return &[A0, A1, A2, A3, A4, A5, A6, A7];
    }

    fn retval_reg() -> Reg { Reg::A0 }
}

impl Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(self.as_str()) }
}

pub struct CodeGen<'a> {
    regs: HashMap<Rc<Decl>, Reg>,
    out: Box<dyn std::io::Write + 'a>,
    label_cntr: usize,
}

impl<'a> CodeGen<'a> {
    pub fn new(out: Box<dyn std::io::Write + 'a>) -> CodeGen<'a> {
        CodeGen {
            regs: HashMap::new(),
            out,
            label_cntr: 0
        }
    }

    pub fn header(&mut self) -> Result<(), std::io::Error> {
        write!(self.out, "\t.option pic\n")?;
        write!(self.out, "\t.attribute arch, \"rv64i2p1_m2p0_a2p1_f2p2_d2p2_c2p0_zicsr2p0_zifencei2p0\"\n")?;
        write!(self.out, "\t.attribute unaligned_access, 0\n")?;
        write!(self.out, "\t.attribute stack_align, 16\n")?;
        write!(self.out, "\t.text\n")?;
        Ok(())
    }

    pub fn write(&mut self, fun: &Function) -> Result<(), std::io::Error> {
        // TODO: Handle spilling or put locals with non-overlapping lifetimes in same reg.
        // TODO: Use caller save regs and so on?
        assert!(fun.args.len() < Reg::argument_regs().len());
        assert!(fun.locals.len() < Reg::caller_save().len());

        self.regs.clear();
        self.label_cntr = 0;
        for (i, reg) in (0..fun.args.len()).zip(Reg::argument_regs().iter()) {
            let arg = &fun.locals[i];
            assert!(arg.is_argument);
            self.regs.insert(arg.clone(), *reg);
        }

        for (decl, reg) in fun.locals
                .iter().skip(fun.args.len())
                .zip(Reg::caller_save().iter()) {
            assert!(decl.is_local);
            self.regs.insert(decl.clone(), *reg);
        }

        let name = &*fun.name.clone();
        write!(self.out, "\n\t.globl {}\n", name)?;
        write!(self.out, "\t.type  {}, @function\n", name)?;
        write!(self.out, "{}:\n", name)?;
        self.stmt(fun.body.as_ref().unwrap().as_ref(), Reg::scratch_regs())?;
        write!(self.out, "\t.size  {}, .-{}\n", name, name)?;
        Ok(())
    }

    fn mov(&mut self, dst: Reg, src: Reg) -> Result<(), std::io::Error> {
        if dst != src {
            write!(self.out, "\tmv {}, {}\n", dst, src)?;
        }
        Ok(())
    }

    fn push(&mut self, regs: &[Reg]) -> Result<(), std::io::Error> {
        for (i, reg) in regs.iter().enumerate() {
            write!(self.out, "\tsd {}, -{}({})\n", reg, 8 * (i + 1), Reg::SP)?;
        }
        write!(self.out, "\taddi {}, {}, -{}\n", Reg::SP, Reg::SP, regs.len() * 8)?;
        Ok(())
    }

    fn pop(&mut self, regs: &[Reg]) -> Result<(), std::io::Error> {
        for (i, reg) in regs.iter().enumerate() {
            write!(self.out, "\tld {}, {}({})\n", reg, 8 * i, Reg::SP)?;
        }
        write!(self.out, "\taddi {}, {}, {}\n", Reg::SP, Reg::SP, regs.len() * 8)?;
        Ok(())
    }

    // FIXME: truncations/extensions/non-word sized arith.
    fn expr(&mut self, expr: &Expr, dst: Reg, scratch: &[Reg]) -> Result<(), std::io::Error> {
        match expr {
            Expr::Id { decl, .. } => match self.regs.get(decl).cloned() {
                Some(reg) => self.mov(dst, reg),
                None => unimplemented!()
            },
            Expr::Int { num: 0, .. } => self.mov(dst, Reg::Zero),
            Expr::Int { num, .. } => write!(self.out, "\tli {}, {}\n", dst, num),
            Expr::BinOp { op: BinOp::Add, lhs, rhs, .. } if rhs.is_constant(512).is_some() => {
                self.expr(lhs.as_ref(), dst, scratch)?;
                write!(self.out, "\taddi {}, {}, {}\n", dst, dst, rhs.is_constant(512).unwrap())
            },
            Expr::BinOp { op, lhs, rhs, .. } => {
                self.expr(lhs, scratch[0], scratch)?;
                self.push(&[scratch[0]])?;
                self.expr(rhs, scratch[1], scratch)?;
                self.pop(&[scratch[0]])?;
                match op {
                    BinOp::Add => write!(self.out, "\tadd {}, {}, {}\n", dst, scratch[0], scratch[1]),
                    BinOp::Sub => write!(self.out, "\tsub {}, {}, {}\n", dst, scratch[0], scratch[1]),
                    BinOp::LT  => write!(self.out, "\tslt {}, {}, {}\n", dst, scratch[0], scratch[1]),
                    _ => unimplemented!()
                }
            },
            _ => unimplemented!()
        }
    }

    fn stmt(&mut self, stmt: &Stmt, scratch: &[Reg]) -> Result<(), std::io::Error> {
        match stmt {
            Stmt::NoOp {..} => Ok(()),
            Stmt::Compound { stmts, .. } => {
                for stmt in stmts {
                    self.stmt(stmt, scratch)?;
                }
                Ok(())
            },
            Stmt::Expr { expr, .. } => self.expr(expr, Reg::Zero, scratch),
            Stmt::Ret { val: None, .. } => write!(self.out, "\tret\n"),
            Stmt::Ret { val: Some(val), .. } => {
                self.expr(val.as_ref(), Reg::retval_reg(), scratch)?;
                write!(self.out, "\tret\n")
            },
            Stmt::If { cond, then, otherwise: None, .. } => {
                let id = self.label_cntr;
                self.label_cntr += 1;
                self.expr(cond.as_ref(), scratch[0], scratch)?;
                write!(self.out, "\tbeq {}, zero, .BB{}\n", scratch[0], id)?;
                self.stmt(then.as_ref(), scratch)?;
                write!(self.out, ".BB{}:\n", id)
            },
            _ => unimplemented!()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lex::*;
    use crate::ast::*;

    fn parse_func(input: &str) -> Rc<Function> {
        let buf = input.as_bytes().to_vec();
        let mut lex = Lexer::new(std::path::Path::new("text.c"), &buf);
        let mut p = Parser::new();
        p.parse_function(&mut lex).unwrap().unwrap()
    }

    fn codegen(input: &str) -> String {
        let f = parse_func(input);
        let mut buf = Vec::new();
        {
            let mut cg = CodeGen::new(Box::new(&mut buf));
            cg.write(f.as_ref()).unwrap();
        }
        std::str::from_utf8(buf.as_slice()).unwrap().to_string()
    }

    /*
    #[test]
    fn add() {
        let res = codegen("long add(long x, long y) { return x + y; }");
        let res = res.replace("\t", "    ");
        assert_eq!(res.as_str(),r#"
    .globl add
    .type  add, @function
add:
    mv t0, a0
    sd t0, -8(sp)
    addi sp, sp, -8
    mv t1, a1
    ld t0, 8(sp)
    addi sp, sp, 8
    add a0, t0, t1
    ret
    .size  add, .-add
"#);
    }
    */
}
