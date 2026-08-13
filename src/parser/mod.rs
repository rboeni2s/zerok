mod atom;
mod expr;
mod ops;


use chumsky::prelude::*;


pub use expr::Expr;


pub trait P<'a, T>: Parser<'a, &'a str, T, extra::Err<Rich<'a, char>>> + Clone {}
impl<'a, T, I> P<'a, T> for I where I: Parser<'a, &'a str, T, extra::Err<Rich<'a, char>>> + Clone {}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Synt
{
    Semicolon,
    Plus,
    Star,
    Nop,
    Minus,
    Slash,
    LParen,
    RParen,
    Percent,
    StarStar,
}


impl Synt
{
    fn repr(&self) -> &'static str
    {
        match self
        {
            Synt::Semicolon => ";",
            Synt::Nop => "nop",
            Synt::Plus => "+",
            Synt::Star => "*",
            Synt::Minus => "-",
            Synt::Slash => "/",
            Synt::LParen => "(",
            Synt::RParen => ")",
            Synt::Percent => "%",
            Synt::StarStar => "**",
        }
    }
}
