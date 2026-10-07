mod env;


use crate::parser::{
    Expr,
    Node,
    atom::Atom,
    ops::{Binop, Unaop},
};
use std::sync::atomic::AtomicUsize;


pub use crate::annotator::env::{Env, EnvEntry};


/// The Type of a node in the ast
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind
{
    Num,
    String,
    None,
}


impl From<&Atom> for Kind
{
    fn from(value: &Atom) -> Self
    {
        match value
        {
            Atom::Str(_) => Kind::String,
            Atom::Num(_) => Kind::Num,
            Atom::Nop => Kind::None,
        }
    }
}


/// The Annotations for one ast node
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Annotation
{
    /// Infered or given type
    pub kind: Kind,

    /// store cell index
    pub cell: Option<usize>,
}


impl<'a> Node<'a, Option<Annotation>>
{
    fn reg() -> usize
    {
        static REGISTER: AtomicUsize = AtomicUsize::new(0);
        REGISTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn annotate(&mut self, env: &Env<'a, EnvEntry>)
    {
        let annotation = match &mut self.inner
        {
            // Typecheck and annotate the atoms
            Expr::Atom(atom) => match atom
            {
                Atom::Nop => Ok(Annotation {
                    kind: Kind::None,
                    cell: None,
                }),

                atom => Ok(Annotation {
                    kind: Kind::from(&*atom),
                    cell: Some(env.reserve_and_put(EnvEntry::Atom(atom.clone()))),
                }),
            },

            Expr::Binop { op, lhs, rhs } =>
            {
                lhs.annotate(env);
                rhs.annotate(env);

                let lhs = lhs.data.as_ref().unwrap().kind;
                let rhs = rhs.data.as_ref().unwrap().kind;

                match binop_compat(op, lhs, rhs, env)
                {
                    Some(ret) => Ok(Annotation {
                        kind: ret,
                        cell: Some(env.reserve_and_put(EnvEntry::Register(Self::reg()))),
                    }),
                    None => Err(format!(
                        "Cannot apply {op:?} to arguments of type {lhs:?} and {rhs:?}"
                    )),
                }
            }

            Expr::Unaop { op, val } =>
            {
                val.annotate(env);

                let val = val.data.as_ref().unwrap().kind;

                match unaop_compat(op, val, env)
                {
                    Some(ret) => Ok(Annotation {
                        kind: ret,
                        cell: Some(env.reserve_and_put(EnvEntry::Register(Self::reg()))),
                    }),

                    None => Err(format!("Cannot apply {op:?} to argument of type {val:?}")),
                }
            }

            Expr::Decl { name, kind, val } => todo!(),

            Expr::Chain(ast_nodes) =>
            {
                let mut last_annotation = Err(format!("Empty expression chain"));

                for ast in ast_nodes
                {
                    ast.annotate(env);
                    //TODO: Hier weiter
                }
            }

            Expr::ParseError =>
            {
                todo!("Überlegen, was der Typechecker mit einem Parse Error macht...")
            }
        };
    }
}


//HACK: Replace this function with a proper lut
fn binop_compat<'a>(op: &Binop, lhs: Kind, rhs: Kind, _env: &Env<'a, EnvEntry>) -> Option<Kind>
{
    match (op, lhs, rhs)
    {
        (Binop::Add, Kind::Num, Kind::Num) => Some(Kind::Num),
        _ => None,
    }
}


//HACK: Replace this function with a proper lut
fn unaop_compat<'a>(op: &Unaop, arg: Kind, _env: &Env<'a, EnvEntry>) -> Option<Kind>
{
    match (op, arg)
    {
        (Unaop::Neg, Kind::Num) => Some(Kind::Num),
        _ => None,
    }
}
