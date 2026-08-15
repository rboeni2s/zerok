pub use super::atom::Atom;
pub use crate::chain;
pub use crate::parser::Expr;
pub use crate::parser::ops::{Binop, Unaop};


#[macro_export]
macro_rules! chain {
    ($($e:expr),*) => {
        Expr::Chain(vec![$($e),*])
    };
}


pub fn num<'a>(n: impl Into<f64>) -> Expr<'a>
{
    Expr::Atom(Atom::Num(n.into()))
}


pub fn nop<'a>() -> Expr<'a>
{
    Expr::Atom(Atom::Nop)
}


pub fn atom_str<'a>(s: impl Into<String>) -> Expr<'a>
{
    Expr::Atom(Atom::Str(s.into()))
}


pub fn decl<'a>(name: &'a str, kind: &'a str, expr: Expr<'a>) -> Expr<'a>
{
    Expr::Decl {
        name,
        kind,
        val: Box::new(expr),
    }
}


pub fn binop<'a>(op: Binop, lhs: Expr<'a>, rhs: Expr<'a>) -> Expr<'a>
{
    Expr::Binop {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    }
}


pub fn unaop<'a>(op: Unaop, val: Expr<'a>) -> Expr<'a>
{
    Expr::Unaop {
        op,
        val: Box::new(val),
    }
}


pub fn add<'a>(lhs: Expr<'a>, rhs: Expr<'a>) -> Expr<'a>
{
    binop(Binop::Add, lhs, rhs)
}


pub fn sub<'a>(lhs: Expr<'a>, rhs: Expr<'a>) -> Expr<'a>
{
    binop(Binop::Sub, lhs, rhs)
}


pub fn mul<'a>(lhs: Expr<'a>, rhs: Expr<'a>) -> Expr<'a>
{
    binop(Binop::Mul, lhs, rhs)
}


pub fn div<'a>(lhs: Expr<'a>, rhs: Expr<'a>) -> Expr<'a>
{
    binop(Binop::Div, lhs, rhs)
}


pub fn modulo<'a>(lhs: Expr<'a>, rhs: Expr<'a>) -> Expr<'a>
{
    binop(Binop::Mod, lhs, rhs)
}


pub fn pow<'a>(lhs: Expr<'a>, rhs: Expr<'a>) -> Expr<'a>
{
    binop(Binop::Pow, lhs, rhs)
}


pub fn neg<'a>(val: Expr<'a>) -> Expr<'a>
{
    unaop(Unaop::Neg, val)
}
