/// Creates a type error at `span`, the message is formatted like with `format!`
macro_rules! err {
    ($span:expr, $($msg:tt)+) => {
        Err(($span, format!($($msg)+)))
    };
}


mod env;
mod kind;
mod program;


use crate::{
    parser::{Expr, Node, atom::Atom},
    reg_util::new_register,
};
use chumsky::span::SimpleSpan;
use kind::{binop_compat, cast_compat, unaop_compat};
use std::{fmt, rc::Rc, sync::atomic::AtomicUsize};


pub use env::{Env, EnvEntry};
pub use kind::Kind;

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

    /// Returns true if this node ends in a return, e.g. "a; geve b;", so its own value is never used
    fn ends_with_return(&self) -> bool
    {
        match &self.inner
        {
            Expr::Return { .. } => true,

            // A chain ending in ";" has a nop as its last element
            Expr::Chain(ast_nodes) =>
            {
                match ast_nodes.as_slice()
                {
                    [.., last, nop] if matches!(nop.inner, Expr::Atom(Atom::Nop)) =>
                    {
                        last.ends_with_return()
                    }
                    [.., last] => last.ends_with_return(),
                    [] => false,
                }
            }

            _ => false,
        }
    }

    /// Annotates the nodes of a chain in `env`, without opening a new scope.
    /// Every node can see the bindings of the nodes before it, a later binding with the same name shadows the earlier one
    fn annotate_chain(
        ast_nodes: &mut [Self],
        env: &Rc<Env<'a, EnvEntry>>,
        expected: Option<Kind>,
    ) -> Result<Annotation, (SimpleSpan, String)>
    {
        let mut last_annotation = Annotation::default();

        // Only the last expression of a chain determines its type
        let last = ast_nodes.len().saturating_sub(1);

        for (i, ast) in ast_nodes.iter_mut().enumerate()
        {
            last_annotation =
                ast.annotate_expecting(env, if i == last { expected } else { None })?;
        }

        Ok(last_annotation)
    }

    /// Annotates this node, but if it is a chain it does not open its own scope, so its bindings stay visible in `env`.
    /// This is used for the conditions of if statements, e.g. "wenn sett x: u32 = 5; c { x }"
    fn annotate_in_scope(
        &mut self,
        env: &Rc<Env<'a, EnvEntry>>,
        expected: Option<Kind>,
    ) -> Result<Annotation, (SimpleSpan, String)>
    {
        let Expr::Chain(ast_nodes) = &mut self.inner
        else
        {
            return self.annotate_expecting(env, expected);
        };

        let annotation = Self::annotate_chain(ast_nodes, env, expected)?;
        self.data = Some(annotation);
        Ok(annotation)
    }

    /// Annotates this node and makes sure it has the type `kind`, `what` describes the node in the error message
    fn annotate_as(
        &mut self,
        env: &Rc<Env<'a, EnvEntry>>,
        kind: Kind,
        what: impl fmt::Display,
    ) -> Result<Annotation, (SimpleSpan, String)>
    {
        let annotation = self.annotate_expecting(env, Some(kind))?;

        if annotation.kind != kind
        {
            return err!(
                self.span,
                "{} has to be of type {}, but is of type {}",
                what,
                kind,
                annotation.kind
            );
        }

        Ok(annotation)
    }

    /// Like `annotate_as`, but a chain does not open its own scope, see `annotate_in_scope`
    fn annotate_in_scope_as(
        &mut self,
        env: &Rc<Env<'a, EnvEntry>>,
        kind: Kind,
        what: impl fmt::Display,
    ) -> Result<Annotation, (SimpleSpan, String)>
    {
        let annotation = self.annotate_in_scope(env, Some(kind))?;

        if annotation.kind != kind
        {
            return err!(
                self.span,
                "{} has to be of type {}, but is of type {}",
                what,
                kind,
                annotation.kind
            );
        }

        Ok(annotation)
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
        let annotation: Annotation = match &mut self.inner
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
                            cell: Some(env.put(EnvEntry::Atom(atom.clone()))),
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
                            cell: Some(new_register(env)),
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
                            cell: Some(new_register(env)),
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

                val.annotate_as(&env.child_env(), kind, format!("The value of {name:?}"))?;

                let annotation = Annotation {
                    kind,
                    cell: Some(new_register(env)),
                };

                env.bind_cell(name, &annotation);

                Ok(annotation)
            }

            Expr::Binding { name } =>
            {
                let Some((kind, cell)) = env.fetch_bound_cell(name)
                else
                {
                    return err!(span, "Unknown binding {:?}", name);
                };

                Ok(Annotation {
                    kind,
                    cell: Some(cell),
                })
            }

            Expr::Cast { val, kind } =>
            {
                let Some(kind) = Kind::from_str(kind)
                else
                {
                    return err!(span, "Unknown type {:?}", kind);
                };

                if kind == Kind::None
                {
                    return err!(span, "Cannot cast to none");
                }

                let val = val.annotate_expecting(&env.child_env(), Some(kind))?.kind;

                if !cast_compat(val, kind)
                {
                    return err!(span, "Cannot cast a value of type {} to {}", val, kind);
                }

                Ok(Annotation {
                    kind,
                    cell: Some(new_register(env)),
                })
            }

            // A chain has its own scope, so its bindings are not visible after it
            Expr::Chain(ast_nodes) => Self::annotate_chain(ast_nodes, &env.child_env(), expected),

            Expr::Call { name, args } =>
            {
                let Some((ret, params)) = env.get_function(name)
                else
                {
                    return err!(span, "Unknown function {:?}", name);
                };

                if args.len() != params.len()
                {
                    return err!(
                        span,
                        "Function {:?} takes {} argument(s), but {} were given",
                        name,
                        params.len(),
                        args.len()
                    );
                }

                for (i, (arg, param)) in args.iter_mut().zip(params).enumerate()
                {
                    arg.annotate_as(
                        &env.child_env(),
                        param,
                        format!("Argument {} of {:?}", i + 1, name),
                    )?;
                }

                // Every call gets a register, even if the function does not return anything (it then returns 0)
                Ok(Annotation {
                    kind: ret,
                    cell: Some(new_register(env)),
                })
            }

            Expr::Return { val } =>
            {
                let Some(ret) = env.ret()
                else
                {
                    return err!(span, "Cannot return outside of a function");
                };

                let val = match val
                {
                    Some(val) => val.annotate_as(&env.child_env(), ret, "The returned value")?,

                    // A return without a value returns nothing
                    None if ret == Kind::None => Annotation::default(),
                    None =>
                    {
                        return err!(span, "A function returning {} has to return a value", ret);
                    }
                };

                // A return takes the type and cell of its value, so a body ending in a return has the right type
                Ok(val)
            }

            Expr::If {
                condition,
                body,
                elifs,
                else_body,
            } =>
            {
                // Without an else there is no value if no condition is true, so the if does not have a value then
                let has_else = else_body.is_some();
                let expected = if has_else { expected } else { None };

                // The bindings of a condition are visible in its body and in all later branches, but not after the if
                let mut scope = env.child_env();

                condition.annotate_in_scope_as(&scope, Kind::Bool, "The condition")?;

                let kind = body.annotate_expecting(&scope.child_env(), expected)?.kind;

                // Each elif sees the bindings of all conditions before it
                for (elif_cond, elif_body) in elifs
                {
                    scope = scope.child_env();

                    elif_cond.annotate_in_scope_as(&scope, Kind::Bool, "The condition")?;

                    // Without an else the values of the branches are not used, so their types do not matter
                    if has_else
                    {
                        elif_body.annotate_as(&scope.child_env(), kind, "This branch of the if")?;
                    }
                    else
                    {
                        elif_body.annotate_expecting(&scope.child_env(), Some(kind))?;
                    }
                }

                match else_body
                {
                    None => Ok(Annotation::default()),

                    Some(else_body) =>
                    {
                        else_body.annotate_as(&scope.child_env(), kind, "This branch of the if")?;

                        // Every branch writes its value into the register of the if
                        Ok(Annotation {
                            kind,
                            cell: (kind != Kind::None).then(|| new_register(env)),
                        })
                    }
                }
            }

            Expr::ParseError =>
            {
                err!(
                    span,
                    "Jemand hat sich noch nicht überlegt, ob der Typ-Checker bei ParserFehler trotzdem versuchen könnte das Programm nach dem Fehler zu checken..."
                )
            }
        }?;

        self.data = Some(annotation);
        Ok(annotation)
    }
}
