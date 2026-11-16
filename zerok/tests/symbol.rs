use zerok::symbol::Symbol;


#[test]
fn root_symbols_are_only_prefixed()
{
    assert_eq!(Symbol::root("add").mangled(), "zk_add");
    assert_eq!(Symbol::root("add_return").mangled(), "zk_add_return");
}


#[test]
fn modules_are_length_prefixed()
{
    let symbol = |module: &[&'static str], name| {
        Symbol {
            module: module.to_vec(),
            name,
        }
        .mangled()
    };

    assert_eq!(symbol(&["math"], "add"), "zk_4math_add");
    assert_eq!(symbol(&["math", "vec"], "add"), "zk_4math3vec_add");

    // A `_` in a module or function name can not be confused with the separator
    assert_ne!(symbol(&["a"], "b_c"), symbol(&["a_b"], "c"));
}
