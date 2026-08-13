#![allow(unused)]


use anyhow::{Context, Result, anyhow};
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::Parser;
use std::process::ExitCode;
use zerok::parser::Expr;


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

    let (ast, errors) = Expr::parser()
        .parse(&source_file_content)
        .into_output_errors();

    for err in &errors
    {
        print_err(&source_file_path, &source_file_content, err)?;
    }

    if !errors.is_empty()
    {
        return Err(anyhow!("Parsing failed with {} error(s)", errors.len()));
    }

    dbg!(ast.context("Parser produced no output despite no errors")?);

    Ok(())
}

fn print_err(
    source_file_path: &String,
    source_file_content: &String,
    err: &chumsky::prelude::Rich<'_, char>,
) -> Result<(), anyhow::Error>
{
    Report::build(
        ReportKind::Error,
        (source_file_path, err.span().into_range()),
    )
    .with_message(err.to_string())
    .with_label(
        Label::new((source_file_path, err.span().into_range()))
            .with_message(err.reason().to_string())
            .with_color(Color::Red),
    )
    .finish()
    .print((source_file_path, Source::from(source_file_content)))?;
    Ok(())
}


fn print_warn(
    source_file_path: &String,
    source_file_content: &String,
    err: &chumsky::prelude::Rich<'_, char>,
) -> Result<(), anyhow::Error>
{
    Report::build(
        ReportKind::Warning,
        (source_file_path, err.span().into_range()),
    )
    .with_message(err.to_string())
    .with_label(
        Label::new((source_file_path, err.span().into_range()))
            .with_message(err.reason().to_string())
            .with_color(Color::Yellow),
    )
    .finish()
    .print((source_file_path, Source::from(source_file_content)))?;
    Ok(())
}


fn print_info(
    source_file_path: &String,
    source_file_content: &String,
    err: &chumsky::prelude::Rich<'_, char>,
) -> Result<(), anyhow::Error>
{
    Report::build(
        ReportKind::Advice,
        (source_file_path, err.span().into_range()),
    )
    .with_message(err.to_string())
    .with_label(
        Label::new((source_file_path, err.span().into_range()))
            .with_message(err.reason().to_string())
            .with_color(Color::Green),
    )
    .finish()
    .print((source_file_path, Source::from(source_file_content)))?;
    Ok(())
}
