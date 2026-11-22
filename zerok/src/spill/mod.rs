mod cfg;
mod liveness;
mod reg_helper;


// Type aliase because the registers and instructions are both indexed as usize
// and seeing usize everywhere is confusing.
pub type RegNum = usize;
pub type CmdIdx = usize;


pub use liveness::Liveness;
