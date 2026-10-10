mod builder;
mod gen_ir;
mod iic;


pub use builder::IrBuilder;
pub use gen_ir::generate_ir;
pub use iic::ToIIC;


use crate::annotator::EnvEntry;


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IrOp
{
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Lte,
    Eq,
    Neq,
    Gt,
    Gte,
    And,
    Or,
}


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IrReg
{
    R(usize),
    G(usize),
}


#[derive(Debug, Clone, PartialEq)]
pub enum IrChunk
{
    Bin
    {
        op: IrOp,
        lhs: EnvEntry,
        rhs: EnvEntry,
        reg: IrReg,
    },

    Una
    {
        op: IrOp, arg: EnvEntry, reg: IrReg
    },

    Bind
    {
        val: EnvEntry, reg: IrReg
    },

    Func
    {
        name: String, args: Vec<IrReg>
    },

    Call
    {
        name: String,
        args: Vec<EnvEntry>,
        reg: IrReg,
    },

    Ret
    {
        val: EnvEntry
    },
}
