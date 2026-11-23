pub mod target
{
    mod sysv;
    pub use sysv::*;
}


use crate::{
    annotator::Kind,
    spill::{Color, RegClass},
};


/// Describes a register file (Register-Satz)
pub trait RegFile
{
    /// The concrete enum of the register file
    type Reg: Copy + Eq + std::fmt::Debug + 'static;

    /// Usable registers in order of prerefence
    const USEABLE: &'static [Self::Reg];

    /// Count of the useable registers
    const COLORS: usize = Self::USEABLE.len();

    /// Array of _caller_ saved registers, each entry must also appear in `Self::USEABLE`
    const CALLER_SAVED: &'static [Self::Reg];

    /// Array of _callee_ saved registers, each entry must also appear in `Self::USEABLE`
    const CALLEE_SAVED: &'static [Self::Reg];

    /// Return register, must also appear in `Self::Useable`
    const RETURN: Self::Reg;
}


/// Function Parameter Location
///
/// Function parameters can be passed to a function using designated registers or the stack
/// this is decided by the calling convention of the architecture.
pub enum ParamLoc
{
    /// This parameter is passed via a register
    Reg
    {
        /// Register class to use
        class: RegClass,
        /// specific register (color) of the register class
        color: Color,
    },

    Stack(usize),
}


/// Describes a architecture and its features
pub trait Arch
{
    /// The registerfile to use for general purpose registers
    type Gpr: RegFile;
    /// The registerfile to use for float registers
    type Float: RegFile;

    /// Returns `Some(RegClass)` for each `Kind` to indicate which register class to use for each data type.
    /// `None` is returned if `kind` does not need a register.
    fn class_of(kind: Kind) -> Option<RegClass>;

    /// Returns a vector if function parameter locations given a slice of parameter classes
    fn param_loc(params: &[RegClass]) -> Vec<ParamLoc>;
}
