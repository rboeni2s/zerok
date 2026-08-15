use zerok::parser::{self, P, ast_builder::*};


fn parse<'a, T: P<'a, Node<'a, ()>>>(
    parser: &T,
    input: &'a str,
) -> Result<Node<'a, ()>, anyhow::Error>
{
    parser::parse(parser.clone(), input, "")
}


#[test]
fn basic_chaining()
{
    let parser = &Expr::parser();

    assert_eq!(parse(parser, "10").unwrap(), chain![num(10.0)]);

    assert_eq!(
        parse(parser, "10; 39; \"Test\"").unwrap(),
        chain![num(10), num(39), atom_str("Test")]
    );

    assert_eq!(
        parse(parser, "10; 39; \"Test\";").unwrap(),
        chain![num(10), num(39), atom_str("Test"), nop()]
    );

    assert_eq!(
        parse(parser, "12+7; -39").unwrap(),
        chain![add(num(12), num(7)), neg(num(39))]
    );

    assert_eq!(
        parse(parser, "(12*(11-10); -39); (1; (2; 3); 4);").unwrap(),
        chain![
            chain![mul(num(12), chain![sub(num(11), num(10))]), neg(num(39))],
            chain![num(1), chain![num(2), num(3)], num(4)],
            nop()
        ]
    );

    assert_eq!(
        parse(parser, "decl test: int = 0; -39").unwrap(),
        chain![decl("test", "int", num(0)), neg(num(39))]
    );

    assert_eq!(
        parse(parser, "decl test: int = (0; -39)").unwrap(),
        chain![decl("test", "int", chain![num(0), neg(num(39))])]
    );
}
