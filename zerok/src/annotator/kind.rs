use super::env::{Env, EnvEntry};
use crate::parser::{
    Type,
    atom::Atom,
    ops::{Binop, Unaop},
};
use std::{
    collections::HashSet,
    fmt,
    sync::{LazyLock, Mutex},
};


/// The Type of a node in the ast
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Kind
{
    U32,
    U64,
    I32,
    I64,
    F32,
    F64,
    String,
    None,
    Bool,
    Ref(&'static Self),
}


impl Kind
{
    pub fn referenced(self) -> Self
    {
        // Ich hätte gerne, dass Kind: Copy bleibt, Also leake ich für jeden Typ einfach einmal
        // eine reference zu ihm so dass ich in Kind::Ref(referenced) eine statische reference für referenced benutzen kann
        //
        // Ich würde super gern einen statischen lut benutzen mit den enum descriminants, dann kann man aber
        // keine (beliebigen) Ref(Ref(kind)) haben. Also muss ich den lut als hashmap zur laufzeit bauen...
        // wenn ich später user defined types habe brauche ich das sowie so...
        static LEAKY_TABLE: LazyLock<Mutex<HashSet<&'static Kind>>> =
            LazyLock::new(Default::default);

        // Mit auto deref wird &Kind draus... ich brauche aber spezifisch die leaked &'static aus dem table
        #[allow(clippy::explicit_auto_deref)]
        Self::Ref(
            *LEAKY_TABLE
                .lock()
                .unwrap()
                .get_or_insert_with(&self, |_| Box::leak(Box::new(self))),
        )
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<Self>
    {
        if let Some(inner) = value.strip_prefix("&")
        {
            return Self::from_str(inner).map(Self::referenced);
        }

        match value
        {
            "u32" => Some(Kind::U32),
            "u64" => Some(Kind::U64),
            "i32" => Some(Kind::I32),
            "i64" => Some(Kind::I64),
            "f32" => Some(Kind::F32),
            "f64" => Some(Kind::F64),
            "string" => Some(Kind::String),
            "none" => Some(Kind::None),
            "bool" => Some(Kind::Bool),
            _ => None,
        }
    }

    pub fn from_type(ty: &Type<'_>) -> Option<Self>
    {
        match ty
        {
            Type::Name(name) => Self::from_str(name),
            Type::Ref(inner) => Self::from_type(inner).map(Self::referenced),
        }
    }

    pub fn is_int(self) -> bool
    {
        matches!(self, Kind::U32 | Kind::U64 | Kind::I32 | Kind::I64)
    }

    pub fn is_float(self) -> bool
    {
        matches!(self, Kind::F32 | Kind::F64)
    }

    pub fn is_numeric(self) -> bool
    {
        self.is_int() || self.is_float()
    }

    pub fn is_signed(self) -> bool
    {
        matches!(self, Kind::I32 | Kind::I64) || self.is_float()
    }

    /// The type of `atom`, number literals take the `expected` type if it fits and default to i32 and f32 otherwise
    pub fn of_atom(atom: &Atom, expected: Option<Kind>) -> Self
    {
        match atom
        {
            Atom::Str(_) => Kind::String,
            Atom::Int(_) => expected.filter(|kind| kind.is_int()).unwrap_or(Kind::I32),
            Atom::Float(_) => expected.filter(|kind| kind.is_float()).unwrap_or(Kind::F32),
            Atom::Bool(_) => Kind::Bool,
            Atom::Nop => Kind::None,
        }
    }

    /// Checks if the value of a number literal can be represented by this type
    pub fn fits(self, atom: &Atom) -> bool
    {
        match (atom, self)
        {
            (Atom::Int(n), Kind::U32) => *n <= u32::MAX as u64,
            (Atom::Int(n), Kind::I32) => *n <= i32::MAX as u64,
            (Atom::Int(n), Kind::I64) => *n <= i64::MAX as u64,
            (Atom::Float(n), Kind::F32) => n.abs() <= f32::MAX as f64,
            _ => true,
        }
    }
}


impl fmt::Display for Kind
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        let name = match self
        {
            Kind::U32 => "u32",
            Kind::U64 => "u64",
            Kind::I32 => "i32",
            Kind::I64 => "i64",
            Kind::F32 => "f32",
            Kind::F64 => "f64",
            Kind::String => "string",
            Kind::None => "none",
            Kind::Bool => "bool",
            Kind::Ref(kind) => return write!(f, "&{kind}"),
        };

        write!(f, "{name}")
    }
}


//HACK: Replace this function with a proper lut
pub fn binop_compat<'a>(op: &Binop, lhs: Kind, rhs: Kind, _env: &Env<'a, EnvEntry>)
-> Option<Kind>
{
    match (op, lhs, rhs)
    {
        (Binop::BoolAnd | Binop::BoolOr, Kind::Bool, Kind::Bool) => Some(Kind::Bool),

        (Binop::Add | Binop::Sub | Binop::Div | Binop::Mul | Binop::Mod | Binop::Pow, lhs, rhs)
            if lhs == rhs && lhs.is_numeric() =>
        {
            Some(lhs)
        }
        (Binop::Eq | Binop::Neq, lhs, rhs)
            if lhs == rhs && (lhs.is_numeric() || lhs == Kind::Bool) =>
        {
            Some(Kind::Bool)
        }
        (Binop::Lt | Binop::Gt | Binop::Lte | Binop::Gte, lhs, rhs)
            if lhs == rhs && lhs.is_numeric() =>
        {
            Some(Kind::Bool)
        }
        _ => None,
    }
}


//HACK: Replace this function with a proper lut
pub fn unaop_compat<'a>(op: &Unaop, arg: Kind, _env: &Env<'a, EnvEntry>) -> Option<Kind>
{
    match (op, arg)
    {
        (Unaop::Neg, arg) if arg.is_signed() => Some(arg),
        (Unaop::BoolNot, Kind::Bool) => Some(Kind::Bool),
        _ => None,
    }
}

//HACK: Replace this function with a proper lut if things other than number become catsable in the future
pub fn cast_compat(from: Kind, to: Kind) -> bool
{
    from == to || (from.is_numeric() && to.is_numeric())
}
