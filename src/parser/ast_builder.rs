pub use super::atom::Atom;
pub use crate::chain;
pub use crate::parser::Expr;
pub use crate::parser::Node;
pub use crate::parser::ops::{Binop, Unaop};


#[macro_export]
macro_rules! chain {
    ($($e:expr),*) => {
        Expr::Chain(vec![$($e),*]).into()
    };
}


pub fn num<'a, D: Default>(n: impl Into<f64>) -> Node<'a, D>
{
    Expr::Atom(Atom::Num(n.into())).into()
}


pub fn nop<'a, D: Default>() -> Node<'a, D>
{
    Expr::Atom(Atom::Nop).into()
}


pub fn atom_str<'a, D: Default>(s: impl Into<String>) -> Node<'a, D>
{
    Expr::Atom(Atom::Str(s.into())).into()
}


pub fn decl<'a, D: Default>(name: &'a str, kind: &'a str, expr: Node<'a, D>) -> Node<'a, D>
{
    Expr::Decl {
        name,
        kind,
        val: Box::new(expr),
    }
    .into()
}


pub fn binop<'a, D: Default>(op: Binop, lhs: Node<'a, D>, rhs: Node<'a, D>) -> Node<'a, D>
{
    Expr::Binop {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    }
    .into()
}


pub fn unaop<'a, D: Default>(op: Unaop, val: Node<'a, D>) -> Node<'a, D>
{
    Expr::Unaop {
        op,
        val: Box::new(val),
    }
    .into()
}


pub fn add<'a, D: Default>(lhs: Node<'a, D>, rhs: Node<'a, D>) -> Node<'a, D>
{
    binop(Binop::Add, lhs, rhs)
}


pub fn sub<'a, D: Default>(lhs: Node<'a, D>, rhs: Node<'a, D>) -> Node<'a, D>
{
    binop(Binop::Sub, lhs, rhs)
}


pub fn mul<'a, D: Default>(lhs: Node<'a, D>, rhs: Node<'a, D>) -> Node<'a, D>
{
    binop(Binop::Mul, lhs, rhs)
}


pub fn div<'a, D: Default>(lhs: Node<'a, D>, rhs: Node<'a, D>) -> Node<'a, D>
{
    binop(Binop::Div, lhs, rhs)
}


pub fn modulo<'a, D: Default>(lhs: Node<'a, D>, rhs: Node<'a, D>) -> Node<'a, D>
{
    binop(Binop::Mod, lhs, rhs)
}


pub fn pow<'a, D: Default>(lhs: Node<'a, D>, rhs: Node<'a, D>) -> Node<'a, D>
{
    binop(Binop::Pow, lhs, rhs)
}


pub fn neg<'a, D: Default>(val: Node<'a, D>) -> Node<'a, D>
{
    unaop(Unaop::Neg, val)
}
