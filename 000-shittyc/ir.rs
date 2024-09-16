/*
use std::{cell::RefCell, fmt::Display, rc::{Rc, Weak}};

use crate::common::Type;

pub type Reg = u32;

pub struct Local {
    pub slot: isize,
    pub name: Option<Rc<str>>,
    pub typp: Type,
    pub regi: Option<Reg>
}

impl Display for Local {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "%{}:{}", self.name.as_ref().map(|x| &**x).unwrap_or("v"), self.typp)?;
        if let Some(reg) = self.regi { write!(f, "(reg={})", reg)?; }
        Ok(())
    }
}

pub enum OpCode {
    NoOp,
    Plus,
    Minus,
    Call,
    LowerThen,
    Br,
    CondBr,
    Ret
}

pub struct Inst {
    pub dst: Option<Weak<RefCell<Local>>>,
    pub ops: Vec<Rc<RefCell<Local>>>,
    pub opcode: OpCode,
    pub block: Option<Weak<RefCell<Block>>>
}

impl Display for Inst {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(dst) = self.dst.clone() {
            write!(f, "{} = ", dst.upgrade().unwrap().borrow())?;
        }
        let opcode = match self.opcode {
            OpCode::NoOp => "nop",
            OpCode::Plus => "add",
            OpCode::Minus => "minus",
            OpCode::Call => "call",
            OpCode::LowerThen => "ult",
            OpCode::Br => "jmp",
            OpCode::CondBr => "jmp-if",
            OpCode::Ret => "ret"
        };
        for (i, op) in self.ops.iter().enumerate() {
            write!(f, "{}{}", if i == 0 { " " } else { ", " }, op.borrow())?;
        }
        Ok(())
    }
}

pub struct Block {
    pub idx: usize,
    pub name: Option<Rc<str>>,
    pub instrs: Vec<Inst>,
    pub preds: Vec<Weak<RefCell<Block>>>,
    pub succs: Vec<Rc<RefCell<Block>>>
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:\n\t# preds: {{", self.idx)?;
        for (i, pred) in self.preds.iter().enumerate() {
            write!(f, "{}{}", if i == 0 { " " } else { ", " }, pred.upgrade().unwrap().borrow().idx)?;
        }
        write!(f, "}}\n")?;
        for inst in self.instrs.iter() {
            write!(f, "\t{}\n", inst)?;
        }
        write!(f, "\t# succs: {{")?;
        for (i, succ) in self.succs.iter().enumerate() {
            write!(f, "{}{}", if i == 0 { " " } else { ", " }, succ.borrow().idx)?;
        }
        write!(f, "}}\n")
    }
}

pub struct Function {
    pub exported: bool,
    pub name: Rc<str>,
    pub args: Vec<Rc<RefCell<Local>>>,
    pub locals: Vec<Rc<RefCell<Local>>>,
    pub entry: Rc<Block>
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}fn '{}'(", if self.exported { "export " } else { "" }, &*self.name)?;
        unimplemented!();
        Ok(())
    }
}
*/
