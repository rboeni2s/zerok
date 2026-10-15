//! Helpers to build expected asts in tests

// Not every test file uses every helper
#![allow(dead_code)]

pub use zerok::parser::atom::Atom;
pub use zerok::parser::ops::{Binop, Unaop};
pub use zerok::parser::{Expr, Function, Node, Param, Program, Type};


macro_rules! chain {
    ($($e:expr),*) => {
        zerok::parser::Expr::Chain(vec![$($e),*]).into()
    };
}
pub(crate) use chain;


pub fn num<'a, D: Default>(n: u64) -> Node<'a, D>
{
    Expr::Atom(Atom::Int(n)).into()
}


pub fn float<'a, D: Default>(n: f64) -> Node<'a, D>
{
    Expr::Atom(Atom::Float(n)).into()
}


pub fn boolean<'a, D: Default>(b: bool) -> Node<'a, D>
{
    Expr::Atom(Atom::Bool(b)).into()
}


pub fn nop<'a, D: Default>() -> Node<'a, D>
{
    Expr::Atom(Atom::Nop).into()
}


pub fn atom_str<'a, D: Default>(s: impl Into<String>) -> Node<'a, D>
{
    Expr::Atom(Atom::Str(s.into())).into()
}


pub fn decl<'a, D: Default>(
    name: &'a str,
    kind: impl Into<Type<'a>>,
    expr: Node<'a, D>,
) -> Node<'a, D>
{
    Expr::Decl {
        name,
        kind: kind.into(),
        val: Box::new(expr),
    }
    .into()
}


pub fn binding<'a, D: Default>(name: &'a str) -> Node<'a, D>
{
    Expr::Binding { name }.into()
}


pub fn call<'a, D: Default>(name: &'a str, args: Vec<Node<'a, D>>) -> Node<'a, D>
{
    Expr::Call { name, args }.into()
}


/// Builds a function, `params` are pairs of names and types
pub fn function<'a, D: Default>(
    name: &'a str,
    params: &[(&'a str, Type<'a>)],
    ret: Option<Type<'a>>,
    body: Node<'a, D>,
) -> Function<'a, D>
{
    Function {
        name,
        params: params
            .iter()
            .map(|(name, kind)| {
                Param {
                    name,
                    kind: kind.clone(),
                    span: Default::default(),
                    data: Default::default(),
                }
            })
            .collect(),
        ret,
        body,
        span: Default::default(),
    }
}


pub fn if_<'a, D: Default>(
    condition: Node<'a, D>,
    body: Node<'a, D>,
    elifs: Vec<(Node<'a, D>, Node<'a, D>)>,
    else_body: Option<Node<'a, D>>,
) -> Node<'a, D>
{
    Expr::If {
        condition: Box::new(condition),
        body: Box::new(body),
        elifs,
        else_body: else_body.map(Box::new),
    }
    .into()
}


pub fn ret<'a, D: Default>(val: Option<Node<'a, D>>) -> Node<'a, D>
{
    Expr::Return {
        val: val.map(Box::new),
    }
    .into()
}


pub fn cast<'a, D: Default>(val: Node<'a, D>, kind: impl Into<Type<'a>>) -> Node<'a, D>
{
    Expr::Cast {
        val: Box::new(val),
        kind: kind.into(),
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


pub fn reference<'a, D: Default>(val: Node<'a, D>) -> Node<'a, D>
{
    unaop(Unaop::Ref, val)
}


/// The type "&kind"
pub fn ref_type<'a>(kind: impl Into<Type<'a>>) -> Type<'a>
{
    Type::Ref(Box::new(kind.into()))
}
