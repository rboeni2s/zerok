use super::{P, Token};
use chumsky::prelude::*;


#[derive(Debug, Clone, PartialEq)]
pub enum Binop
{
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    BoolAnd,
    BoolOr,
}


impl Binop
{
    pub fn add<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::Plus).to(Self::Add)
    }

    pub fn sub<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::Minus).to(Self::Sub)
    }

    pub fn mul<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::Star).to(Self::Mul)
    }

    pub fn div<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::Slash).to(Self::Div)
    }

    pub fn modulo<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::Percent).to(Self::Mod)
    }

    pub fn pow<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::StarStar).to(Self::Pow)
    }

    pub fn bool_and<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::BoolAnd).to(Self::BoolAnd)
    }

    pub fn bool_or<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::BoolOr).to(Self::BoolOr)
    }

    pub fn compare<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        choice((
            just(Token::EqEq).to(Self::Eq),
            just(Token::NotEq).to(Self::Neq),
            just(Token::Lt).to(Self::Lt),
            just(Token::Gt).to(Self::Gt),
            just(Token::Lte).to(Self::Lte),
            just(Token::Gte).to(Self::Gte),
        ))
    }

    /// Comparisons always return a bool, independent of the type of their operands
    pub fn is_comparison(&self) -> bool
    {
        matches!(
            self,
            Self::Eq | Self::Neq | Self::Lt | Self::Gt | Self::Lte | Self::Gte
        )
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum Unaop
{
    Neg,
    BoolNot,
}


impl Unaop
{
    pub fn neg<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::Minus).to(Self::Neg)
    }

    pub fn bool_not<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::BoolNot).to(Self::BoolNot)
    }
}
