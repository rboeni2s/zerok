pub mod ast;
pub mod ast_builder;
pub mod atom;
pub mod expr;
pub mod lexer;
pub mod ops;


use anyhow::Context;
use chumsky::input::MappedInput;
use chumsky::prelude::*;
use std::{marker::PhantomData, range::Range};


pub use expr::Expr;
pub use lexer::{Spanned, Token};


pub type TokenInput<'t, 's> = MappedInput<'t, Token<'s>, SimpleSpan, &'t [Spanned<Token<'s>>]>;


pub trait P<'t, 's: 't, T>:
    Parser<'t, TokenInput<'t, 's>, T, extra::Err<Rich<'t, Token<'s>>>> + Clone
{
}
impl<'t, 's: 't, T, I> P<'t, 's, T> for I where
    I: Parser<'t, TokenInput<'t, 's>, T, extra::Err<Rich<'t, Token<'s>>>> + Clone
{
}


pub type Node<'a, D> = ast::AstNode<'a, Expr<'a, D>, D>;


pub fn parse<'s, D>(src: &'s str, src_path: &str) -> anyhow::Result<Node<'s, D>>
where
    D: Default + 's,
{
    let (tokens, lex_errors) = lexer::lexer().parse(src).into_output_errors();

    for err in &lex_errors
    {
        crate::diagnostic::print_err(src_path, src, err)?;
    }

    let tokens = tokens.context(format!("Lexing failed with {} error(s)", lex_errors.len()))?;

    let input_end = SimpleSpan::from(src.len()..src.len());
    let (ast, parse_errors) = Expr::parser()
        .parse(tokens.as_slice().split_token_span(input_end))
        .into_output_errors();

    for err in &parse_errors
    {
        crate::diagnostic::print_err(src_path, src, err)?;
    }

    if !lex_errors.is_empty()
    {
        anyhow::bail!("Lexing failed with {} error(s)", lex_errors.len());
    }

    ast.context(format!(
        "Parsing failed with {} error(s)",
        parse_errors.len()
    ))
}
