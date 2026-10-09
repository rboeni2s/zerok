use anyhow::{Context, Result};
use chumsky::error::Rich;
use std::{process::ExitCode, rc::Rc};
use zerok::{
    annotator::{self, Annotation, EnvEntry},
    diagnostic,
    parser::{self, Expr},
};


fn main() -> ExitCode
{
    match run()
    {
        Err(e) =>
        {
            eprintln!("Aborted, due to {e}");
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
    let env = Rc::new(annotator::Env::<EnvEntry>::default());

    let mut ast = parser::parse(
        Expr::<Option<Annotation>>::parser(),
        &source_file_content,
        &source_file_path,
    )?;

    // Annotate the ast
    if let Err((span, msg)) = ast.annotate(&env)
    {
        diagnostic::print_err(
            &source_file_path,
            &source_file_content,
            &Rich::custom(span, msg),
        )?;

        return Err(anyhow::anyhow!("Type Error"));
    }

    dbg!(ast);

    Ok(())
}
