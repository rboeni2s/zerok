//! # This module describes SystemV 64bit
//!
//! ## Calling conventions (`https://wiki.osdev.org/System_V_ABI#x86-64`)
//! This is a 64-bit platform. The stack grows downwards. Parameters to functions are passed
//! in via the registers rdi, rsi, rdx, rcx, r8, and r9. Floating-point parameters are passed
//!  in via xmm0 through xmm7. Unlike the standard parameter passing registers, floating point
//! registers are not preserved between function calls. Any additional arguments that do not
//! fit in these registers are passed on the stack in reverse order. Parameters passed via the
//! stack may be modified by the called function. Functions are called using the call instruction,
//! which pushes the address of the next instruction onto the stack and jumps to the operand.
//! Functions return to the caller using the ret instruction, which pops the return address from the stack and jumps to it.
//! The stack is 16-byte aligned just before the call instruction is executed.
//! Functions preserve the registers rbx, rsp, rbp, r12, r13, r14, and r15;
//! while rax, rdi, rsi, rdx, rcx, r8, r9, r10, r11 are scratch registers.
//! The return value is stored in the rax register, or if it is a 128-bit value,
//! then the higher 64-bits go in rdx. Optionally, functions push rbp such that
//! the caller-return-rip is 8 bytes above it, and set rbp to the address of the saved rbp.
//! This allows iterating through the existing stack frames. This can be eliminated by specifying
//! the -fomit-frame-pointer GCC option.
//! If the return value does not fit in rax and rdx, then the caller must reserve storage space
//! and pass a pointer in rdi as-if it was the first argument. The callee must return the same pointer in rax.


use crate::spill::arch::*;


/// System V AMD64 ABI
pub struct SysV64;


impl SysV64
{
    /// Registers for integer/pointer parameters, in parameter order
    const GPR_PARAMS: &'static [SysV64Gpr] = &[
        SysV64Gpr::Rdi,
        SysV64Gpr::Rsi,
        SysV64Gpr::Rdx,
        SysV64Gpr::Rcx,
        SysV64Gpr::R8,
        SysV64Gpr::R9,
    ];

    /// Registers for float parameters, in parameter order
    const FLOAT_PARAMS: &'static [SysV64Xmm] = &[
        SysV64Xmm::Xmm0,
        SysV64Xmm::Xmm1,
        SysV64Xmm::Xmm2,
        SysV64Xmm::Xmm3,
        SysV64Xmm::Xmm4,
        SysV64Xmm::Xmm5,
        SysV64Xmm::Xmm6,
        SysV64Xmm::Xmm7,
    ];
}


impl Arch for SysV64
{
    type Gpr = SysV64Gpr;
    type Float = SysV64Xmm;

    fn class_of(kind: Kind) -> Option<RegClass>
    {
        match kind
        {
            Kind::None => None,
            Kind::F32 | Kind::F64 => Some(RegClass::Float),
            Kind::U32
            | Kind::U64
            | Kind::I32
            | Kind::I64
            | Kind::Char
            | Kind::Bool
            | Kind::Ref(_)
            | Kind::AnyRef
            | Kind::Intern => Some(RegClass::Gpr),
        }
    }

    fn param_loc(params: &[RegClass]) -> Vec<ParamLoc>
    {
        let mut next_gpr = 0;
        let mut next_float = 0;
        let mut next_stack = 0;

        let mut stack = || {
            next_stack += 1;
            ParamLoc::Stack(next_stack - 1)
        };

        params
            .iter()
            .map(|&class| match class
            {
                RegClass::Gpr => match Self::GPR_PARAMS.get(next_gpr)
                {
                    Some(&reg) =>
                    {
                        next_gpr += 1;
                        ParamLoc::Reg {
                            class,
                            color: SysV64Gpr::color_of(reg)
                                .expect("gpr parameter register must be useable"),
                        }
                    }
                    None => stack(),
                },
                RegClass::Float => match Self::FLOAT_PARAMS.get(next_float)
                {
                    Some(&reg) =>
                    {
                        next_float += 1;
                        ParamLoc::Reg {
                            class,
                            color: SysV64Xmm::color_of(reg)
                                .expect("float parameter register must be useable"),
                        }
                    }
                    None => stack(),
                },
            })
            .collect()
    }
}


/// The 64-bit general purpose registers
///
/// Not every register is useable for the allocation:
/// - `rsp` is the stack pointer
/// - `rbp` is the frame pointer
/// - `r11` is the scratch register for spill code (it is caller saved, so it never has to be preserved)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SysV64Gpr
{
    Rax,
    Rbx,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    Rsp,
    Rbp,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}


impl SysV64Gpr
{
    /// Stack pointer
    pub const STACK_POINTER: Self = Self::Rsp;
    /// Frame pointer
    pub const FRAME_POINTER: Self = Self::Rbp;
}


impl RegFile for SysV64Gpr
{
    type Reg = Self;

    // Caller saved first: they cost nothing as long as there is no call in between.
    // Callee saved registers have to be saved in the prologue once they are used.
    const USEABLE: &'static [Self] = &[
        Self::Rax,
        Self::Rcx,
        Self::Rdx,
        Self::Rsi,
        Self::Rdi,
        Self::R8,
        Self::R9,
        Self::R10,
        Self::Rbx,
        Self::R12,
        Self::R13,
        Self::R14,
        Self::R15,
    ];

    // r11 is caller saved too, but it is the scratch register and therefore not useable
    const CALLER_SAVED: &'static [Self] = &[
        Self::Rax,
        Self::Rcx,
        Self::Rdx,
        Self::Rsi,
        Self::Rdi,
        Self::R8,
        Self::R9,
        Self::R10,
    ];

    // rbp is callee saved too, but it is the frame pointer and therefore not useable
    const CALLEE_SAVED: &'static [Self] = &[Self::Rbx, Self::R12, Self::R13, Self::R14, Self::R15];
    const RETURN: Self = Self::Rax;
    const SCRATCH: Self = Self::R11;
}


/// The sse registers
///
/// `xmm15` is the scratch register for spill code and not useable for the allocation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SysV64Xmm
{
    Xmm0,
    Xmm1,
    Xmm2,
    Xmm3,
    Xmm4,
    Xmm5,
    Xmm6,
    Xmm7,
    Xmm8,
    Xmm9,
    Xmm10,
    Xmm11,
    Xmm12,
    Xmm13,
    Xmm14,
    Xmm15,
}


impl RegFile for SysV64Xmm
{
    type Reg = Self;

    const USEABLE: &'static [Self] = &[
        Self::Xmm0,
        Self::Xmm1,
        Self::Xmm2,
        Self::Xmm3,
        Self::Xmm4,
        Self::Xmm5,
        Self::Xmm6,
        Self::Xmm7,
        Self::Xmm8,
        Self::Xmm9,
        Self::Xmm10,
        Self::Xmm11,
        Self::Xmm12,
        Self::Xmm13,
        Self::Xmm14,
    ];

    const CALLER_SAVED: &'static [Self] = Self::USEABLE; // All sse registers are caller saved
    const CALLEE_SAVED: &'static [Self] = &[];
    const RETURN: Self = Self::Xmm0;
    const SCRATCH: Self = Self::Xmm15;
}
