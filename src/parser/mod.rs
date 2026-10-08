pub mod ast;
pub mod ast_builder;
pub mod atom;
pub mod expr;
pub mod ops;


use anyhow::Context;
use chumsky::prelude::*;
use std::{marker::PhantomData, range::Range};


pub use expr::Expr;


pub trait P<'a, T>: Parser<'a, &'a str, T, extra::Err<Rich<'a, char>>> + Clone {}
impl<'a, T, I> P<'a, T> for I where I: Parser<'a, &'a str, T, extra::Err<Rich<'a, char>>> + Clone {}


pub type Node<'a, D> = ast::AstNode<'a, Expr<'a, D>, D>;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Synt
{
    Semicolon,
    Doublecolon,
    Plus,
    Star,
    Nop,
    Minus,
    Slash,
    LParen,
    RParen,
    Percent,
    StarStar,
    RBrace,
    LBrace,
    Comma,
    Decl,
    Eq,
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
            Synt::Doublecolon => ":",
            Synt::RBrace => "}",
            Synt::LBrace => "{",
            Synt::Comma => ",",
            Synt::Decl => "decl",
            Synt::Eq => "=",
        }
    }
}


pub fn parse<'a, D>(
    parser: impl P<'a, Node<'a, D>>,
    src: &'a str,
    src_path: &str,
) -> anyhow::Result<Node<'a, D>>
where
    D: Default + 'a,
{
    let (ast, errors) = Expr::parser().parse(src).into_output_errors();

    for err in &errors
    {
        crate::diagnostic::print_err(src_path, src, err)?;
    }

    ast.context(format!("Parsing failed with {} error(s)", errors.len()))
}
