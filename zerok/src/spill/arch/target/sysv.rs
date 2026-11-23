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


pub struct SysV64;
impl Arch for SysV64
{
    type Gpr = SysV64Gpr;
    type Float = SysV64Xmm;

    fn class_of(kind: Kind) -> Option<RegClass>
    {
        todo!()
    }

    fn param_loc(params: &[RegClass]) -> Vec<ParamLoc>
    {
        todo!()
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysV64Gpr {}


impl RegFile for SysV64Gpr
{
    type Reg = Self;
    const USEABLE: todo!();
    const CALLER_SAVED: todo!();
    const CALLEE_SAVED: todo!();
    const RETURN: todo!();
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SysV64Xmm {}


impl RegFile for SysV64Xmm
{
    type Reg = Self;
    const USEABLE: todo!();
    const CALLER_SAVED: todo!();
    const CALLEE_SAVED: todo!();
    const RETURN: todo!();
}
