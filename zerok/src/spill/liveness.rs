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
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Liveness
{
    pub live_in: Vec<BTreeSet<RegNum>>,
    pub live_out: Vec<BTreeSet<RegNum>>,
}


impl Liveness
{
    /// Computes the liveness of all registers of `chunks`
    pub fn new(chunks: &[IrChunk<EnvEntry, Kind>]) -> Self
    {
        let succ = cfg_successors(chunks);

        let uses: Vec<BTreeSet<RegNum>> = chunks
            .iter()
            .map(|c| c.cfg_use().into_iter().collect())
            .collect();

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
                // Nach n ist live, was irgendein nachfolger braucht
                let out: BTreeSet<RegNum> = succ[n]
                    .iter()
                    .flat_map(|&s| live_in[s].iter().copied())
                    .collect();

                // Vor n ist live, was n liest, plus was danach live ist und nicht von n überschrieben wird
                let inn: BTreeSet<RegNum> = uses[n]
                    .iter()
                    .copied()
                    .chain(out.difference(&defs[n]).copied())
                    .collect();

                // `out` hängt nur von den `in`-mengen ab, es reicht also deren änderungen zu prüfen
                if inn != live_in[n]
                {
                    live_in[n] = inn;
                    changed = true;
                }

                live_out[n] = out;
            }
        }

        Self { live_in, live_out }
    }
}
