use std::collections::BTreeSet;

use ir::IrChunk;

use crate::{
    annotator::{EnvEntry, Kind},
    spill::{
        RegNum,
        cfg::{CfgRegs, cfg_successors},
    },
};

/// Result of the liveness analysis:
///
/// `Self::live_in[n]` holds a vector if registers that are live directly before the instruction`n`
/// `Self::live_out[n]` holds a vector if registers that are live directly after the instruction `n`
///
/// The analysed instructions are kept as a reference, so later passes (like building the
/// interference graph) always work on the same instructions the liveness was computed for.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Liveness<'a>
{
    chunks: &'a [IrChunk<EnvEntry, Kind>],
    pub live_in: Vec<BTreeSet<RegNum>>,
    pub live_out: Vec<BTreeSet<RegNum>>,
}


impl<'a> Liveness<'a>
{
    /// The instructions this liveness was computed for, indexed like `live_in` and `live_out`
    pub fn chunks(&self) -> &'a [IrChunk<EnvEntry, Kind>]
    {
        self.chunks
    }

    /// Computes the liveness of all registers of `chunks`
    pub fn new(chunks: &'a [IrChunk<EnvEntry, Kind>]) -> Self
    {
        // Listet alle nachfolger einer instruction auf
        let succ = cfg_successors(chunks);

        // Listet alle gelesenen register pro instruction auf
        let uses: Vec<BTreeSet<RegNum>> = chunks
            .iter()
            .map(|c| c.cfg_use().into_iter().collect())
            .collect();

        // Listet alle geschriebenen register pro instruction auf
        let defs: Vec<BTreeSet<RegNum>> = chunks
            .iter()
            .map(|c| c.cfg_def().into_iter().collect())
            .collect();

        let mut live_in = vec![BTreeSet::new(); chunks.len()];
        let mut live_out = vec![BTreeSet::new(); chunks.len()];

        let mut changed = true;
        while changed
        {
            changed = false;

            for n in (0..chunks.len()).rev()
            {
                // Nach n ist alles live was irgendeiner von n's nachfolgern braucht
                // "out"[n] = union.big_(s in "succ"(n)) "in"[s]
                let out_n: BTreeSet<RegNum> = succ[n]
                    .iter()
                    .flat_map(|&s| live_in[s].iter().copied())
                    .collect();

                // Vor n ist live, was n liest, UND was danach live ist und nicht von n überschrieben wird
                // "in"[n] = "use"[n] union ("out"[n] without "def"[n])
                let in_n: BTreeSet<RegNum> = uses[n]
                    .iter()
                    .copied()
                    .chain(out_n.difference(&defs[n]).copied())
                    .collect();

                // nur die in-menge prüfen, da `out` nur von den in-mengen abhängt (hoffe ich)
                if in_n != live_in[n]
                {
                    live_in[n] = in_n;
                    changed = true;
                }

                live_out[n] = out_n;
            }
        }

        Self {
            chunks,
            live_in,
            live_out,
        }
    }
}
