use super::{P, Token};
use chumsky::prelude::*;
use std::fmt;


/// A path to a symbol like: `ice::print` or `add`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<'src>
{
    /// The modules in front of the name, empty for paths like `add`
    pub module: Vec<&'src str>,
    pub name: &'src str,
}


impl<'src> Path<'src>
{
    /// A path without modules
    pub fn name(name: &'src str) -> Self
    {
        Self {
            module: Vec::new(),
            name,
        }
    }

    /// Parses `ident (:: ident)*`
    pub fn parser<'t>() -> impl P<'t, 'src, Self>
    where
        'src: 't,
    {
        let ident = select! { Token::Ident(name) => name }.labelled("identifier");

        ident
            .separated_by(just(Token::PathSep))
            .at_least(1)
            .collect::<Vec<_>>()
            .map(|mut segments| {
                let name = segments.pop().expect("A path has at least one segment");
                Self {
                    module: segments,
                    name,
                }
            })
            .labelled("path")
    }
}


impl fmt::Display for Path<'_>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        for module in &self.module
        {
            write!(f, "{module}::")?;
        }

        write!(f, "{}", self.name)
    }
}
