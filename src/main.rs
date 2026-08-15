use anyhow::{Context, Result};
use std::process::ExitCode;
use zerok::parser::{self, Expr};


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

    let ast = parser::parse(Expr::parser(), &source_file_content, &source_file_path)?;

    dbg!(ast);

    Ok(())
}
