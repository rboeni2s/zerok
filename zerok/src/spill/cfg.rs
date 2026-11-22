use crate::{
    annotator::{EnvEntry, Kind},
    spill::{
        CmdIdx,
        RegNum,
        reg_helper::{GetRegR, operands},
    },
};
use ir::{IrChunk, IrTypedReg};
use std::collections::HashMap;


/// Register used by a IR-instruction
pub trait CfgRegs
{
    /// Returns a vector of all local registers being read by this instruction
    fn cfg_use(&self) -> Vec<RegNum>;

    /// Returns a vector of all local registers being written to by this instruction
    fn cfg_def(&self) -> Vec<RegNum>;

    /// Like [`Self::cfg_def`], together with the kind of the value written to each register
    fn cfg_def_kinds(&self) -> Vec<(RegNum, Kind)>;
}


impl CfgRegs for IrChunk<EnvEntry, Kind>
{
    /// Returns a Vec of all R register numbers read by this instruction
    fn cfg_use(&self) -> Vec<RegNum>
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

            // nix lesen:
            IrChunk::Func { .. } | IrChunk::Label { .. } | IrChunk::Jump { .. } => Vec::new(),
        }
    }

    /// Returns a Vec of all R Register numbers written by this instruction
    fn cfg_def(&self) -> Vec<RegNum>
    {
        self.cfg_def_kinds()
            .into_iter()
            .map(|(reg, _)| reg)
            .collect()
    }

    /// Returns a Vec of all R Register numbers written by this instruction and their kinds
    fn cfg_def_kinds(&self) -> Vec<(RegNum, Kind)>
    {
        // Geschriebene register sind immer `IrTypedReg`s, kennen also ihren typ
        let regs: &[IrTypedReg<Kind>] = match self
        {
            IrChunk::Func { args, .. } => args,

            IrChunk::Bin { reg, .. }
            | IrChunk::Una { reg, .. }
            | IrChunk::Bind { reg, .. }
            | IrChunk::Call { reg, .. }
            | IrChunk::MemAlloc { reg, .. }
            | IrChunk::MemRead { reg, .. } => std::slice::from_ref(reg),

            IrChunk::Ret { .. }
            | IrChunk::Label { .. }
            | IrChunk::Jump { .. }
            | IrChunk::CondJump { .. }
            | IrChunk::MemWrite { .. } => &[],
        };

        regs.iter()
            .filter_map(|r| r.get_r_reg_of().map(|reg| (reg, r.kind)))
            .collect()
    }
}


/// Returns a list of successors for each IrChunk
pub fn cfg_successors(chunks: &[IrChunk<EnvEntry, Kind>]) -> Vec<Vec<CmdIdx>>
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
        .collect::<HashMap<&str, CmdIdx>>();

    let target = |name: &String| labels[name.as_str()];

    // index vom nöächsten ziel wenn es denn eins gibt...
    let next = |i: CmdIdx| (i + 1 < chunks.len()).then_some(i + 1);

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
