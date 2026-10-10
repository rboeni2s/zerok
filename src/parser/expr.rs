use crate::parser::ast::AstNode;

use super::atom::Atom;
use super::ops::{Binop, Unaop};
use super::{Node, P, Token};
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
        kind: &'src str,
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
                    .then(ident)
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
                let operand = choice((atom.clone(), parenthesized, decl, ret, call, binding));

                // Parse the different operators and set their associativity and precedence
                operand.pratt((
                    infix(
                        left(0),
                        Binop::add().or(Binop::sub()),
                        |lhs, op, rhs, info| Self::binop(op, lhs, rhs, info.span()),
                    ),
                    infix(
                        left(1),
                        choice((Binop::mul(), Binop::div(), Binop::modulo())),
                        |lhs, op, rhs, info| Self::binop(op, lhs, rhs, info.span()),
                    ),
                    infix(right(4), Binop::pow(), |lhs, op, rhs, info| {
                        Self::binop(op, lhs, rhs, info.span())
                    }),
                    // "-a as u32" is "(-a) as u32" and "a * b as u32" is "a * (b as u32)"
                    postfix(2, just(Token::As).ignore_then(ident), |val, kind, info| {
                        AstNode::new(
                            Self::Cast {
                                val: Box::new(val),
                                kind,
                            },
                            info.span(),
                        )
                    }),
                    prefix(3, Unaop::neg(), |op, val, info| {
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
                        condition: todo!(),
                        body: todo!(),
                        elifs,
                        else_body: else_body.map(Box::new),
                    },
                    info.span(),
                )
            })
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
