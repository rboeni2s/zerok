use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{
    Result,
    Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};


mod kw
{
    syn::custom_keyword!(and);
    syn::custom_keyword!(or);
    syn::custom_keyword!(not);
    syn::custom_keyword!(define);
    syn::custom_keyword!(call);
    syn::custom_keyword!(ret);
    syn::custom_keyword!(goto);
}


enum IicReg
{
    R(syn::Expr),
    G(syn::Expr),
}


impl Parse for IicReg
{
    /// parse register expressions like "R(n + 1)" or "G(39)"
    fn parse(input: ParseStream) -> Result<Self>
    {
        let ident: syn::Ident = input.parse()?;

        let content;
        syn::parenthesized!(content in input);

        match ident.to_string().as_str()
        {
            "R" | "r" => Ok(Self::R(content.parse()?)),
            "G" | "g" => Ok(Self::G(content.parse()?)),
            _ =>
            {
                Err(syn::Error::new(
                    ident.span(),
                    "expected a register G(expr) or R(expr)",
                ))
            }
        }
    }
}


enum IicOp
{
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Lte,
    Eq,
    Neq,
    Gt,
    Gte,
    And,
    Or,
    Not,
}


impl Parse for IicOp
{
    fn parse(input: ParseStream) -> Result<Self>
    {
        let lookahead = input.lookahead1();

        let op = if lookahead.peek(Token![<=])
        {
            input.parse::<Token![<=]>()?;
            Self::Lte
        }
        else if lookahead.peek(Token![>=])
        {
            input.parse::<Token![>=]>()?;
            Self::Gte
        }
        else if lookahead.peek(Token![==])
        {
            input.parse::<Token![==]>()?;
            Self::Eq
        }
        else if lookahead.peek(Token![!=])
        {
            input.parse::<Token![!=]>()?;
            Self::Neq
        }
        else if lookahead.peek(Token![+])
        {
            input.parse::<Token![+]>()?;
            Self::Add
        }
        else if lookahead.peek(Token![-])
        {
            input.parse::<Token![-]>()?;
            Self::Sub
        }
        else if lookahead.peek(Token![*])
        {
            input.parse::<Token![*]>()?;
            Self::Mul
        }
        else if lookahead.peek(Token![/])
        {
            input.parse::<Token![/]>()?;
            Self::Div
        }
        else if lookahead.peek(Token![<])
        {
            input.parse::<Token![<]>()?;
            Self::Lt
        }
        else if lookahead.peek(Token![>])
        {
            input.parse::<Token![>]>()?;
            Self::Gt
        }
        else if lookahead.peek(kw::and)
        {
            input.parse::<kw::and>()?;
            Self::And
        }
        else if lookahead.peek(kw::or)
        {
            input.parse::<kw::or>()?;
            Self::Or
        }
        else if lookahead.peek(kw::not)
        {
            input.parse::<kw::not>()?;
            Self::Not
        }
        else
        {
            return Err(lookahead.error());
        };

        Ok(op)
    }
}


enum IicVal
{
    Reg(syn::Expr),
    Val(syn::Expr),
}


impl Parse for IicVal
{
    fn parse(input: ParseStream) -> Result<Self>
    {
        parse_val_until(input, |input| input.peek(Token![,]), "an operand")
    }
}


enum IicChunk
{
    // reg = lhs op rhs
    Bin
    {
        op: IicOp,
        lhs: IicVal,
        rhs: IicVal,
        reg: IicReg,
    },

    // reg = op arg
    Una
    {
        op: IicOp, arg: IicVal, reg: IicReg
    },

    // reg = val
    Bind
    {
        val: IicVal, reg: IicReg
    },

    // define name(arg1, arg2, arg3, ...):
    Func
    {
        name: syn::Expr, args: Vec<IicReg>
    },

    // reg = call name(arg1, arg2, arg3, ...)
    Call
    {
        name: syn::Expr,
        args: Vec<IicVal>,
        reg: IicReg,
    },

    // ret val
    Ret
    {
        val: IicVal
    },

    // name:
    Label
    {
        name: syn::Expr
    },

    // goto name
    Jump
    {
        name: syn::Expr
    },

    // if val goto name
    CondJump
    {
        val: IicVal, name: syn::Expr
    },
}


fn parse_expr_until(
    input: ParseStream,
    stop: impl Fn(ParseStream) -> bool,
    what: &str,
) -> Result<syn::Expr>
{
    syn::parse2(collect_until(input, stop, what)?)
}


/// Like `parse_expr_until`, but recognizes a lone `R(expr)` as a register
fn parse_val_until(
    input: ParseStream,
    stop: impl Fn(ParseStream) -> bool,
    what: &str,
) -> Result<IicVal>
{
    let tokens = collect_until(input, stop, what)?;
    let mut iter = tokens.clone().into_iter();

    if let (Some(TokenTree::Ident(ident)), Some(TokenTree::Group(group)), None) =
        (iter.next(), iter.next(), iter.next())
        && (ident == "R" || ident == "r")
        && group.delimiter() == Delimiter::Parenthesis
    {
        return Ok(IicVal::Reg(syn::parse2(group.stream())?));
    }

    Ok(IicVal::Val(syn::parse2(tokens)?))
}


fn collect_until(
    input: ParseStream,
    stop: impl Fn(ParseStream) -> bool,
    what: &str,
) -> Result<TokenStream>
{
    let mut tokens = TokenStream::new();

    while !input.is_empty() && !stop(input)
    {
        if input.peek(Token![::])
        {
            input.parse::<Token![::]>()?.to_tokens(&mut tokens);
            continue;
        }

        tokens.extend([input.parse::<TokenTree>()?]);
    }

    if tokens.is_empty()
    {
        return Err(input.error(format!("expected {what}")));
    }

    Ok(tokens)
}


fn at_end(input: ParseStream) -> bool
{
    input.is_empty() || input.peek(Token![;])
}


fn peek_op(input: ParseStream) -> bool
{
    input.fork().parse::<IicOp>().is_ok()
}


/// Checks for a `(..)` group that is directly followed by `follow`
fn peek_trailing_parens(input: ParseStream, follow: fn(ParseStream) -> bool) -> bool
{
    if !input.peek(syn::token::Paren)
    {
        return false;
    }

    let fork = input.fork();
    fork.parse::<TokenTree>().is_ok() && follow(&fork)
}


fn peek_colon(input: ParseStream) -> bool
{
    input.peek(Token![:]) && !input.peek(Token![::])
}


impl Parse for IicChunk
{
    fn parse(input: ParseStream) -> Result<Self>
    {
        // define name(arg1, arg2, arg3, ...):
        if input.peek(kw::define)
        {
            input.parse::<kw::define>()?;

            let name = parse_expr_until(
                input,
                |input| peek_trailing_parens(input, peek_colon),
                "a function name",
            )?;

            let content;
            syn::parenthesized!(content in input);
            let args = Punctuated::<IicReg, Token![,]>::parse_terminated(&content)?;
            input.parse::<Token![:]>()?;

            return Ok(Self::Func {
                name,
                args: args.into_iter().collect(),
            });
        }

        // name:
        if !input.peek(kw::ret)
            && !input.peek(kw::goto)
            && !input.peek(Token![if])
            && !peek_assignment(input)
        {
            let name = parse_expr_until(input, peek_colon, "a label name")?;
            input.parse::<Token![:]>()?;

            return Ok(Self::Label { name });
        }

        let chunk = if input.peek(kw::ret)
        {
            // ret val
            input.parse::<kw::ret>()?;
            let val = parse_val_until(input, at_end, "a return value")?;

            Self::Ret { val }
        }
        else if input.peek(kw::goto)
        {
            // goto name
            input.parse::<kw::goto>()?;
            let name = parse_expr_until(input, at_end, "a label name")?;

            Self::Jump { name }
        }
        else if input.peek(Token![if])
        {
            // if val goto name
            input.parse::<Token![if]>()?;
            let val = parse_val_until(input, |input| input.peek(kw::goto), "a condition")?;
            input.parse::<kw::goto>()?;
            let name = parse_expr_until(input, at_end, "a label name")?;

            Self::CondJump { val, name }
        }
        else
        {
            parse_assignment(input)?
        };

        // Statements are terminated by `;`, only the last one may omit it
        if !input.is_empty()
        {
            input.parse::<Token![;]>()?;
        }

        Ok(chunk)
    }
}


fn peek_assignment(input: ParseStream) -> bool
{
    let fork = input.fork();
    fork.parse::<IicReg>().is_ok() && fork.peek(Token![=]) && !fork.peek(Token![==])
}


/// Parses all chunks of the form `reg = ...`
fn parse_assignment(input: ParseStream) -> Result<IicChunk>
{
    let reg: IicReg = input.parse()?;
    input.parse::<Token![=]>()?;

    // reg = op arg
    if peek_op(input)
    {
        let op = input.parse()?;
        let arg = parse_val_until(input, at_end, "an operand")?;

        return Ok(IicChunk::Una { op, arg, reg });
    }

    // reg = call name(arg1, arg2, arg3, ...)
    if input.peek(kw::call)
    {
        input.parse::<kw::call>()?;

        let name = parse_expr_until(
            input,
            |input| peek_trailing_parens(input, at_end),
            "a function name",
        )?;

        let content;
        syn::parenthesized!(content in input);
        let args = Punctuated::<IicVal, Token![,]>::parse_terminated(&content)?;

        return Ok(IicChunk::Call {
            name,
            args: args.into_iter().collect(),
            reg,
        });
    }

    let lhs = parse_val_until(input, |input| at_end(input) || peek_op(input), "a value")?;

    // reg = lhs op rhs
    if peek_op(input)
    {
        let op = input.parse()?;
        let rhs = parse_val_until(input, at_end, "an operand")?;

        return Ok(IicChunk::Bin { op, lhs, rhs, reg });
    }

    // reg = val
    Ok(IicChunk::Bind { val: lhs, reg })
}


struct IicChunks(Vec<IicChunk>);


impl Parse for IicChunks
{
    fn parse(input: ParseStream) -> Result<Self>
    {
        let mut chunks = Vec::new();

        while !input.is_empty()
        {
            chunks.push(input.parse()?);
        }

        Ok(Self(chunks))
    }
}


impl ToTokens for IicReg
{
    fn to_tokens(&self, tokens: &mut TokenStream)
    {
        tokens.extend(match self
        {
            IicReg::R(val) => quote!(::ir::IrReg::R(#val)),
            IicReg::G(val) => quote!(::ir::IrReg::G(#val)),
        });
    }
}


impl ToTokens for IicOp
{
    fn to_tokens(&self, tokens: &mut TokenStream)
    {
        let variant = match self
        {
            IicOp::Add => quote!(Add),
            IicOp::Sub => quote!(Sub),
            IicOp::Mul => quote!(Mul),
            IicOp::Div => quote!(Div),
            IicOp::Lt => quote!(Lt),
            IicOp::Lte => quote!(Lte),
            IicOp::Eq => quote!(Eq),
            IicOp::Neq => quote!(Neq),
            IicOp::Gt => quote!(Gt),
            IicOp::Gte => quote!(Gte),
            IicOp::And => quote!(And),
            IicOp::Or => quote!(Or),
            IicOp::Not => quote!(Not),
        };

        tokens.extend(quote!(::ir::IrOp::#variant));
    }
}


impl ToTokens for IicVal
{
    fn to_tokens(&self, tokens: &mut TokenStream)
    {
        tokens.extend(match self
        {
            IicVal::Reg(reg) => quote!(::ir::ToIrReg::to_ir_reg(#reg)),
            IicVal::Val(val) => quote!(::ir::ToIrVal::to_ir_val(#val)),
        });
    }
}


impl ToTokens for IicChunk
{
    fn to_tokens(&self, tokens: &mut TokenStream)
    {
        // Names are converted the same way the IrBuilder does it
        let name =
            |name: &syn::Expr| quote!(::std::convert::Into::<::std::string::String>::into(#name));

        tokens.extend(match self
        {
            IicChunk::Bin { op, lhs, rhs, reg } =>
            {
                quote!(::ir::IrChunk::Bin { op: #op, lhs: #lhs, rhs: #rhs, reg: #reg })
            }

            IicChunk::Una { op, arg, reg } =>
            {
                quote!(::ir::IrChunk::Una { op: #op, arg: #arg, reg: #reg })
            }

            IicChunk::Bind { val, reg } => quote!(::ir::IrChunk::Bind { val: #val, reg: #reg }),

            IicChunk::Func { name: func, args } =>
            {
                let func = name(func);
                quote!(::ir::IrChunk::Func { name: #func, args: ::std::vec![#(#args),*] })
            }

            IicChunk::Call { name: func, args, reg } =>
            {
                let func = name(func);
                quote!(::ir::IrChunk::Call { name: #func, args: ::std::vec![#(#args),*], reg: #reg })
            }

            IicChunk::Ret { val } => quote!(::ir::IrChunk::Ret { val: #val }),

            IicChunk::Label { name: label } =>
            {
                let label = name(label);
                quote!(::ir::IrChunk::Label { name: #label })
            }

            IicChunk::Jump { name: label } =>
            {
                let label = name(label);
                quote!(::ir::IrChunk::Jump { name: #label })
            }

            IicChunk::CondJump { val, name: label } =>
            {
                let label = name(label);
                quote!(::ir::IrChunk::CondJump { val: #val, name: #label })
            }
        });
    }
}

#[proc_macro]
pub fn iic_quote(input: proc_macro::TokenStream) -> proc_macro::TokenStream
{
    let IicChunks(chunks) = syn::parse_macro_input!(input as IicChunks);
    quote!(::std::vec![#(#chunks),*]).into()
}
