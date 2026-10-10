mod common;

use common::*;
use zerok::parser;


fn parse(input: &str) -> Result<Node<'_, ()>, anyhow::Error>
{
    parser::parse_expr(input, "")
}


fn parse_program(input: &str) -> Result<Program<'_, ()>, anyhow::Error>
{
    parser::parse_prog(input, "")
}


#[test]
fn basic_chaining()
{
    assert_eq!(parse("10").unwrap(), chain![num(10)]);

    assert_eq!(
        parse("10; 39; \"Test\"").unwrap(),
        chain![num(10), num(39), atom_str("Test")]
    );

    assert_eq!(
        parse("10; 39; \"Test\";").unwrap(),
        chain![num(10), num(39), atom_str("Test"), nop()]
    );

    assert_eq!(
        parse("12+7; -39").unwrap(),
        chain![add(num(12), num(7)), neg(num(39))]
    );

    assert_eq!(
        parse("(12*(11-10); -39); (1; (2; 3); 4);").unwrap(),
        chain![
            chain![mul(num(12), chain![sub(num(11), num(10))]), neg(num(39))],
            chain![num(1), chain![num(2), num(3)], num(4)],
            nop()
        ]
    );

    assert_eq!(
        parse("sett test: int = 0; -39").unwrap(),
        chain![decl("test", "int", num(0)), neg(num(39))]
    );

    assert_eq!(
        parse("sett test: int = (0; -39)").unwrap(),
        chain![decl("test", "int", chain![num(0), neg(num(39))])]
    );
}


#[test]
fn comments()
{
    assert_eq!(
        parse("// Kommentar\n10; // Kommentar\n39").unwrap(),
        chain![num(10), num(39)]
    );

    assert_eq!(
        parse("/* mehrere\n Zeilen */ 10 /* x */ / 2; /**/ 3 /* * / */").unwrap(),
        chain![div(num(10), num(2)), num(3)]
    );

    assert_eq!(
        parse("/* a /* b /* c */ */ d */ 10; /* /**/ */ 3").unwrap(),
        chain![num(10), num(3)]
    );

    assert!(parse("10 /* nicht geschlossen").is_err());
    assert!(parse("10 /* a /* b */ nicht geschlossen").is_err());
}


#[test]
fn keywords_and_identifiers()
{
    assert_eq!(
        parse("sett nopx: num = 1; setts").unwrap(),
        chain![
            decl("nopx", "num", num(1)),
            Expr::Binding { name: "setts" }.into()
        ]
    );
}


#[test]
fn only_comments()
{
    assert_eq!(
        parse("// nur ein Kommentar\n/* und noch einer */").unwrap(),
        chain![]
    );
}


#[test]
fn casts()
{
    assert_eq!(
        parse("a as u32").unwrap(),
        chain![cast(binding("a"), "u32")]
    );

    // "as" binds stronger than binary operators, but weaker than "-"
    assert_eq!(
        parse("1 + a * b as u32").unwrap(),
        chain![add(num(1), mul(binding("a"), cast(binding("b"), "u32")))]
    );
    assert_eq!(
        parse("-a as i64").unwrap(),
        chain![cast(neg(binding("a")), "i64")]
    );
    assert_eq!(
        parse("a as u32 as f64").unwrap(),
        chain![cast(cast(binding("a"), "u32"), "f64")]
    );
    assert_eq!(
        parse("sett x: u64 = a as u64").unwrap(),
        chain![decl("x", "u64", cast(binding("a"), "u64"))]
    );
}


#[test]
fn operator_precedence_and_associativity()
{
    // Operators with the same precedence are left associative
    assert_eq!(
        parse("10 - 2 + 3").unwrap(),
        chain![add(sub(num(10), num(2)), num(3))]
    );
    assert_eq!(
        parse("8 / 2 * 3 % 5").unwrap(),
        chain![modulo(mul(div(num(8), num(2)), num(3)), num(5))]
    );

    // "**" is right associative and binds stronger than "-"
    assert_eq!(
        parse("-2 ** 3 ** 2").unwrap(),
        chain![neg(pow(num(2), pow(num(3), num(2))))]
    );
    assert_eq!(
        parse("1 + 2 * 3").unwrap(),
        chain![add(num(1), mul(num(2), num(3)))]
    );
}


#[test]
fn calls()
{
    assert_eq!(parse("f()").unwrap(), chain![call("f", vec![])]);
    assert_eq!(
        parse("add(1, a * 2,)").unwrap(),
        chain![call("add", vec![num(1), mul(binding("a"), num(2))])]
    );

    // Calls can be nested and used as operands, chains as arguments need parentheses
    assert_eq!(
        parse("-f(g(1), (2; 3)) + 1").unwrap(),
        chain![add(
            neg(call(
                "f",
                vec![call("g", vec![num(1)]), chain![num(2), num(3)]]
            )),
            num(1)
        )]
    );

    assert!(parse("f(1; 2)").is_err());
}


#[test]
fn functions()
{
    assert_eq!(
        parse_program("op add(a: i32, b: i32) -> i32 { a+b}").unwrap(),
        Program {
            functions: vec![function(
                "add",
                &[("a", "i32"), ("b", "i32")],
                Some("i32"),
                chain![add(binding("a"), binding("b"))]
            )]
        }
    );

    // Functions without parameters, return type or body
    assert_eq!(
        parse_program("op main() { add(1, 2); } op nothing() {}").unwrap(),
        Program {
            functions: vec![
                function(
                    "main",
                    &[],
                    None,
                    chain![call("add", vec![num(1), num(2)]), nop()]
                ),
                function("nothing", &[], None, chain![]),
            ]
        }
    );

    assert_eq!(parse_program("").unwrap(), Program { functions: vec![] });
}


#[test]
fn only_functions_at_toplevel()
{
    assert!(parse_program("1 + 2").is_err());
    assert!(parse_program("op main() { 1 } 1 + 2").is_err());
    assert!(parse_program("op main() { op inner() { 1 } }").is_err());

    // Malformed heads
    assert!(parse_program("op main { 1 }").is_err());
    assert!(parse_program("op main() -> { 1 }").is_err());
    assert!(parse_program("op main(a) { 1 }").is_err());
    assert!(parse_program("op main() 1").is_err());
}


#[test]
fn returns()
{
    assert_eq!(parse("return").unwrap(), chain![ret(None)]);
    assert_eq!(parse("return 1").unwrap(), chain![ret(Some(num(1)))]);
    assert_eq!(parse("return;").unwrap(), chain![ret(None), nop()]);

    // The returned value is a whole term, but not the rest of the chain
    assert_eq!(
        parse("return a + 2 * b; 3").unwrap(),
        chain![
            ret(Some(add(binding("a"), mul(num(2), binding("b"))))),
            num(3)
        ]
    );
    assert_eq!(
        parse("return (1; 2)").unwrap(),
        chain![ret(Some(chain![num(1), num(2)]))]
    );
    assert_eq!(
        parse("return f(1) as u32").unwrap(),
        chain![ret(Some(cast(call("f", vec![num(1)]), "u32")))]
    );

    // A return without a value at the end of a body or parenthesized chain
    assert_eq!(
        parse_program("op main() { return }").unwrap(),
        Program {
            functions: vec![function("main", &[], None, chain![ret(None)])]
        }
    );
    assert_eq!(parse("(return)").unwrap(), chain![chain![ret(None)]]);

    // "return" is a keyword and can not be used as an identifier
    assert!(parse("sett return: i32 = 1").is_err());
    assert_eq!(parse("returns").unwrap(), chain![binding("returns")]);
}
