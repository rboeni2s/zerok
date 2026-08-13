use super::atom::Atom;
use super::ops::{Binop, Unaop};
use super::{P, Synt};
use chumsky::pratt::*;
use chumsky::prelude::*;


#[derive(Debug, Clone)]
pub enum Expr
{
    Atom(Atom),

    Binop
    {
        op: Binop,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    Unaop
    {
        op: Unaop,
        val: Box<Expr>,
    },

    Chain(Vec<Expr>),
}


impl<'src> Expr
{
    pub fn parser() -> impl P<'src, Self>
    {
        // Create the atom parser
        let atom = Atom::parser().padded().map(Self::Atom);

        recursive(move |expr| {
            // Check if this expressions is "geklammert"
            let parenthesized = expr.clone().delimited_by(
                just(Synt::LParen.repr()).padded(),
                just(Synt::RParen.repr()).padded(),
            );

            // Operands to operators can either be an atom or a (chained) expression
            let operand = atom.clone().or(parenthesized);

            // Parse the different operators and set their associativity and precedence
            let term_expr = operand.pratt((
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
            ));

            // Chain term_exprs separated by ";"
            let expr_chain = term_expr
                .separated_by(just(Synt::Semicolon.repr()).padded())
                .collect::<Vec<_>>()
                .map(Self::Chain);

            // Check if the expression chain ends in ";" and add a nop node if it does
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
        })
    }
}
