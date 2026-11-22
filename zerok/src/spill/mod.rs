pub mod cfg;
pub mod intf_graph;
pub mod liveness;
pub mod reg_helper;


// Type aliase because the registers and instructions are both indexed as usize
// and seeing usize everywhere is confusing.
pub type RegNum = usize;
pub type CmdIdx = usize;

/// A color of the interference graph, stands for one machine register of a register class
pub type Color = usize;


pub use intf_graph::{
    FloatNode,
    IntNode,
    IntfEdge,
    IntfEntries,
    IntfGraph,
    IntfNode,
    NodeColor,
    RegClass,
    build_interference_graph,
    neighbours,
};
pub use cfg::split_functions;
pub use liveness::Liveness;
