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


/// A register that is written to, together with the type of the value it holds
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct IrTypedReg<K>
{
    pub reg: IrReg,
    pub kind: K,
}


#[derive(Debug, Clone, PartialEq)]
pub enum IrChunk<T, K>
{
    Bin
    {
        op: IrOp,
        lhs: T,
        rhs: T,
        reg: IrTypedReg<K>,
    },

    Una
    {
        op: IrOp,
        arg: T,
        reg: IrTypedReg<K>,
    },

    Bind
    {
        val: T, reg: IrTypedReg<K>
    },

    Func
    {
        name: String,
        args: Vec<IrTypedReg<K>>,
    },

    Call
    {
        name: String,
        args: Vec<T>,
        reg: IrTypedReg<K>,
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
