#![allow(unused)]
#![feature(iter_intersperse)]


pub mod annotator;
pub mod diagnostic;
pub mod ir;
pub mod parser;


use crate::{
    annotator::{Annotation, EnvEntry},
    parser::ast::AstNode,
};

pub type ExprAst<'a> = AstNode<'a, parser::Expr<'a, Option<Annotation>>, Option<Annotation>>;
pub type ProgAst<'a> = parser::Program<'a, Option<Annotation>>;
pub type Env<'a> = annotator::Env<'a, EnvEntry>;
