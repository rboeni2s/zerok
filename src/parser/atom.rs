use super::{P, Token};
use chumsky::prelude::*;


#[derive(Debug, Clone, PartialEq)]
pub enum Atom
{
    Str(String),
    Num(f64),
    Nop,
}


impl Atom
{
    pub fn parser<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        select! {
            Token::Str(s) => Self::Str(s.to_string()),
            Token::Num(n) => Self::Num(n),
            Token::Nop => Self::Nop,
        }
        .labelled("value")
    }
}
