mod env;


use crate::{
    diagnostic,
    parser::{
        Expr,
        Node,
        atom::Atom,
        ops::{Binop, Unaop},
    },
};
use chumsky::span::{SimpleSpan, Span};
use std::{rc::Rc, sync::atomic::AtomicUsize};


pub use crate::annotator::env::{Env, EnvEntry};


fn make_err<S>(span: S, err: impl ToString) -> Result<Annotation, (S, String)>
{
    Err((span, err.to_string()))
}


macro_rules! err {
    ($span:expr, $err:expr) => {
        make_err($span, $err)
    };

    ($span:expr, $err:expr, $($args:expr),+) => {
        make_err($span, format!($err, $($args),+))
    };

}

/// The Type of a node in the ast
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind
{
    Num,
    String,
    None,
}


impl Kind
{
    fn from_str(value: &str) -> Option<Self>
    {
        match value
        {
            "num" => Some(Kind::Num),
            "string" => Some(Kind::String),
            "none" => Some(Kind::None),
            _ => None,
        }
    }
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
    /// Inferred or given type
    pub kind: Kind,

    /// store cell index
    pub cell: Option<usize>,
}


impl Default for Annotation
{
    fn default() -> Self
    {
        Self {
            kind: Kind::None,
            cell: None,
        }
    }
}


impl<'a> Node<'a, Option<Annotation>>
{
    fn reg() -> usize
    {
        static REGISTER: AtomicUsize = AtomicUsize::new(0);
        REGISTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn annotate(
        &mut self,
        env: &Rc<Env<'a, EnvEntry>>,
    ) -> Result<Annotation, (SimpleSpan, String)>
    {
        let span = self.span;
        let annotation = match &mut self.inner
        {
            // Typecheck and annotate the atoms
            Expr::Atom(atom) =>
            {
                match atom
                {
                    Atom::Nop =>
                    {
                        Ok(Annotation {
                            kind: Kind::None,
                            cell: None,
                        })
                    }

                    atom =>
                    {
                        Ok(Annotation {
                            kind: Kind::from(&*atom),
                            cell: Some(env.reserve_and_put(EnvEntry::Atom(atom.clone()))),
                        })
                    }
                }
            }

            Expr::Binop { op, lhs, rhs } =>
            {
                let lhs = lhs.annotate(&env.child_env())?.kind;
                let rhs = rhs.annotate(&env.child_env())?.kind;

                match binop_compat(op, lhs, rhs, env)
                {
                    Some(ret) =>
                    {
                        Ok(Annotation {
                            kind: ret,
                            cell: Some(env.reserve_and_put(EnvEntry::Register(Self::reg()))),
                        })
                    }
                    None =>
                    {
                        err!(
                            span,
                            "Cannot apply {:?} to arguments of type {:?} and {:?}",
                            op,
                            lhs,
                            rhs
                        )
                    }
                }
            }

            Expr::Unaop { op, val } =>
            {
                let val = val.annotate(&env.child_env())?.kind;

                match unaop_compat(op, val, env)
                {
                    Some(ret) =>
                    {
                        Ok(Annotation {
                            kind: ret,
                            cell: Some(env.reserve_and_put(EnvEntry::Register(Self::reg()))),
                        })
                    }

                    None => err!(span, "Cannot apply {:?} to argument of type {:?}", op, val),
                }
            }

            Expr::Decl { name, kind, val } =>
            {
                let val = val.annotate(&env.child_env())?;

                let Some(kind) = Kind::from_str(kind)
                else
                {
                    return err!(span, "Unknown type {:?}", kind);
                };

                if kind != val.kind
                {
                    return err!(
                        span,
                        "Cannot assign a value of type {:?} to a binding of type {:?}",
                        val.kind,
                        kind
                    );
                }

                let annotation = Annotation {
                    kind,
                    cell: Some(env.reserve_and_put(EnvEntry::Register(Self::reg()))),
                };

                env.bind_cell(name, &annotation);

                Ok(annotation)
            }

            Expr::Binding { name } =>
            {
                env.fetch_bound(name)
                    .map(|(kind, cell)| {
                        Annotation {
                            kind,
                            cell: Some(cell),
                        }
                    })
                    .ok_or(err!(span, "Unknown binding {:?}", name).unwrap_err())
            }

            Expr::Chain(ast_nodes) =>
            {
                let mut last_annotation = Annotation::default();
                let mut chained_env = env.child_env();

                for ast in ast_nodes
                {
                    chained_env = chained_env.child_env();
                    last_annotation = ast.annotate(&chained_env)?;
                }

                Ok(last_annotation)
            }

            Expr::ParseError =>
            {
                err!(
                    span,
                    "Jemand hat sich noch nicht überlegt, ob der Typ-Checker bei ParserFehler trotzdem versuchen könnte das Programm nach dem Fehler zu checken..."
                )
            }
        };

        match annotation
        {
            Ok(annotation) =>
            {
                self.data = Some(annotation);
                Ok(annotation)
            }

            e => e,
        }
    }
}


//HACK: Replace this function with a proper lut
fn binop_compat<'a>(op: &Binop, lhs: Kind, rhs: Kind, _env: &Env<'a, EnvEntry>) -> Option<Kind>
{
    match (op, lhs, rhs)
    {
        (Binop::Add, Kind::Num, Kind::Num) => Some(Kind::Num),
        (Binop::Sub, Kind::Num, Kind::Num) => Some(Kind::Num),
        (Binop::Div, Kind::Num, Kind::Num) => Some(Kind::Num),
        (Binop::Mul, Kind::Num, Kind::Num) => Some(Kind::Num),
        (Binop::Mod, Kind::Num, Kind::Num) => Some(Kind::Num),
        (Binop::Pow, Kind::Num, Kind::Num) => Some(Kind::Num),
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
