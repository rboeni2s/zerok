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


/// Parses and annotates the program `input`, returning the type error message if there is one
fn check_program(input: &str) -> Result<(), String>
{
    let mut program = parser::parse_prog::<Option<Annotation>>(input, "").expect("Parsing failed");

    program
        .annotate(&Rc::new(Env::default()))
        .map_err(|(_, msg)| msg)
}


#[test]
fn functions()
{
    assert!(check_program("op main() {}").is_ok());
    assert!(check_program("op main() -> i32 { 1 }").is_ok());
    assert!(check_program("op add(a: i32, b: i32) -> i32 { a + b } op main() {}").is_ok());

    // The body has to return the return type, functions without one return nothing
    assert!(check_program("op main() -> u64 { sett a: i32 = 1; a }").is_err());
    assert!(check_program("op main() { 1 }").is_err());
    assert!(check_program("op main() { 1; }").is_ok());

    // The return type is the expected type of the body, so literals take it
    assert!(check_program("op main() -> u64 { 4000000000 }").is_ok());

    // Parameters are only visible inside their function
    assert!(check_program("op f(a: i32) {} op main() -> i32 { a }").is_err());

    // Signatures have to be valid and unique
    assert!(check_program("op main(a: foo) {}").is_err());
    assert!(check_program("op main() -> foo {}").is_err());
    assert!(check_program("op f(a: i32, a: i32) {} op main() {}").is_err());
    assert!(check_program("op main() {} op main() {}").is_err());

    // There has to be a main function without parameters
    assert!(check_program("op f() {}").is_err());
    assert!(check_program("op main(a: i32) {}").is_err());
}


#[test]
fn calls()
{
    let add = "op add(a: i32, b: i32) -> i32 { a + b }";

    assert!(check_program(&format!("{add} op main() -> i32 {{ add(1, 2) }}")).is_ok());
    assert!(
        check_program(&format!(
            "{add} op main() -> i32 {{ add(add(1, 2), 3) * 2 }}"
        ))
        .is_ok()
    );

    // Calls can come before the definition of the function, so recursion works
    assert!(check_program("op main() -> i32 { f(1) } op f(n: i32) -> i32 { f(n - 1) }").is_ok());
    assert!(check_program("op main() { even(1); } op even(n: u32) -> u32 { odd(n) } op odd(n: u32) -> u32 { even(n) }").is_ok());

    // Arguments take the type of their parameter
    assert!(check_program("op f(a: u64) {} op main() { f(4000000000); }").is_ok());
    assert!(check_program("op f(a: i32) {} op main() { f(1f); }").is_err());
    assert!(check_program("op f(a: i32) {} op main() { sett a: u32 = 1; f(a); }").is_err());

    // Number of arguments
    assert!(check_program(&format!("{add} op main() {{ add(1); }}")).is_err());
    assert!(check_program(&format!("{add} op main() {{ add(1, 2, 3); }}")).is_err());

    // Unknown functions
    assert!(check_program("op main() { nope(); }").is_err());

    // A call to a function without a return type has no value
    assert!(check_program("op f() {} op main() -> i32 { f() + 1 }").is_err());
    assert!(check_program("op f() {} op main() { f() }").is_ok());
}


#[test]
fn returns()
{
    assert!(check_program("op main() -> i32 { geve 1 }").is_ok());
    assert!(check_program("op main() -> i32 { geve 1; }").is_ok());
    assert!(check_program("op main() -> i32 { sett a: i32 = 1; geve a; }").is_ok());
    assert!(check_program("op main() { geve }").is_ok());
    assert!(check_program("op main() { geve; }").is_ok());

    // The returned value has to have the return type of the function
    assert!(check_program("op main() -> i32 { geve 1f }").is_err());
    assert!(check_program("op main() -> i32 { sett a: u32 = 1; geve a }").is_err());
    assert!(check_program("op main() { geve 1 }").is_err());
    assert!(check_program("op main() -> i32 { geve }").is_err());

    // Literals take the return type of the function
    assert!(check_program("op main() -> u64 { geve 4000000000 }").is_ok());

    // A return in the middle of a body does not change the type of the body
    assert!(check_program("op main() -> i32 { geve 1; 2 }").is_ok());
    assert!(check_program("op main() -> i32 { geve 1; 2f }").is_err());

    // Returns inside of expressions
    assert!(check_program("op main() -> i32 { 1 + geve 2 }").is_ok());
    assert!(check_program("op main() -> i32 { sett a: i32 = (geve 1); a }").is_ok());

    // Return types of other functions do not matter
    assert!(check_program("op f() -> u64 { geve 1 } op main() -> i32 { geve 1 }").is_ok());

    // There is no function to return from outside of functions
    assert!(kind_of("geve 1").is_err());
}


#[test]
fn none_values()
{
    // None can not be the type of a parameter or the target of a cast
    assert!(check_program("op f(x: none) {} op main() {}").is_err());
    assert!(kind_of("keenop as none").is_err());
    assert!(kind_of("1 as none").is_err());

    // But it can be the type of a binding and the return type of a function
    assert_eq!(kind_of("sett x: none = keenop; x"), Ok(Kind::None));
    assert!(check_program("op main() -> none { geve keenop }").is_ok());
}


#[test]
fn booleans()
{
    assert_eq!(kind_of("ja"), Ok(Kind::Bool));
    assert_eq!(kind_of("sett b: bool = nee; b"), Ok(Kind::Bool));
    assert!(check_program("op f(b: bool) -> bool { b } op main() { f(ja); }").is_ok());

    // Booleans are not numbers
    assert!(kind_of("sett b: i32 = ja").is_err());
    assert!(kind_of("sett b: bool = 1").is_err());
    assert!(kind_of("ja + nee").is_err());
    assert!(kind_of("-ja").is_err());
    assert!(kind_of("ja as i32").is_err());
}
