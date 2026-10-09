use chumsky::prelude::*;
use std::fmt;


pub const KW_DECL: &str = "sett";
pub const KW_NOP: &str = "nop";


pub type Spanned<T> = (T, SimpleSpan);


#[derive(Debug, Clone, PartialEq)]
pub enum Token<'src>
{
    Num(f64),
    Str(&'src str),
    Ident(&'src str),

    // Keywords
    Decl,
    Nop,

    // Symbols
    Semicolon,
    Doublecolon,
    Plus,
    Minus,
    Star,
    StarStar,
    Slash,
    Percent,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Eq,
}


impl fmt::Display for Token<'_>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Token::Num(n) => write!(f, "{n}"),
            Token::Str(s) => write!(f, "\"{s}\""),
            Token::Ident(name) => write!(f, "{name}"),
            Token::Decl => write!(f, "{KW_DECL}"),
            Token::Nop => write!(f, "{KW_NOP}"),
            Token::Semicolon => write!(f, ";"),
            Token::Doublecolon => write!(f, ":"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::StarStar => write!(f, "**"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Comma => write!(f, ","),
            Token::Eq => write!(f, "="),
        }
    }
}


/// Skips any amount of whitespace. INCLUDING comments
fn whitespace<'src>() -> impl Parser<'src, &'src str, (), extra::Err<Rich<'src, char>>> + Clone
{
    let line_comment = just("//").then(none_of("\n").repeated()).ignored();

    // Block comments may be nested, so every "/*" inside a comment needs its own matching "*/"
    let block_comment = recursive(|block_comment| {
        just("/*")
            .then(choice((block_comment, any().and_is(just("*/").not()).ignored())).repeated())
            .then(just("*/"))
            .ignored()
    });

    // A "/*" that is not consumed by `block_comment` was never closed, report it instead of lexing it as "/" and "*"
    let unterminated_comment = just("/*")
        .then(any().repeated())
        .validate(|_, info, emitter| {
            emitter.emit(Rich::custom(info.span(), "Unterminated block comment"))
        });

    choice((
        text::whitespace().at_least(1),
        line_comment,
        block_comment,
        unterminated_comment,
    ))
    .repeated()
}


pub fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char>>>
{
    let num = text::int(10).from_str().unwrapped().map(Token::Num);

    let string = one_of("\"'")
        .ignore_then(none_of("\"'").repeated().to_slice())
        .then_ignore(one_of("\"'"))
        .map(Token::Str);

    // Identifiers are lexed as a whole, so that keywords are only recognized if they are not a prefix of an identifier
    let ident = text::ident().map(|ident| {
        match ident
        {
            KW_DECL => Token::Decl,
            KW_NOP => Token::Nop,
            _ => Token::Ident(ident),
        }
    });

    let symbol = choice((
        just("**").to(Token::StarStar),
        just(";").to(Token::Semicolon),
        just(":").to(Token::Doublecolon),
        just("+").to(Token::Plus),
        just("-").to(Token::Minus),
        just("*").to(Token::Star),
        just("/").to(Token::Slash),
        just("%").to(Token::Percent),
        just("(").to(Token::LParen),
        just(")").to(Token::RParen),
        just("{").to(Token::LBrace),
        just("}").to(Token::RBrace),
        just(",").to(Token::Comma),
        just("=").to(Token::Eq),
    ));

    let token = choice((num, string, ident, symbol))
        .map_with(|token, info| (token, info.span()))
        // Skip characters that do not start a valid token, so that the rest of the input can still be lexed
        .recover_with(skip_then_retry_until(any().ignored(), end()));

    whitespace().ignore_then(token.then_ignore(whitespace()).repeated().collect())
}
