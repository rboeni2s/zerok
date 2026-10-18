use super::{P, Token};
use chumsky::prelude::*;
use std::fmt;


/// A type as it is written in the source, e.g. "u32" or "&u32"
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type<'src>
{
    /// A named type, e.g. "u32"
    Name(&'src str),

    /// A reference to a value of the inner type on the heap, e.g. "&u32"
    Ref(Box<Type<'src>>),

    /// A reference to a vector on the heap
    Vector(Box<Type<'src>>),
}


impl<'src> Type<'src>
{
    pub fn parser<'t>() -> impl P<'t, 'src, Self>
    where
        'src: 't,
    {
        let ident = select! { Token::Ident(name) => name }.labelled("type");

        recursive(|ty| {
            choice((
                // [u32]
                ty.clone()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .map(|ty| Self::Vector(Box::new(ty))),
                // &u32
                just(Token::Ref)
                    .ignore_then(ty.clone())
                    .map(|ty| Self::Ref(Box::new(ty))),
                // "&&" is lexed as one token, so "&&u32" is a reference to a reference
                just(Token::BoolAnd)
                    .ignore_then(ty)
                    .map(|ty| Self::Ref(Box::new(Self::Ref(Box::new(ty))))),
                ident.map(Self::Name),
            ))
            .boxed()
        })
        .boxed()
    }
}


impl<'src> From<&'src str> for Type<'src>
{
    fn from(name: &'src str) -> Self
    {
        Self::Name(name)
    }
}


impl fmt::Display for Type<'_>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Type::Name(name) => write!(f, "{name}"),
            Type::Ref(inner) => write!(f, "&{inner}"),
            Type::Vector(element) => write!(f, "[{element}]"),
        }
    }
}
