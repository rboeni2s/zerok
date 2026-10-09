use super::env::{Env, EnvEntry};
use crate::parser::{
    atom::Atom,
    ops::{Binop, Unaop},
};
use std::fmt;


/// The Type of a node in the ast
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
}


impl Kind
{
    pub(super) fn from_str(value: &str) -> Option<Self>
    {
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
            _ => None,
        }
    }

    pub(super) fn is_int(self) -> bool
    {
        matches!(self, Kind::U32 | Kind::U64 | Kind::I32 | Kind::I64)
    }

    pub(super) fn is_float(self) -> bool
    {
        matches!(self, Kind::F32 | Kind::F64)
    }

    pub(super) fn is_numeric(self) -> bool
    {
        self.is_int() || self.is_float()
    }

    pub(super) fn is_signed(self) -> bool
    {
        matches!(self, Kind::I32 | Kind::I64) || self.is_float()
    }

    /// The type of `atom`, number literals take the `expected` type if it fits and default to i32 and f32 otherwise
    pub(super) fn of_atom(atom: &Atom, expected: Option<Kind>) -> Self
    {
        match atom
        {
            Atom::Str(_) => Kind::String,
            Atom::Int(_) => expected.filter(|kind| kind.is_int()).unwrap_or(Kind::I32),
            Atom::Float(_) => expected.filter(|kind| kind.is_float()).unwrap_or(Kind::F32),
            Atom::Nop => Kind::None,
        }
    }

    /// Checks if the value of a number literal can be represented by this type
    pub(super) fn fits(self, atom: &Atom) -> bool
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
        };

        write!(f, "{name}")
    }
}


//HACK: Replace this function with a proper lut
pub(super) fn binop_compat<'a>(
    op: &Binop,
    lhs: Kind,
    rhs: Kind,
    _env: &Env<'a, EnvEntry>,
) -> Option<Kind>
{
    match (op, lhs, rhs)
    {
        (Binop::Add | Binop::Sub | Binop::Div | Binop::Mul | Binop::Mod | Binop::Pow, lhs, rhs)
            if lhs == rhs && lhs.is_numeric() =>
        {
            Some(lhs)
        }
        _ => None,
    }
}


//HACK: Replace this function with a proper lut
pub(super) fn unaop_compat<'a>(op: &Unaop, arg: Kind, _env: &Env<'a, EnvEntry>) -> Option<Kind>
{
    match (op, arg)
    {
        (Unaop::Neg, arg) if arg.is_signed() => Some(arg),
        _ => None,
    }
}


pub(super) fn cast_compat(from: Kind, to: Kind) -> bool
{
    from == to || (from.is_numeric() && to.is_numeric())
}
