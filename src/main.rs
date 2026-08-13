#![allow(unused)]


use anyhow::{Context, Result, anyhow};
use chumsky::pratt::*;
use chumsky::prelude::*;
use std::process::ExitCode;


trait P<'a, T>: Parser<'a, &'a str, T, extra::Err<Rich<'a, char>>> + Clone {}
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


#[derive(Debug, Clone)]
enum Atom
{
    Str(String),
    Num(f64),
    Nop,
}

impl<'src> Atom
{
    fn parser() -> impl P<'src, Self>
    {
        //::<_, _, extra::Err<Simple<char>>>
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


#[derive(Debug, Clone)]
enum Binop
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
    fn add() -> impl P<'src, Self>
    {
        just(Synt::Plus.repr()).to(Self::Add)
    }

    fn sub() -> impl P<'src, Self>
    {
        just(Synt::Minus.repr()).to(Self::Sub)
    }

    fn mul() -> impl P<'src, Self>
    {
        just(Synt::Star.repr()).to(Self::Mul)
    }

    fn div() -> impl P<'src, Self>
    {
        just(Synt::Slash.repr()).to(Self::Div)
    }

    fn r#mod() -> impl P<'src, Self>
    {
        just(Synt::Percent.repr()).to(Self::Mod)
    }

    fn pow() -> impl P<'src, Self>
    {
        just(Synt::StarStar.repr()).to(Self::Pow)
    }
}


#[derive(Debug, Clone)]
enum Unaop
{
    Neg,
}


impl<'src> Unaop
{
    fn neg() -> impl P<'src, Self>
    {
        just(Synt::Minus.repr()).padded().to(Self::Neg)
    }
}


#[derive(Debug, Clone)]
enum Expr
{
    Atom(Atom),

    Binop
    {
        op: Binop,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    Unaop
    {
        op: Unaop,
        val: Box<Expr>,
    },

    Chain(Vec<Expr>),
}


impl<'src> Expr
{
    fn parser() -> impl P<'src, Self>
    {
        // Create the atom parser
        let atom = Atom::parser().padded().map(Self::Atom);

        let expr = recursive(move |expr| {
            // Check if this expressions is "geklammert"
            let parenthesized = expr.clone().delimited_by(
                just(Synt::LParen.repr()).padded(),
                just(Synt::RParen.repr()).padded(),
            );

            // Operands to operators can either be an atom or a (chained) expression
            let operand = atom.clone().or(parenthesized);

            // Parse the different operators and set their associativity and precedence
            let term_expr = operand.pratt((
                infix(left(0), Binop::add(), |lhs, _, rhs, _| Self::Binop {
                    op: Binop::Add,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }),
                infix(left(0), Binop::sub(), |lhs, _, rhs, _| Self::Binop {
                    op: Binop::Sub,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }),
                infix(left(1), Binop::mul(), |lhs, _, rhs, _| Self::Binop {
                    op: Binop::Mul,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }),
                infix(left(1), Binop::div(), |lhs, _, rhs, _| Self::Binop {
                    op: Binop::Div,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }),
                infix(left(1), Binop::r#mod(), |lhs, _, rhs, _| Self::Binop {
                    op: Binop::Mod,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }),
                infix(right(3), Binop::pow(), |lhs, _, rhs, _| Self::Binop {
                    op: Binop::Pow,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }),
                prefix(2, Unaop::neg(), |op, val, _| Self::Unaop {
                    op,
                    val: Box::new(val),
                }),
            ));

            // Chain term_exprs separated by ";"
            let expr_chain = term_expr
                .separated_by(just(Synt::Semicolon.repr()).padded())
                .collect::<Vec<_>>()
                .map(Self::Chain);

            // Check if the expression chain ends in ";" and add a nop node if it does
            expr_chain
                .then(just(Synt::Semicolon.repr()).padded().or_not())
                .map(|(mut chain, term)| match term
                {
                    Some(_) =>
                    {
                        if let Self::Chain(chain) = &mut chain
                        {
                            chain.push(Expr::Atom(Atom::Nop));
                        }

                        chain
                    }
                    None => chain,
                })
        });

        expr
    }
}


fn main() -> ExitCode
{
    match run()
    {
        Err(e) =>
        {
            eprintln!("{e}");
            ExitCode::FAILURE
        }

        _ => ExitCode::SUCCESS,
    }
}

fn run() -> Result<()>
{
    let source_file_path = std::env::args()
        .nth(1)
        .context("Missing source file path")?;

    let source_file_content = std::fs::read_to_string(&source_file_path)?;

    let ast = match Expr::parser().parse(&source_file_content).into_result()
    {
        Ok(ast) => ast,
        Err(errors) => return Err(anyhow!("{errors:#?}")),
    };

    dbg!(ast);

    Ok(())
}
