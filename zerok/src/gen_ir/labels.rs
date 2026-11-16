//! All labels of the ir are generated here, so they share one naming scheme:
//!
//! * `zkl_{function}_{n}`:      a unique label inside of `function`
//! * `zkl_{function}_return`:   the return label of `function`
//! * `zkl_{loop}_{part}`:       the start/continue/done label of a loop
//!
//! `{function}` is the mangled path of the function, so labels of functions in different
//! modules can not collide. Every label starts with [`LABEL_PREFIX`], which no function has.

use super::GenEnv;
use crate::symbol::LABEL_PREFIX;
use std::sync::atomic::{AtomicUsize, Ordering};


/// The parts of a loop, that can be jumped to
pub(super) enum LoopPart
{
    Start,
    Continue,
    Done,
}


impl GenEnv<'_, '_>
{
    /// A new label, that is unique in the whole program
    pub(super) fn unique_label(&self) -> String
    {
        static LABEL_NUMBER: AtomicUsize = AtomicUsize::new(0);

        format!(
            "{LABEL_PREFIX}{}_{}",
            self.symbol.mangled_path(),
            LABEL_NUMBER.fetch_add(1, Ordering::Relaxed)
        )
    }

    /// The label at the end of the function, which every return jumps to
    pub(super) fn return_label(&self) -> String
    {
        format!("{LABEL_PREFIX}{}_return", self.symbol.mangled_path())
    }
}


/// The label of `part` of the loop with the unique label `loop_label` (from the annotator)
pub(super) fn loop_label(loop_label: &str, part: LoopPart) -> String
{
    let part = match part
    {
        LoopPart::Start => "start",
        LoopPart::Continue => "continue",
        LoopPart::Done => "done",
    };

    format!("{LABEL_PREFIX}{loop_label}_{part}")
}
