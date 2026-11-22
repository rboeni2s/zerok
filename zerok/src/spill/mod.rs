mod cfg;
mod intf_graph;
mod liveness;
mod reg_helper;


// Type aliase because the registers and instructions are both indexed as usize
// and seeing usize everywhere is confusing.
pub type RegNum = usize;
pub type CmdIdx = usize;


pub use intf_graph::{IntfEdge, IntfGraph, build_interference_graph, neighbours};
pub use liveness::Liveness;
