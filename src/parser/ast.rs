use crate::parser::Expr;
use chumsky::span::SimpleSpan;
use std::marker::PhantomData;
use std::range::Range;


#[derive(Debug, Clone)]
pub struct AstNode<'a, T, D>
{
    pub(crate) inner: T,
    pub(crate) span: SimpleSpan,
    pub(crate) data: D,
    pub(crate) _phantom: PhantomData<&'a ()>,
}


impl<'a, T, D> AstNode<'a, T, D>
where
    D: Default,
{
    pub fn new(inner: T, span: SimpleSpan) -> Self
    {
        Self {
            inner,
            span,
            data: Default::default(),
            _phantom: PhantomData,
        }
    }
}


impl<'a, T: PartialEq, D> PartialEq for AstNode<'a, T, D>
{
    fn eq(&self, other: &Self) -> bool
    {
        // Do not compare the spans, phantoms ot the data
        self.inner == other.inner
    }
}


impl<'a, D> From<Expr<'a, D>> for AstNode<'a, Expr<'a, D>, D>
where
    D: Default,
{
    fn from(value: Expr<'a, D>) -> Self
    {
        Self::new(value, Default::default())
    }
}
