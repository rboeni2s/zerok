use super::{P, Synt};
use chumsky::prelude::*;


#[derive(Debug, Clone)]
pub enum Binop
{
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}


impl<'src> Binop
{
    pub fn add() -> impl P<'src, Self>
    {
        just(Synt::Plus.repr()).to(Self::Add)
    }

    pub fn sub() -> impl P<'src, Self>
    {
        just(Synt::Minus.repr()).to(Self::Sub)
    }

    pub fn mul() -> impl P<'src, Self>
    {
        just(Synt::Star.repr()).to(Self::Mul)
    }

    pub fn div() -> impl P<'src, Self>
    {
        just(Synt::Slash.repr()).to(Self::Div)
    }

    pub fn modulo() -> impl P<'src, Self>
    {
        just(Synt::Percent.repr()).to(Self::Mod)
    }

    pub fn pow() -> impl P<'src, Self>
    {
        just(Synt::StarStar.repr()).to(Self::Pow)
    }
}


#[derive(Debug, Clone)]
pub enum Unaop
{
    Neg,
}


impl<'src> Unaop
{
    pub fn neg() -> impl P<'src, Self>
    {
        just(Synt::Minus.repr()).padded().to(Self::Neg)
    }
}
