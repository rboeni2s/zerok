#![feature(iter_intersperse)]

mod builder;
mod iic;

pub use builder::IrBuilder;
pub use iic::ToIIC;


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
    Not,
}


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IrReg
{
    R(usize),
    G(usize),
}


#[derive(Debug, Clone, PartialEq)]
pub enum IrChunk<T>
{
    Bin
    {
        op: IrOp,
        lhs: T,
        rhs: T,
        reg: IrReg,
    },

    Una
    {
        op: IrOp, arg: T, reg: IrReg
    },

    Bind
    {
        val: T, reg: IrReg
    },

    Func
    {
        name: String, args: Vec<IrReg>
    },

    Call
    {
        name: String,
        args: Vec<T>,
        reg: IrReg,
    },

    Ret
    {
        val: T
    },

    Label
    {
        name: String
    },

    Jump
    {
        name: String
    },

    CondJump
    {
        val: T, name: String
    },
}
