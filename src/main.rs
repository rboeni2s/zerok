use anyhow::{Context, Result};
use chumsky::error::Rich;
use std::{process::ExitCode, rc::Rc};
use zerok::{
    annotator::{self, Annotation, EnvEntry},
    diagnostic,
    ir::{ToIIC, generate_ir},
    parser,
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

    let mut program =
        parser::parse_prog::<Option<Annotation>>(&source_file_content, &source_file_path)?;

    //TODO: Alle funktionen bis auf main erstmal ignorieren...
    let main = program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .context("missing main funktion")?;

    // Annotate the ast
    if let Err((span, msg)) = main.body.annotate(&env)
    {
        diagnostic::print_err(
            &source_file_path,
            &source_file_content,
            &Rich::<char>::custom(span, msg),
        )?;

        return Err(anyhow::anyhow!("Type Error"));
    }


    let ir = generate_ir(&main.body, &env);

    dbg!(&program);
    dbg!(&ir);

    println!("{}", ir.to_iic());

    Ok(())
}
