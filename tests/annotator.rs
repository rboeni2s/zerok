use std::rc::Rc;
use zerok::annotator::{Annotation, Env, Kind};
use zerok::parser;


/// Parses and annotates `input`, returning the type of the whole program or the type error message
fn kind_of(input: &str) -> Result<Kind, String>
{
    let mut ast = parser::parse_expr::<Option<Annotation>>(input, "").expect("Parsing failed");

    ast.annotate(&Rc::new(Env::default()))
        .map(|annotation| annotation.kind)
        .map_err(|(_, msg)| msg)
}


#[test]
fn literals_default_to_i32_and_f32()
{
    assert_eq!(kind_of("39"), Ok(Kind::I32));
    assert_eq!(kind_of("39f"), Ok(Kind::F32));
    assert_eq!(kind_of("3.5f"), Ok(Kind::F32));
    assert_eq!(kind_of("-1 + 2 * 3"), Ok(Kind::I32));
}


#[test]
fn literals_take_the_expected_type()
{
    assert_eq!(kind_of("sett a: u64 = 1; a"), Ok(Kind::U64));
    assert_eq!(kind_of("sett a: f64 = 1.5f * 2f; a"), Ok(Kind::F64));
    assert_eq!(kind_of("sett a: i64 = (1; 2 + 3)"), Ok(Kind::I64));

    // The literal takes the type of the other operand, no matter on which side it is
    assert_eq!(kind_of("sett a: u32 = 1; a + 1"), Ok(Kind::U32));
    assert_eq!(kind_of("sett a: u32 = 1; 1 + a"), Ok(Kind::U32));
    assert_eq!(kind_of("sett a: f64 = 1f; 2f * a"), Ok(Kind::F64));
}


#[test]
fn type_errors()
{
    // Integers and floats can not be mixed
    assert!(kind_of("1 + 1f").is_err());
    assert!(kind_of("sett a: f32 = 1").is_err());
    assert!(kind_of("sett a: i32 = 1f").is_err());

    // Different number types can not be mixed
    assert!(kind_of("sett a: u32 = 1; sett b: u64 = 2; a + b").is_err());

    // Unsigned numbers can not be negated
    assert!(kind_of("sett a: u32 = -1").is_err());
    assert!(kind_of("sett a: i32 = -1; a").is_ok());

    // Literals must fit into their type
    assert!(kind_of("sett a: i32 = 2147483648").is_err());
    assert!(kind_of("sett a: i64 = 2147483648").is_ok());
    assert!(kind_of("sett a: u32 = 4294967296").is_err());
}


#[test]
fn casts()
{
    assert_eq!(kind_of("sett a: u32 = 1; a as i64"), Ok(Kind::I64));
    assert_eq!(kind_of("sett a: f32 = 1.5f; a as u64 * 2"), Ok(Kind::U64));
    assert_eq!(
        kind_of("sett a: i32 = -1; sett b: u32 = a as u32; b"),
        Ok(Kind::U32)
    );

    // Literals take the target type of the cast
    assert_eq!(kind_of("4000000000 as u64"), Ok(Kind::U64));
    assert!(kind_of("4000000000 as i32").is_err());

    // Integer literals can be cast to floats
    assert_eq!(kind_of("5 as f64"), Ok(Kind::F64));

    // Only numbers can be cast
    assert!(kind_of("'text' as u32").is_err());
    assert!(kind_of("1 as string").is_err());
    assert!(kind_of("1 as foo").is_err());
}
