use super::{P, Token};
use chumsky::prelude::*;
use std::fmt;


#[derive(Debug, Clone, PartialEq)]
pub enum Atom
{
    Str(String),
    Int(u64),
    Float(f64),
    Nop,
}


impl Atom
{
    pub fn parser<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        select! {
            Token::Str(s) => Self::Str(s.to_string()),
            Token::Num(n) => Self::Int(n),
            Token::Float(n) => Self::Float(n),
            Token::Nop => Self::Nop,
        }
        .labelled("value")
    }
}


impl fmt::Display for Atom
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Atom::Str(s) => write!(f, "\"{s}\""),
            Atom::Int(n) => write!(f, "{n}"),
            Atom::Float(n) => write!(f, "{n}f"),
            Atom::Nop => write!(f, "nop"),
        }
    }
}
