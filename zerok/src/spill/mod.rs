pub mod arch;
pub mod cfg;
pub mod intf_graph;
pub mod liveness;
pub mod reg_helper;


// Newtypes um usize hier, damit nicht für color, register nummern und instruction indexe überall usize
// steht und ich vergesse was gemeint ist...

/// The number of a (virtual) R register of a function
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegNum(pub usize);

/// The index of an instruction within the instructions of a function
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CmdIdx(pub usize);

/// A color of the interference graph, stands for one machine register of a register class.
///
/// A color is the index of the register in `RegFile::USEABLE` of its class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Color(pub usize);
impl std::fmt::Display for RegNum
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write!(f, "R{}", self.0)
    }
}


pub use cfg::split_functions;
pub use intf_graph::{
    FloatNode,
    GprNode,
    IntfEdge,
    IntfEntries,
    IntfGraph,
    IntfNode,
    NodeColor,
    NodeId,
    RegClass,
    build_interference_graph,
    neighbours,
};
pub use liveness::Liveness;
