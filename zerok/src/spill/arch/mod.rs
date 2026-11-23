pub mod target
{
    mod sysv;
    pub use sysv::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub enum Target
    {
        SysV64,
    }
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

    /// Return register, must also appear in `Self::USEABLE` (except for an [`EmptyFile`])
    const RETURN: Self::Reg;

    /// Scratch register for spill code.
    /// Must _not_ appear in `Self::USEABLE`.
    const SCRATCH: Self::Reg;

    /// The register of color `color`
    fn reg_of(color: Color) -> Self::Reg
    {
        Self::USEABLE[color.0]
    }

    /// The color of `reg`, or `None` if `reg` is not useable
    fn color_of(reg: Self::Reg) -> Option<Color>
    {
        Self::USEABLE.iter().position(|&r| r == reg).map(Color)
    }
}


/// Placeholder register file for a class the architecture has no registers for,
/// e.g. `Arch::Float` on a soft-float architecture.
///
/// The file has no useable registers (`COLORS == 0`). `Arch::class_of` of such an architecture
/// must never return the class of this file, so there are no nodes of this class and the
/// coloring never needs one of its colors.
///
/// `RETURN` and `SCRATCH` only exist because the trait requires them: they are
/// [`EmptyFile::Placeholder`], which is no real register and has no color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EmptyFile
{
    /// Stands for a register that does not exist
    Placeholder,
}


impl RegFile for EmptyFile
{
    type Reg = Self;

    const USEABLE: &'static [Self] = &[];
    const CALLER_SAVED: &'static [Self] = &[];
    const CALLEE_SAVED: &'static [Self] = &[];
    const RETURN: Self = Self::Placeholder;
    const SCRATCH: Self = Self::Placeholder;
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
