use std::{marker::PhantomData, process::ExitCode};

use anyhow::{Context, Result, anyhow};
use chumsky::prelude::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Synt
{
    ChainNext,
}

impl Synt
{
    fn repr(&self) -> &'static str
    {
        match self
        {
            Synt::ChainNext => ";",
        }
    }
}

#[derive(Debug)]
enum Op<'src>
{
    Add(Box<Expr<'src>>, Box<Expr<'src>>),
}

#[derive(Debug)]
enum Val
{
    Num(f64),
    Nop,
}

impl Val
{
    fn parser<'src>() -> impl Parser<'src, &'src str, Self> + Clone
    {
        text::int(10)
            .map(|s: &str| Val::Num(s.parse().unwrap()))
            .padded()
    }
}


#[derive(Debug)]
enum InnerExpr<'src>
{
    Val(Val),
    Op(Op<'src>),
}

impl<'src> InnerExpr<'src>
{
    fn parser() -> impl Parser<'src, &'src str, Self> + Clone
    {
        Val::parser().map(Self::Val)
    }
}


#[derive(Debug)]
struct Expr<'src>
{
    inner: InnerExpr<'src>,
    then: Option<Box<Expr<'src>>>,
    _phantom: PhantomData<&'src ()>,
}


impl<'src> Expr<'src>
{
    fn parser() -> impl Parser<'src, &'src str, Self> + Clone
    {
        recursive(|expr_parser| {
            InnerExpr::parser()
                .then(choice((
                    just(Synt::ChainNext.repr())
                        .padded()
                        .ignore_then(expr_parser)
                        .or_not(),
                    just(Synt::ChainNext.repr()).padded().map(|_| {
                        Some(Self {
                            inner: InnerExpr::Val(Val::Nop),
                            then: None,
                            _phantom: PhantomData,
                        })
                    }),
                )))
                .map(|(inner, then)| Self {
                    inner,
                    then: then.map(Box::new),
                    _phantom: PhantomData,
                })
        })
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
