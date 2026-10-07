use anyhow::{Context, Result};
use std::process::ExitCode;
use zerok::{
    annotator::{self, Annotation, EnvEntry},
    parser::{self, Expr},
};


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
    let env = annotator::Env::<EnvEntry>::default();

    let mut ast = parser::parse(
        Expr::<Option<Annotation>>::parser(),
        &source_file_content,
        &source_file_path,
    )?;

    // Annotate the ast
    ast.annotate(&env);

    dbg!(ast);

    Ok(())
}
