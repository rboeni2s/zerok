use chumsky::prelude::*;
use std::fmt;


pub const KW_DECL: &str = "sett";
pub const KW_NOP: &str = "keenop";
pub const KW_AS: &str = "as";
pub const KW_FN: &str = "op";
pub const KW_RET: &str = "geve";
pub const KW_IF: &str = "wenn";
pub const KW_ELSE: &str = "anners";
pub const KW_TRUE: &str = "ja";
pub const KW_FALSE: &str = "nee";
pub const KW_LOOP: &str = "sling";
pub const KW_BREAK: &str = "rut";
pub const KW_CONTINUE: &str = "over";


pub type Spanned<T> = (T, SimpleSpan);


#[derive(Debug, Clone, PartialEq)]
pub enum Token<'src>
{
    Num(u64),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(String),
    Ident(&'src str),

    // Keywords
    Decl,
    As,
    Fn,
    Return,
    Nop,
    If,
    Else,
    Loop,
    Break,
    Continue,

    // Symbols
    BoolAnd,
    BoolOr,
    BoolNot,
    Ref,
    RefEq,
    Semicolon,
    Doublecolon,
    Arrow,
    EqEq,
    NotEq,
    Lt,
    Gt,
    Lte,
    Gte,
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
    LBracket,
    RBracket,
    Comma,
    Eq,
    Tick,
}


impl fmt::Display for Token<'_>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            Token::Num(n) => write!(f, "{n}"),
            Token::Float(n) => write!(f, "{n}f"),
            Token::Bool(true) => write!(f, "{KW_TRUE}"),
            Token::Bool(false) => write!(f, "{KW_FALSE}"),
            Token::Char(c) => write!(f, "{c:?}"),
            Token::Str(s) => write!(f, "{s:?}"),
            Token::Ident(name) => write!(f, "{name}"),
            Token::Decl => write!(f, "{KW_DECL}"),
            Token::Nop => write!(f, "{KW_NOP}"),
            Token::As => write!(f, "{KW_AS}"),
            Token::Fn => write!(f, "{KW_FN}"),
            Token::Return => write!(f, "{KW_RET}"),
            Token::If => write!(f, "{KW_IF}"),
            Token::Else => write!(f, "{KW_ELSE}"),
            Token::Semicolon => write!(f, ";"),
            Token::Doublecolon => write!(f, ":"),
            Token::Arrow => write!(f, "->"),
            Token::EqEq => write!(f, "=="),
            Token::NotEq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::Lte => write!(f, "<="),
            Token::Gte => write!(f, ">="),
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
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Comma => write!(f, ","),
            Token::Eq => write!(f, "="),
            Token::Loop => write!(f, "{KW_LOOP}"),
            Token::Break => write!(f, "{KW_BREAK}"),
            Token::Continue => write!(f, "{KW_CONTINUE}"),
            Token::Tick => write!(f, "'"),
            Token::BoolAnd => write!(f, "&&"),
            Token::BoolOr => write!(f, "||"),
            Token::BoolNot => write!(f, "!"),
            Token::Ref => write!(f, "&"),
            Token::RefEq => write!(f, "&="),
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
    let num = text::int(10)
        .validate(|digits: &str, info, emitter| {
            digits.parse().unwrap_or_else(|_| {
                emitter.emit(Rich::custom(
                    info.span(),
                    format!("Integer literal {digits} is too large"),
                ));
                0
            })
        })
        .map(Token::Num);

    // Floats need an "f" suffix, e.g. "39f" or "3.5f"
    let float = text::int(10)
        .then(just('.').then(text::digits(10)).or_not())
        .to_slice()
        .then_ignore(just('f'))
        // Make sure the "f" is not the start of an identifier, e.g. "39foo"
        .then_ignore(
            any()
                .filter(|c: &char| c.is_alphanumeric() || *c == '_')
                .not(),
        )
        .from_str()
        .unwrapped()
        .map(Token::Float);

    // string escapes wie "\n", "\'" or "\u{1F600}"
    let escape = just('\\').ignore_then(choice((
        just('n').to('\n'),
        just('t').to('\t'),
        just('r').to('\r'),
        just('0').to('\0'),
        just('\\'),
        just('\''),
        just('"'),
        text::digits(16)
            .to_slice()
            .delimited_by(just("u{"), just('}'))
            .validate(|digits: &str, info, emitter| {
                u32::from_str_radix(digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .unwrap_or_else(|| {
                        emitter.emit(Rich::custom(
                            info.span(),
                            format!("\\u{{{digits}}} is not a unicode character"),
                        ));
                        '\0'
                    })
            }),
        any().validate(|c: char, info, emitter| {
            emitter.emit(Rich::custom(
                info.span(),
                format!("Unknown escape sequence \\{c}"),
            ));
            c
        }),
    )));

    // A char is a single unicode codepoint
    let char = escape
        .clone()
        .or(none_of("\\'\n"))
        .delimited_by(just('\''), just('\''))
        .map(Token::Char);

    // Manche buchstaben sind aus mehreren codepoints gemacht etwa ein hand emoji mit hautfarbe '👍🏽'
    // anstatt mich damit rumzuärgern verbiete ich sowas einfach (erstmal)
    let long_char = none_of("'\n")
        .filter(|c: &char| !c.is_whitespace() && *c != ';')
        .repeated()
        .at_least(2)
        .to_slice()
        .delimited_by(just('\''), just('\''))
        .validate(|chars: &str, info, emitter| {
            emitter.emit(Rich::custom(
                info.span(),
                format!(
                    "A char is a single unicode codepoint, but '{chars}' consists of {} codepoints",
                    chars.chars().count()
                ),
            ));
            chars.chars().next().unwrap_or_default()
        })
        .map(Token::Char);

    let string = escape
        .or(none_of("\\\""))
        .repeated()
        .collect::<String>()
        .delimited_by(just('"'), just('"'))
        .map(Token::Str);

    // Identifiers are lexed as a whole, so that keywords are only recognized if they are not a prefix of an identifier
    let ident = text::ident().map(|ident| {
        match ident
        {
            KW_DECL => Token::Decl,
            KW_NOP => Token::Nop,
            KW_AS => Token::As,
            KW_FN => Token::Fn,
            KW_RET => Token::Return,
            KW_IF => Token::If,
            KW_ELSE => Token::Else,
            KW_TRUE => Token::Bool(true),
            KW_FALSE => Token::Bool(false),
            KW_BREAK => Token::Break,
            KW_CONTINUE => Token::Continue,
            KW_LOOP => Token::Loop,
            _ => Token::Ident(ident),
        }
    });

    // Multi character symbols have to be checked before the single character symbols they start with
    let multi_char_symbol = choice((
        just("**").to(Token::StarStar),
        just("&&").to(Token::BoolAnd),
        just("&=").to(Token::RefEq),
        just("||").to(Token::BoolOr),
        just("->").to(Token::Arrow),
        just("==").to(Token::EqEq),
        just("!=").to(Token::NotEq),
        just("<=").to(Token::Lte),
        just(">=").to(Token::Gte),
    ));

    let symbol = choice((
        multi_char_symbol,
        just("!").to(Token::BoolNot),
        just("&").to(Token::Ref),
        just("<").to(Token::Lt),
        just(">").to(Token::Gt),
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
        just("[").to(Token::LBracket),
        just("]").to(Token::RBracket),
        just(",").to(Token::Comma),
        just("=").to(Token::Eq),
        just("'").to(Token::Tick),
    ));

    // Chars have to be checked before the "'" of labels
    let token = choice((float, num, char, long_char, string, ident, symbol))
        .map_with(|token, info| (token, info.span()))
        // Skip characters that do not start a valid token, so that the rest of the input can still be lexed
        .recover_with(skip_then_retry_until(any().ignored(), end()));

    whitespace().ignore_then(token.then_ignore(whitespace()).repeated().collect())
}
