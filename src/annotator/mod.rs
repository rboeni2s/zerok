mod env;
mod kind;


use crate::parser::{Expr, Node, atom::Atom};
use chumsky::span::SimpleSpan;
use kind::{binop_compat, cast_compat, unaop_compat};
use std::{rc::Rc, sync::atomic::AtomicUsize};


pub use env::{Env, EnvEntry};
pub use kind::Kind;


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

    /// Returns true if the type of this node is only determined by number literals, so it adapts to the expected type
    fn is_untyped_literal(&self) -> bool
    {
        match &self.inner
        {
            Expr::Atom(atom) => matches!(atom, Atom::Int(_) | Atom::Float(_)),
            Expr::Unaop { val, .. } => val.is_untyped_literal(),
            Expr::Binop { lhs, rhs, .. } => lhs.is_untyped_literal() && rhs.is_untyped_literal(),
            Expr::Chain(ast_nodes) => ast_nodes.last().is_some_and(|ast| ast.is_untyped_literal()),
            _ => false,
        }
    }

    pub fn annotate(
        &mut self,
        env: &Rc<Env<'a, EnvEntry>>,
    ) -> Result<Annotation, (SimpleSpan, String)>
    {
        self.annotate_expecting(env, None)
    }

    /// Annotates this node, `expected` is the type the surrounding expression expects (if known) and is used to
    /// determine the type of number literals
    fn annotate_expecting(
        &mut self,
        env: &Rc<Env<'a, EnvEntry>>,
        expected: Option<Kind>,
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
                        let kind = Kind::of_atom(atom, expected);

                        if !kind.fits(atom)
                        {
                            return err!(span, "Literal {} does not fit into {}", atom, kind);
                        }

                        Ok(Annotation {
                            kind,
                            cell: Some(env.reserve_and_put(EnvEntry::Atom {
                                atom: atom.clone(),
                                reg: Self::reg(),
                            })),
                        })
                    }
                }
            }

            Expr::Binop { op, lhs, rhs } =>
            {
                // All operators return the type of their operands, so the expected type is passed on to the operands.
                // Untyped literals take the type of the other operand, so the operand with a known type is annotated first
                let (lhs, rhs) = if lhs.is_untyped_literal() && !rhs.is_untyped_literal()
                {
                    let rhs = rhs.annotate_expecting(&env.child_env(), expected)?.kind;
                    let lhs = lhs.annotate_expecting(&env.child_env(), Some(rhs))?.kind;
                    (lhs, rhs)
                }
                else
                {
                    let lhs = lhs.annotate_expecting(&env.child_env(), expected)?.kind;
                    let rhs = rhs.annotate_expecting(&env.child_env(), Some(lhs))?.kind;
                    (lhs, rhs)
                };

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
                            "Cannot apply {:?} to arguments of type {} and {}",
                            op,
                            lhs,
                            rhs
                        )
                    }
                }
            }

            Expr::Unaop { op, val } =>
            {
                let val = val.annotate_expecting(&env.child_env(), expected)?.kind;

                match unaop_compat(op, val, env)
                {
                    Some(ret) =>
                    {
                        Ok(Annotation {
                            kind: ret,
                            cell: Some(env.reserve_and_put(EnvEntry::Register(Self::reg()))),
                        })
                    }

                    None => err!(span, "Cannot apply {:?} to argument of type {}", op, val),
                }
            }

            Expr::Decl { name, kind, val } =>
            {
                let Some(kind) = Kind::from_str(kind)
                else
                {
                    return err!(span, "Unknown type {:?}", kind);
                };

                let val = val.annotate_expecting(&env.child_env(), Some(kind))?;

                if kind != val.kind
                {
                    return err!(
                        span,
                        "Cannot assign a value of type {} to a binding of type {}",
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
                env.fetch_bound_cell(name)
                    .map(|(kind, cell)| {
                        Annotation {
                            kind,
                            cell: Some(cell),
                        }
                    })
                    .ok_or(err!(span, "Unknown binding {:?}", name).unwrap_err())
            }

            Expr::Cast { val, kind } =>
            {
                let Some(kind) = Kind::from_str(kind)
                else
                {
                    return err!(span, "Unknown type {:?}", kind);
                };

                let val = val.annotate_expecting(&env.child_env(), Some(kind))?.kind;

                if !cast_compat(val, kind)
                {
                    return err!(span, "Cannot cast a value of type {} to {}", val, kind);
                }

                Ok(Annotation {
                    kind,
                    cell: Some(env.reserve_and_put(EnvEntry::Register(Self::reg()))),
                })
            }

            Expr::Chain(ast_nodes) =>
            {
                let mut last_annotation = Annotation::default();
                let mut chained_env = env.child_env();

                // Only the last expression of a chain determines its type
                let last = ast_nodes.len().saturating_sub(1);

                for (i, ast) in ast_nodes.iter_mut().enumerate()
                {
                    chained_env = chained_env.child_env();
                    last_annotation = ast.annotate_expecting(
                        &chained_env,
                        if i == last { expected } else { None },
                    )?;
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
