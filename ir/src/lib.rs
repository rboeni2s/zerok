#![feature(iter_intersperse)]

mod builder;
mod iic;

pub use builder::IrBuilder;
pub use iic::ToIIC;


pub trait ToIrVal<T>
{
    fn to_ir_val(self) -> T;
}


/// Values that already are ir values (e.g. computed operands) are passed through
impl<T> ToIrVal<T> for T
{
    fn to_ir_val(self) -> T
    {
        self
    }
}


pub trait ToIrReg<T>
{
    fn to_ir_reg(self) -> T;
}


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
