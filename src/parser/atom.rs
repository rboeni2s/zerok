use super::{P, Synt};
use chumsky::prelude::*;


#[derive(Debug, Clone, PartialEq)]
pub enum Atom
{
    Str(String),
    Num(f64),
    Nop,
}


impl<'src> Atom
{
    pub fn parser() -> impl P<'src, Self>
    {
        let string = one_of("\"'")
            .ignore_then(none_of("\"'").repeated().collect::<String>())
            .then_ignore(one_of("\"'"))
            .padded()
            .map(Self::Str);

        let num = text::int(10).padded().from_str().unwrapped().map(Self::Num);

        choice((
            string,
            num,
            just(Synt::Nop.repr()).padded().map(|_| Self::Nop),
        ))
    }
}
