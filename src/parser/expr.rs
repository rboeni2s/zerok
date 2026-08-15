use super::atom::Atom;
use super::ops::{Binop, Unaop};
use super::{P, Synt};
use chumsky::pratt::*;
use chumsky::prelude::*;


#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src>
{
    Atom(Atom),

    Binop
    {
        op: Binop,
        lhs: Box<Expr<'src>>,
        rhs: Box<Expr<'src>>,
    },

    Unaop
    {
        op: Unaop,
        val: Box<Expr<'src>>,
    },

    Decl
    {
        name: &'src str,
        kind: &'src str,
        val: Box<Expr<'src>>,
    },

    Chain(Vec<Expr<'src>>),

    // Placeholder for a something that could not be parsed.
    ParseError,
}


impl<'src> Expr<'src>
{
    pub fn parser() -> impl P<'src, Self>
    {
        // Create the atom parser
        let atom = Atom::parser().padded().map(Self::Atom);

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
                    });

                // Check if this expressions is "geklammert"
                let parenthesized = chain.clone().delimited_by(
                    just(Synt::LParen.repr()).padded(),
                    just(Synt::RParen.repr()).padded(),
                );

                // Operands to operators can either be an atom or a (chained) expression
                let operand = atom.clone().or(parenthesized).or(decl);

                // Parse the different operators and set their associativity and precedence
                operand.pratt((
                    infix(left(0), Binop::add(), |lhs, op, rhs, _| Self::Binop {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }),
                    infix(left(0), Binop::sub(), |lhs, op, rhs, _| Self::Binop {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }),
                    infix(left(1), Binop::mul(), |lhs, op, rhs, _| Self::Binop {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }),
                    infix(left(1), Binop::div(), |lhs, op, rhs, _| Self::Binop {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }),
                    infix(left(1), Binop::modulo(), |lhs, op, rhs, _| Self::Binop {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }),
                    infix(right(3), Binop::pow(), |lhs, op, rhs, _| Self::Binop {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }),
                    prefix(2, Unaop::neg(), |op, val, _| Self::Unaop {
                        op,
                        val: Box::new(val),
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
                .map(|(mut chain, term)| match term
                {
                    Some(_) =>
                    {
                        if let Self::Chain(chain) = &mut chain
                        {
                            chain.push(Expr::Atom(Atom::Nop));
                        }

                        chain
                    }
                    None => chain,
                })
                // Try to skip malformed input in case of an error, so that the rest of the input can be checked
                //TODO: This doesnt work, i should debug and fix this
                .recover_with(skip_until(any().ignored(), terminator().rewind(), || {
                    Self::ParseError
                })),
        );

        chain
    }
}
