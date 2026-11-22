use std::collections::HashMap;

use crate::annotator::{EnvEntry, Kind};
use ir::{IrChunk, IrReg, IrTypedReg};


/// Register used by a IR-instruction
pub trait CfgRegs
{
    /// Returns a vector of all local registers being read by this instruction
    fn cfg_use(&self) -> Vec<usize>;

    /// Returns a vector of all local registers being written to by this instruction
    fn cfg_def(&self) -> Vec<usize>;
}


/// Extracts the R register number from a value (like `EnvEntry` or `IrTypedReg`)
trait GetRegR
{
    fn get_r_reg_of(&self) -> Option<usize>;
}


impl GetRegR for EnvEntry
{
    fn get_r_reg_of(&self) -> Option<usize>
    {
        match self
        {
            EnvEntry::Register(reg) => Some(*reg),
            EnvEntry::Atom(_) => None,
        }
    }
}


impl GetRegR for IrTypedReg<Kind>
{
    fn get_r_reg_of(&self) -> Option<usize>
    {
        match self.reg
        {
            IrReg::R(reg) => Some(reg),
            IrReg::G(_) => None,
        }
    }
}


fn operands<'a, G: GetRegR + 'a>(vals: impl IntoIterator<Item = &'a G>) -> Vec<usize>
{
    vals.into_iter()
        .filter_map(|item| item.get_r_reg_of())
        .collect()
}


impl CfgRegs for IrChunk<EnvEntry, Kind>
{
    fn cfg_use(&self) -> Vec<usize>
    {
        match self
        {
            IrChunk::Bin { lhs, rhs, .. } => operands([lhs, rhs]),
            IrChunk::Una { arg, .. } => operands([arg]),
            IrChunk::Bind { val, .. } => operands([val]),
            IrChunk::Call { args, .. } => operands(args),
            IrChunk::Ret { val } => operands([val]),
            IrChunk::CondJump { val, .. } => operands([val]),
            IrChunk::MemAlloc { size, .. } => operands([size]),

            // `base` is an `IrTypedReg`, but it is only read: the write goes to memory
            IrChunk::MemWrite {
                value,
                base,
                offset,
            } =>
            {
                let mut regs = operands([value, offset]);
                regs.extend(base.get_r_reg_of());
                regs
            }

            IrChunk::MemRead { base, offset, .. } => operands([base, offset]),

            IrChunk::Func { .. } | IrChunk::Label { .. } | IrChunk::Jump { .. } => Vec::new(),
        }
    }

    fn cfg_def(&self) -> Vec<usize>
    {
        match self
        {
            IrChunk::Bin { reg, .. }
            | IrChunk::Una { reg, .. }
            | IrChunk::Bind { reg, .. }
            | IrChunk::Call { reg, .. }
            | IrChunk::MemAlloc { reg, .. }
            | IrChunk::MemRead { reg, .. } => reg.get_r_reg_of().into_iter().collect(),

            // Parameters are defined on function entry
            IrChunk::Func { args, .. } => args.iter().filter_map(|r| r.get_r_reg_of()).collect(),

            IrChunk::Ret { .. }
            | IrChunk::Label { .. }
            | IrChunk::Jump { .. }
            | IrChunk::CondJump { .. }
            | IrChunk::MemWrite { .. } => Vec::new(),
        }
    }
}


/// Returns a list of successors for each IrChunk
fn cfg_successors(chunks: &[IrChunk<EnvEntry, Kind>]) -> Vec<Vec<usize>>
{
    // instruction indexe von den labels
    let labels = chunks
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            match c
            {
                IrChunk::Label { name } => Some((name.as_str(), i)),
                _ => None,
            }
        })
        .collect::<HashMap<&str, usize>>();

    let target = |name: &String| labels[name.as_str()];

    // index vom nöächsten ziel wenn es denn eins gibt...
    let next = |i: usize| (i + 1 < chunks.len()).then_some(i + 1);

    chunks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            match c
            {
                // Jump: immer direkt zum index vom jump ziel
                IrChunk::Jump { name } => vec![target(name)],

                // Conditional Jump: Könnte Springen wie jump, könnte aber auch einfach normal weiter laufen
                // also sowohl das sprungziel als auch die nächste instruction merken
                IrChunk::CondJump { name, .. } =>
                {
                    [Some(target(name)), next(i)]
                        .into_iter()
                        .flatten()
                        .collect()
                }

                // Bei return ist die funktion zu ende, also keinen nachfolger
                IrChunk::Ret { .. } => vec![],

                // Ansonsten machen alle anderen instructions immer mir der nächsten instruction weiter.
                _ => next(i).into_iter().collect(),
            }
        })
        .collect()
}
