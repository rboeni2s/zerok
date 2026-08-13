use anyhow::{Context, Result, anyhow};
use chumsky::Parser;
use std::process::ExitCode;
use zerok::diagnostic;
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
        diagnostic::print_err(&source_file_path, &source_file_content, err)?;
    }

    if !errors.is_empty()
    {
        return Err(anyhow!("Parsing failed with {} error(s)", errors.len()));
    }

    dbg!(ast.context("Parser produced no output despite no errors")?);

    Ok(())
}
