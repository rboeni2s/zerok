use anyhow::{Context, Result, anyhow};
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

    let ast = match Expr::parser().parse(&source_file_content).into_result()
    {
        Ok(ast) => ast,
        Err(errors) => return Err(anyhow!("{errors:#?}")),
    };

    dbg!(ast);

    Ok(())
}
