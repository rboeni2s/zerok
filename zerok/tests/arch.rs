//! Tests of the architecture descriptions

use zerok::annotator::Kind;
use zerok::spill::RegClass;
use zerok::spill::arch::target::{SysV64, SysV64Gpr, SysV64Xmm};
use zerok::spill::arch::{Arch, ParamLoc, RegFile};


/// Checks the documented invariants of a register file
fn check_file<F: RegFile>()
{
    let useable = F::USEABLE;

    for (i, reg) in useable.iter().enumerate()
    {
        assert!(!useable[..i].contains(reg), "{reg:?} is useable twice");
        assert_eq!(F::color_of(*reg), Some(i));
        assert_eq!(F::reg_of(i), *reg);
    }

    for reg in F::CALLER_SAVED.iter().chain(F::CALLEE_SAVED)
    {
        assert!(useable.contains(reg), "{reg:?} is saved but not useable");
    }

    for reg in F::CALLER_SAVED
    {
        assert!(!F::CALLEE_SAVED.contains(reg), "{reg:?} is caller and callee saved");
    }

    assert!(useable.contains(&F::RETURN));

    assert!(!useable.contains(&F::SCRATCH), "scratch register {:?} is useable", F::SCRATCH);
    assert_eq!(F::color_of(F::SCRATCH), None);
    assert_eq!(F::COLORS, useable.len());
}


#[test]
fn sysv64_register_files()
{
    check_file::<SysV64Gpr>();
    check_file::<SysV64Xmm>();

    assert_eq!(SysV64Gpr::COLORS, 13);
    assert_eq!(SysV64Xmm::COLORS, 15);

    // every useable gpr is either caller or callee saved
    assert_eq!(SysV64Gpr::CALLER_SAVED.len() + SysV64Gpr::CALLEE_SAVED.len(), SysV64Gpr::COLORS);

    assert_eq!(SysV64Gpr::SCRATCH, SysV64Gpr::R11);
    assert_eq!(SysV64Xmm::SCRATCH, SysV64Xmm::Xmm15);

    for reserved in [SysV64Gpr::STACK_POINTER, SysV64Gpr::FRAME_POINTER]
    {
        assert_eq!(SysV64Gpr::color_of(reserved), None);
        assert_ne!(reserved, SysV64Gpr::SCRATCH);
    }
}


#[test]
fn sysv64_class_of()
{
    assert_eq!(SysV64::class_of(Kind::None), None);
    assert_eq!(SysV64::class_of(Kind::F32), Some(RegClass::Float));
    assert_eq!(SysV64::class_of(Kind::F64), Some(RegClass::Float));

    for kind in [Kind::U32, Kind::U64, Kind::I32, Kind::I64, Kind::Char, Kind::Bool, Kind::I64.referenced()]
    {
        assert_eq!(SysV64::class_of(kind), Some(RegClass::Gpr), "{kind:?}");
    }
}


/// Translates the parameter locations back into register names
fn locs(params: &[RegClass]) -> Vec<String>
{
    SysV64::param_loc(params)
        .into_iter()
        .map(|loc| match loc
        {
            ParamLoc::Reg {
                class: RegClass::Gpr,
                color,
            } => format!("{:?}", SysV64Gpr::reg_of(color)),
            ParamLoc::Reg {
                class: RegClass::Float,
                color,
            } => format!("{:?}", SysV64Xmm::reg_of(color)),
            ParamLoc::Stack(slot) => format!("Stack({slot})"),
        })
        .collect()
}


#[test]
fn sysv64_param_loc()
{
    use RegClass::{Float as F, Gpr as G};

    assert!(locs(&[]).is_empty());

    // every class counts on its own
    assert_eq!(locs(&[G, F, G]), ["Rdi", "Xmm0", "Rsi"]);

    assert_eq!(
        locs(&[G, G, G, G, G, G, G, F, G]),
        ["Rdi", "Rsi", "Rdx", "Rcx", "R8", "R9", "Stack(0)", "Xmm0", "Stack(1)"]
    );

    // the 9th float goes to the stack, ints still get registers
    let mut params = vec![F; 9];
    params.push(G);
    params.push(F);
    assert_eq!(
        locs(&params),
        ["Xmm0", "Xmm1", "Xmm2", "Xmm3", "Xmm4", "Xmm5", "Xmm6", "Xmm7", "Stack(0)", "Rdi", "Stack(1)"]
    );
}
