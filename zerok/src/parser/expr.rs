use crate::parser::ast::AstNode;

use super::atom::Atom;
use super::ops::{Binop, Unaop};
use super::{Node, P, Token, Type};
use chumsky::pratt::*;
use chumsky::prelude::*;


#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src, D>
{
    Atom(Atom),

    Binop
    {
        op: Binop,
        lhs: Box<Node<'src, D>>,
        rhs: Box<Node<'src, D>>,
    },

    Unaop
    {
        op: Unaop,
        val: Box<Node<'src, D>>,
    },

    Decl
    {
        name: &'src str,
        kind: Type<'src>,
        val: Box<Node<'src, D>>,
    },

    Binding
    {
        name: &'src str,
    },

    Call
    {
        name: &'src str,
        args: Vec<Node<'src, D>>,
    },

    Cast
    {
        val: Box<Node<'src, D>>,
        kind: Type<'src>,
    },

    Return
    {
        val: Option<Box<Node<'src, D>>>,
    },

    If
    {
        condition: Box<Node<'src, D>>,
        body: Box<Node<'src, D>>,
        elifs: Vec<(Node<'src, D>, Node<'src, D>)>, // List of Elif-tuples (condition, body)
        else_body: Option<Box<Node<'src, D>>>,      // Body of the else
    },

    Loop
    {
        name: Option<String>,
        head: Option<Box<Node<'src, D>>>, // The loop evaluates to the value of its head, e.g. "sling sett x: u32 = 0 { ... }"
        body: Box<Node<'src, D>>,
    },

    Break
    {
        name: Option<&'src str>,
    },

    Continue
    {
        name: Option<&'src str>,
    },

    Assign
    {
        target: Box<Node<'src, D>>,
        source: Box<Node<'src, D>>,
    },

    /// Writes `source` into the value the reference `target` points to, e.g. "a &= 5"
    RefAssign
    {
        target: Box<Node<'src, D>>,
        source: Box<Node<'src, D>>,
    },

    Chain(Vec<Node<'src, D>>),

    // Placeholder for a something that could not be parsed.
    ParseError,
}


impl<'src, D> Expr<'src, D>
where
    D: Default + 'src,
{
    pub fn parser<'t>() -> impl P<'t, 'src, Node<'src, D>>
    where
        'src: 't,
    {
        // Create the atom parser
        let atom = Atom::parser()
            .map(Self::Atom)
            .map_with(|atom, span| Node::new(atom, span.span()));

        let ident = select! { Token::Ident(name) => name }.labelled("identifier");

        // Split the recursive parser declaration of a expression chain into the declaration and definition part, to allow
        // building the term expression in between, this allows for complex expressions to be chained with the correct chaining precedence
        let mut chain = Recursive::declare();

        let term = recursive({
            let chain = chain.clone();
            move |term| {
                // Check if this expression is a declaration
                let decl = just(Token::Decl)
                    .ignore_then(ident)
                    .then_ignore(just(Token::Doublecolon))
                    .then(Type::parser())
                    .then_ignore(just(Token::Eq))
                    .then(term.clone())
                    .map(|((name, kind), val)| {
                        Self::Decl {
                            name,
                            kind,
                            val: Box::new(val),
                        }
                    })
                    .map_with(|expr, span| AstNode::new(expr, span.span()));

                // Check if this expression is a return, the returned value is optional
                let ret = just(Token::Return)
                    .ignore_then(term.clone().or_not())
                    .map(|val| {
                        Self::Return {
                            val: val.map(Box::new),
                        }
                    })
                    .map_with(|expr, span| AstNode::new(expr, span.span()));

                // Check if this expression is a function call, the arguments are separated by ","
                let call = ident
                    .then(
                        term.clone()
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .collect::<Vec<_>>()
                            .delimited_by(just(Token::LParen), just(Token::RParen)),
                    )
                    .map(|(name, args)| Self::Call { name, args })
                    .map_with(|expr, span| AstNode::new(expr, span.span()));

                // Check if this expression is a ident to a binding
                let binding = ident
                    .map(|name| Self::Binding { name })
                    .map_with(|expr, span| AstNode::new(expr, span.span()));

                // Check if this expressions is "geklammert"
                let parenthesized = chain
                    .clone()
                    .delimited_by(just(Token::LParen), just(Token::RParen));

                // Operands to operators can either be an atom or a (chained) expression
                let operand = choice((
                    atom.clone(),
                    parenthesized,
                    decl,
                    ret,
                    call,
                    binding,
                    Self::if_parser(chain.clone()),
                    Self::loop_parser(chain.clone()),
                    Self::break_parser(),
                    Self::continue_parser(),
                ));

                // Parse the different operators and set their associativity and precedence
                let operation = operand.pratt((
                    // "a || b && c" is "a || (b && c)"
                    infix(left(0), Binop::bool_or(), |lhs, op, rhs, info| {
                        Self::binop(op, lhs, rhs, info.span())
                    }),
                    // "a == b && c < d" is "(a == b) && (c < d)"
                    infix(left(1), Binop::bool_and(), |lhs, op, rhs, info| {
                        Self::binop(op, lhs, rhs, info.span())
                    }),
                    // "a + 1 < b * 2" is "(a + 1) < (b * 2)"
                    infix(left(2), Binop::compare(), |lhs, op, rhs, info| {
                        Self::binop(op, lhs, rhs, info.span())
                    }),
                    infix(
                        left(3),
                        Binop::add().or(Binop::sub()),
                        |lhs, op, rhs, info| Self::binop(op, lhs, rhs, info.span()),
                    ),
                    infix(
                        left(4),
                        choice((Binop::mul(), Binop::div(), Binop::modulo())),
                        |lhs, op, rhs, info| Self::binop(op, lhs, rhs, info.span()),
                    ),
                    infix(right(7), Binop::pow(), |lhs, op, rhs, info| {
                        Self::binop(op, lhs, rhs, info.span())
                    }),
                    // "-a as u32" is "(-a) as u32" and "a * b as u32" is "a * (b as u32)"
                    postfix(
                        5,
                        just(Token::As).ignore_then(Type::parser()),
                        |val, kind, info| {
                            AstNode::new(
                                Self::Cast {
                                    val: Box::new(val),
                                    kind,
                                },
                                info.span(),
                            )
                        },
                    ),
                    // "!a == b" is "(!a) == b", "-a ** 2" is "-(a ** 2)" and "&a + b" is "(&a) + b"
                    prefix(
                        6,
                        choice((Unaop::neg(), Unaop::bool_not(), Unaop::reference())),
                        |op, val, info| Self::unaop(op, val, info.span()),
                    ),
                    // "&&" is lexed as one token, so "&&a" is a reference to a reference
                    prefix(6, just(Token::BoolAnd), |_, val, info| {
                        let inner = Self::unaop(Unaop::Ref, val, info.span());
                        Self::unaop(Unaop::Ref, inner, info.span())
                    }),
                ));

                // An assignment binds weaker than every operator, so its target can be any expression, e.g. "(f(); x) = 5".
                // The typechecker decides whether the target is actually assignable
                operation
                    // "a = b" assigns to a, "a &= b" writes into the value a points to
                    .then(
                        choice((just(Token::Eq).to(false), just(Token::RefEq).to(true)))
                            .then(term.clone())
                            .or_not(),
                    )
                    .map_with(|(target, assignment), info| {
                        let Some((through_ref, source)) = assignment
                        else
                        {
                            return target;
                        };

                        let (target, source) = (Box::new(target), Box::new(source));

                        let expr = match through_ref
                        {
                            true => Self::RefAssign { target, source },
                            false => Self::Assign { target, source },
                        };

                        AstNode::new(expr, info.span())
                    })
            }
        });

        // Chain terms separated by ";"
        let expr_chain = term
            .separated_by(just(Token::Semicolon))
            .collect::<Vec<_>>()
            .map(Self::Chain);

        // Describes something that could be the end of an expression.
        // This is used for error recovery on malformed input
        let terminator = || {
            choice((
                just(Token::Semicolon).ignored(),
                just(Token::RParen).ignored(),
                just(Token::RBrace).ignored(),
                end(),
            ))
        };

        // Check if the expression chain ends in ";" and add a nop node if it does
        chain.define(
            expr_chain
                .then(just(Token::Semicolon).or_not())
                .map_with(|(mut chain, term), info| {
                    match term
                    {
                        Some(_) =>
                        {
                            if let Self::Chain(chain) = &mut chain
                            {
                                chain.push(Expr::Atom(Atom::Nop).into());
                            }

                            AstNode::new(chain, info.span())
                        }
                        None => AstNode::new(chain, info.span()),
                    }
                })
                // Try to skip malformed input in case of an error, so that the rest of the input can be checked
                //TODO: This doesnt work, i should debug and fix this
                .recover_with(skip_until(any().ignored(), terminator().rewind(), || {
                    Self::ParseError.into()
                })),
        );

        chain
    }

    /// Parsers If statements with optional elif branches and an else block
    fn if_parser<'t>(chain: impl P<'t, 'src, Node<'src, D>>) -> impl P<'t, 'src, Node<'src, D>>
    where
        'src: 't,
    {
        let p_if = just(Token::If).ignore_then(chain.clone()).then(
            chain
                .clone()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        );

        let p_elif = just(Token::If)
            .ignore_then(just(Token::Else))
            .ignore_then(chain.clone())
            .then(
                chain
                    .clone()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            );

        let p_else = just(Token::Else).ignore_then(
            chain
                .clone()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        );

        //p_if, p_elif und p_else nun zu einem parser zusammenfassen, in den ein if vorkommen muss, dann eine beliebige menge an elif und dann optional ein else
        p_if.then(p_elif.repeated().collect::<Vec<_>>())
            .then(p_else.or_not())
            .map_with(|(((cond, body), elifs), else_body), info| {
                AstNode::new(
                    Self::If {
                        condition: Box::new(cond),
                        body: Box::new(body),
                        elifs,
                        else_body: else_body.map(Box::new),
                    },
                    info.span(),
                )
            })
    }

    fn loop_parser<'t>(chain: impl P<'t, 'src, Node<'src, D>>) -> impl P<'t, 'src, Node<'src, D>>
    where
        'src: 't,
    {
        let ident = select! { Token::Ident(name) => name }.labelled("identifier");

        let name = just(Token::Tick)
            .ignore_then(ident)
            .then_ignore(just(Token::Doublecolon));

        name.or_not()
            .then_ignore(just(Token::Loop))
            .then(chain.clone())
            .then(
                chain
                    .clone()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map_with(|((name, head), body), info| {
                // A missing head is parsed as an empty chain
                let head = match &head.inner
                {
                    Self::Chain(nodes) if nodes.is_empty() => None,
                    _ => Some(Box::new(head)),
                };

                AstNode::new(
                    Self::Loop {
                        name: name.map(|n| n.to_string()),
                        head,
                        body: Box::new(body),
                    },
                    info.span(),
                )
            })
    }

    fn break_parser<'t>() -> impl P<'t, 'src, Node<'src, D>>
    where
        'src: 't,
    {
        let ident = select! { Token::Ident(name) => name }.labelled("identifier");

        let name = just(Token::Tick).ignore_then(ident);

        just(Token::Break)
            .ignore_then(name.or_not())
            .map_with(|name, info| AstNode::new(Expr::Break { name }, info.span()))
    }

    fn continue_parser<'t>() -> impl P<'t, 'src, Node<'src, D>>
    where
        'src: 't,
    {
        let ident = select! { Token::Ident(name) => name }.labelled("identifier");

        let name = just(Token::Tick).ignore_then(ident);

        just(Token::Continue)
            .ignore_then(name.or_not())
            .map_with(|name, info| AstNode::new(Expr::Continue { name }, info.span()))
    }


    fn unaop(op: Unaop, val: Node<'src, D>, span: SimpleSpan) -> Node<'src, D>
    {
        AstNode::new(
            Self::Unaop {
                op,
                val: Box::new(val),
            },
            span,
        )
    }


    fn binop(op: Binop, lhs: Node<'src, D>, rhs: Node<'src, D>, span: SimpleSpan) -> Node<'src, D>
    {
        AstNode::new(
            Self::Binop {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            span,
        )
    }
}
