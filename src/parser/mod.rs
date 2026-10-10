pub mod ast;
pub mod atom;
pub mod expr;
pub mod lexer;
pub mod ops;
pub mod program;


use anyhow::Context;
pub use ast::Node;
use chumsky::input::MappedInput;
use chumsky::prelude::*;
pub use expr::Expr;
pub use lexer::{Spanned, Token};
pub use program::{Function, Param, Program};


/// TokenInput
pub type TokenInput<'t, 's> = MappedInput<'t, Token<'s>, SimpleSpan, &'t [Spanned<Token<'s>>]>;

/// Der trait P und seine blanket implementation sind nur ein alias für seinen Viel zu langen supertrait....
pub trait P<'t, 's: 't, T>:
    Parser<'t, TokenInput<'t, 's>, T, extra::Err<Rich<'t, Token<'s>>>> + Clone
{
}
impl<'t, 's: 't, T, I> P<'t, 's, T> for I where
    I: Parser<'t, TokenInput<'t, 's>, T, extra::Err<Rich<'t, Token<'s>>>> + Clone
{
}


/// Parses a whole program
pub fn parse_prog<'s, D>(src: &'s str, src_path: &str) -> anyhow::Result<Program<'s, D>>
where
    D: Default + 's,
{
    let (tokens, lex_errors) = lex(src, src_path)?;
    let input_end = SimpleSpan::from(src.len()..src.len());

    let (program, parse_errors) = Program::parser()
        .parse({ tokens.split_token_span(input_end) })
        .into_output_errors();

    finish(src, src_path, program, &parse_errors, lex_errors)
}


/// Parses a single expression
pub fn parse_expr<'s, D>(src: &'s str, src_path: &str) -> anyhow::Result<Node<'s, D>>
where
    D: Default + 's,
{
    let (tokens, lex_errors) = lex(src, src_path)?;
    let input_end = SimpleSpan::from(src.len()..src.len());

    let (ast, parse_errors) = Expr::parser()
        .parse({ tokens.split_token_span(input_end) })
        .into_output_errors();

    finish(src, src_path, ast, &parse_errors, lex_errors)
}


fn lex<'s>(src: &'s str, src_path: &str) -> anyhow::Result<(Vec<Spanned<Token<'s>>>, usize)>
{
    let (tokens, lex_errors) = lexer::lexer().parse(src).into_output_errors();

    for err in &lex_errors
    {
        crate::diagnostic::print_err(src_path, src, err)?;
    }

    let tokens = tokens.context(format!("Lexing failed with {} error(s)", lex_errors.len()))?;

    Ok((tokens, lex_errors.len()))
}


/// Prints lexer and parser errors and errors out if there where any
fn finish<T>(
    src: &str,
    src_path: &str,
    output: Option<T>,
    parse_errors: &[Rich<'_, Token<'_>>],
    lex_errors: usize,
) -> anyhow::Result<T>
{
    for err in parse_errors
    {
        crate::diagnostic::print_err(src_path, src, err)?;
    }

    if lex_errors > 0
    {
        anyhow::bail!("Lexing failed with {lex_errors} error(s)");
    }

    output.context(format!(
        "Parsing failed with {} error(s)",
        parse_errors.len()
    ))
}
