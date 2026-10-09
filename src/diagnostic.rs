use anyhow::Result;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::prelude::Rich;
use std::fmt::Display;


type Err<'a, T> = Rich<'a, T>;


pub fn print_err<T: Display>(
    src_path: &str,
    src: &str,
    err: &Err<'_, T>,
) -> Result<(), anyhow::Error>
{
    Report::build(ReportKind::Error, (src_path, err.span().into_range()))
        .with_message(err.to_string())
        .with_label(
            Label::new((src_path, err.span().into_range()))
                .with_message(err.reason().to_string())
                .with_color(Color::Red),
        )
        .finish()
        .print((src_path, Source::from(src)))?;
    Ok(())
}


pub fn print_warn<T: Display>(
    src_path: &str,
    src: &str,
    err: &Err<'_, T>,
) -> Result<(), anyhow::Error>
{
    Report::build(ReportKind::Warning, (src_path, err.span().into_range()))
        .with_message(err.to_string())
        .with_label(
            Label::new((src_path, err.span().into_range()))
                .with_message(err.reason().to_string())
                .with_color(Color::Yellow),
        )
        .finish()
        .print((src_path, Source::from(src)))?;
    Ok(())
}


pub fn print_info<T: Display>(
    src_path: &str,
    src: &str,
    err: &Err<'_, T>,
) -> Result<(), anyhow::Error>
{
    Report::build(ReportKind::Advice, (src_path, err.span().into_range()))
        .with_message(err.to_string())
        .with_label(
            Label::new((src_path, err.span().into_range()))
                .with_message(err.reason().to_string())
                .with_color(Color::Green),
        )
        .finish()
        .print((src_path, Source::from(src)))?;
    Ok(())
}
