use crate::spill::RegNum;
use cache_map::ident::IdentRef;
use cache_map::prelude::*;


#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum IntfEntries
{
    RegNode(RegNode),
}

/// A node of the interference graph: a single (virtual) register
#[derive(CacheEntry, Debug, Clone, PartialEq, Eq, Hash)]
#[cache_entry(IntfEntries)]
pub struct RegNode(pub RegNum);


impl IdentRef<RegNum> for RegNode
{
    fn ident_ref(&self) -> &RegNum
    {
        &self.0
    }
}


/// The kinds of edges in the interference graph
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum IntfEdge
{
    /// Both registers are live at the same time and must never share a machine register
    Interferes,

    /// The registers are connected by a move (`Bind`). If they get the same machine register,
    /// the move can be removed (coalescing). This is only a hint, not a constraint.
    Move,
}


/// The interference graph
pub type IntfGraph = Cache<RegNum, IntfEntries, IntfEdge>;


/// Returns all registers connected to `reg` by an edge of kind `edge`, regardless of the direction
/// the link is stored in. For `IntfEdge::Interferes` its count is the degree of `reg`.
pub fn neighbours(
    graph: &IntfGraph,
    reg: RegNum,
    edge: IntfEdge,
) -> impl Iterator<Item = RegNum> + '_
{
    graph
        .any_link(reg, edge)
        .blinks(graph)
        .filter_map(RegNode::unpack)
        .map(|node| node.0)
}
