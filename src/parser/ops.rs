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
}


#[derive(Debug, Clone, PartialEq)]
pub enum Unaop
{
    Neg,
}


impl Unaop
{
    pub fn neg<'t, 's: 't>() -> impl P<'t, 's, Self>
    {
        just(Token::Minus).to(Self::Neg)
    }
}
