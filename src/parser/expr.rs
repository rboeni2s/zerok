use crate::parser::ast::AstNode;

use super::atom::Atom;
use super::ops::{Binop, Unaop};
use super::{Node, P, Synt};
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
        kind: &'src str,
        val: Box<Node<'src, D>>,
    },

    Chain(Vec<Node<'src, D>>),

    // Placeholder for a something that could not be parsed.
    ParseError,
}


impl<'src, D> Expr<'src, D>
where
    D: Default + 'src,
{
    pub fn parser() -> impl P<'src, Node<'src, D>>
    {
        // Create the atom parser
        let atom = Atom::parser()
            .padded()
            .map(Self::Atom)
            .map_with(|atom, span| Node::new(atom, span.span()));

        // Split the recursive parser declaration of a expression chain into the declaration and definition part, to allow
        // building the term expression in between, this allows for complex expressions to be chained with the correct chaining precedence
        let mut chain = Recursive::declare();

        let term = recursive({
            let chain = chain.clone();
            move |term| {
                // Check if this expression is a declaration
                let decl = just(Synt::Decl.repr())
                    .padded()
                    .ignore_then(text::ident().padded())
                    .then_ignore(just(Synt::Doublecolon.repr()).padded())
                    .then(text::ident().padded())
                    .then_ignore(just(Synt::Eq.repr()).padded())
                    .then(term.clone())
                    .map(|((name, kind), val)| Self::Decl {
                        name,
                        kind,
                        val: Box::new(val),
                    })
                    .map_with(|expr, span| AstNode::new(expr, span.span()));

                // Check if this expressions is "geklammert"
                let parenthesized = chain.clone().delimited_by(
                    just(Synt::LParen.repr()).padded(),
                    just(Synt::RParen.repr()).padded(),
                );

                // Operands to operators can either be an atom or a (chained) expression
                let operand = atom.clone().or(parenthesized).or(decl);

                // Parse the different operators and set their associativity and precedence
                operand.pratt((
                    infix(left(0), Binop::add(), |lhs, op, rhs, info| {
                        AstNode::new(
                            Self::Binop {
                                op,
                                lhs: Box::new(lhs),
                                rhs: Box::new(rhs),
                            },
                            info.span(),
                        )
                    }),
                    infix(left(0), Binop::sub(), |lhs, op, rhs, info| {
                        AstNode::new(
                            Self::Binop {
                                op,
                                lhs: Box::new(lhs),
                                rhs: Box::new(rhs),
                            },
                            info.span(),
                        )
                    }),
                    infix(left(1), Binop::mul(), |lhs, op, rhs, info| {
                        AstNode::new(
                            Self::Binop {
                                op,
                                lhs: Box::new(lhs),
                                rhs: Box::new(rhs),
                            },
                            info.span(),
                        )
                    }),
                    infix(left(1), Binop::div(), |lhs, op, rhs, info| {
                        AstNode::new(
                            Self::Binop {
                                op,
                                lhs: Box::new(lhs),
                                rhs: Box::new(rhs),
                            },
                            info.span(),
                        )
                    }),
                    infix(left(1), Binop::modulo(), |lhs, op, rhs, info| {
                        AstNode::new(
                            Self::Binop {
                                op,
                                lhs: Box::new(lhs),
                                rhs: Box::new(rhs),
                            },
                            info.span(),
                        )
                    }),
                    infix(right(3), Binop::pow(), |lhs, op, rhs, info| {
                        AstNode::new(
                            Self::Binop {
                                op,
                                lhs: Box::new(lhs),
                                rhs: Box::new(rhs),
                            },
                            info.span(),
                        )
                    }),
                    prefix(2, Unaop::neg(), |op, val, info| {
                        AstNode::new(
                            Self::Unaop {
                                op,
                                val: Box::new(val),
                            },
                            info.span(),
                        )
                    }),
                ))
            }
        });

        // Chain terms separated by ";"
        let expr_chain = term
            .separated_by(just(Synt::Semicolon.repr()).padded())
            .collect::<Vec<_>>()
            .map(Self::Chain);

        // Describes something that could be the end of an expression.
        // This is used for error recovery on malformed input
        let terminator = || {
            choice((
                just(Synt::Semicolon.repr()).ignored(),
                just(Synt::RParen.repr()).ignored(),
                end(),
            ))
        };

        // Check if the expression chain ends in ";" and add a nop node if it does
        chain.define(
            expr_chain
                .then(just(Synt::Semicolon.repr()).padded().or_not())
                .map_with(|(mut chain, term), info| match term
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
                })
                // Try to skip malformed input in case of an error, so that the rest of the input can be checked
                //TODO: This doesnt work, i should debug and fix this
                .recover_with(skip_until(any().ignored(), terminator().rewind(), || {
                    Self::ParseError.into()
                })),
        );

        chain
    }
}
