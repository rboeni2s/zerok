use super::{Expr, Node, P, Token};
use chumsky::prelude::*;


/// A parameter of a function, e.g. `a: i32`
#[derive(Debug, Clone)]
pub struct Param<'src>
{
    pub name: &'src str,
    pub kind: &'src str,
    pub span: SimpleSpan,
}


/// Function definition:
/// * `op add(a: i32, b: i32) -> i32 { a + b }`
/// * `op the_void(doomed: u32) {}`
#[derive(Debug, Clone)]
pub struct Function<'src, D>
{
    pub name: &'src str,
    pub params: Vec<Param<'src>>,

    /// The return type, `None` if the function does not return anything
    pub ret: Option<&'src str>,

    pub body: Node<'src, D>,
    pub span: SimpleSpan,
}


/// A program is a vector of function definitions
#[derive(Debug, Clone, PartialEq)]
pub struct Program<'src, D>
{
    pub functions: Vec<Function<'src, D>>,
}


impl PartialEq for Param<'_>
{
    fn eq(&self, other: &Self) -> bool
    {
        // Do not compare the spans
        self.name == other.name && self.kind == other.kind
    }
}


impl<D: PartialEq> PartialEq for Function<'_, D>
{
    fn eq(&self, other: &Self) -> bool
    {
        // Do not compare the spans
        self.name == other.name
            && self.params == other.params
            && self.ret == other.ret
            && self.body == other.body
    }
}


impl<'src, D> Program<'src, D>
where
    D: Default + 'src,
{
    pub fn parser<'t>() -> impl P<'t, 'src, Self>
    where
        'src: 't,
    {
        let ident = select! { Token::Ident(name) => name }.labelled("identifier");

        let param = ident
            .then_ignore(just(Token::Doublecolon))
            .then(ident)
            .map_with(|(name, kind), info| {
                Param {
                    name,
                    kind,
                    span: info.span(),
                }
            });

        let params = param
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen));

        let ret = just(Token::Arrow).ignore_then(ident).or_not();

        let body = Expr::parser().delimited_by(just(Token::LBrace), just(Token::RBrace));

        let function = just(Token::Fn)
            .ignore_then(ident)
            .then(params)
            .then(ret)
            .then(body)
            .map_with(|(((name, params), ret), body), info| {
                Function {
                    name,
                    params,
                    ret,
                    body,
                    span: info.span(),
                }
            });

        function
            .repeated()
            .collect()
            .map(|functions| Self { functions })
    }
}
