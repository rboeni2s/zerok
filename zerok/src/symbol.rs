/// Prefix of every symbol (function) the compiler generates in the ir
pub const SYMBOL_PREFIX: &str = "zk_";

/// Prefix of every label the compiler generates in the ir. It differs from [`SYMBOL_PREFIX`],
/// because ir labels and functions share one namespace and must never collide
pub const LABEL_PREFIX: &str = "zkl_";


/// A fully qualified name, e.g. `math::vec::add`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol<'a>
{
    /// The path of the module the symbol is defined in, empty for the root module
    pub module: Vec<&'a str>,
    pub name: &'a str,
}


impl<'a> Symbol<'a>
{
    /// A symbol of the root module
    pub fn root(name: &'a str) -> Self
    {
        Self {
            module: Vec::new(),
            name,
        }
    }

    /// The path of the symbol without any prefix. Every module is prefixed with its length:
    ///
    /// * `add`            -> `add`
    /// * `math::add`      -> `4math_add`
    /// * `math::vec::add` -> `4math3vec_add`
    ///
    /// This is unambiguous, because identifiers can not start with a digit
    pub fn mangled_path(&self) -> String
    {
        let mut mangled = String::new();

        for module in &self.module
        {
            mangled.push_str(&module.len().to_string());
            mangled.push_str(module);
        }

        if !self.module.is_empty()
        {
            mangled.push('_');
        }

        mangled.push_str(self.name);
        mangled
    }

    /// The name of the symbol in the ir, e.g. `zk_add`
    pub fn mangled(&self) -> String
    {
        format!("{SYMBOL_PREFIX}{}", self.mangled_path())
    }
}
