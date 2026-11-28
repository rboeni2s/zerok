use crate::spill::{
    FloatNode,
    GprNode,
    IntfEdge,
    IntfGraph,
    IntfNode,
    NodeColor,
    NodeId,
    arch::{Arch, RegFile},
    neighbours,
};
use std::collections::{HashMap, HashSet};


/// Colors a pre-colored graph respecting the the architecture `A`
pub fn color<A: Arch>(graph: &mut IntfGraph)
{
    color_class::<A, GprNode>(graph);
    color_class::<A, FloatNode>(graph);
}


// Colors a specific class of registers in the given Architecture
fn color_class<A: Arch, N: IntfNode>(graph: &mut IntfGraph)
{
    // k-coloring of the graph
    let k = <N::File<A> as RegFile>::COLORS; // Count of usable registers in this class

    // Get a list of all virtual and uncolored registers
    let virt = graph
        .variants::<N>()
        .filter(|n| n.color() == NodeColor::Uncolored)
        .map(|n| n.id())
        .collect::<Vec<_>>();

    // Dont color if there are no virtual uncolored registers
    if virt.is_empty()
    {
        return;
    }

    // Spill everything if there are no usable registers
    if k == 0
    {
        for id in &virt
        {
            if let Some(node) = graph.get_mut::<N>(id)
            {
                node.set_color(NodeColor::Spilled);
            }
        }

        return;
    }

    // Collect all adjacent nodes for each node per edge type
    #[rustfmt::skip]
    let intf_adj = virt
        .iter()
        .map(|&id| (id, neighbours::<N>(graph, id, IntfEdge::Interferes).collect()))
        .collect::<HashMap<NodeId, Vec<NodeId>>>();

    let move_adj = virt
        .iter()
        .map(|&id| (id, neighbours::<N>(graph, id, IntfEdge::Move).collect()))
        .collect::<HashMap<NodeId, Vec<NodeId>>>();

    // Cache the degree of all (intf-)nodes
    let mut degree = intf_adj
        .iter()
        .map(|(&id, ns)| (id, ns.len()))
        .collect::<HashMap<NodeId, usize>>();

    // Setup a set of all nodes that still need to be looked at
    // and a stack for all nodes that can be colored (degree < k)
    let mut remaining: HashSet<NodeId> = virt.iter().copied().collect();
    let mut stack = Vec::with_capacity(virt.len());

    // Find all nodes that are trivially colorable (degree < k), push them on the stack and decrease
    // the degrees of their neighbors by 1. If no such node exists decide on a node to spill and push that onto
    // the stack instead...
    if remaining.is_empty()
    {
        // Find a suitable candidate...
        let candidate = match remaining.iter().find(|id| degree[id] < k)
        {
            // Just return the trivially colorable node if it exists
            Some(id) => *id,

            // If no node is trivially colorable, select a node to be spilled.
            // Ideally the node with the highest degree is spilled because this will
            // free up a degree on the highest count of neighbors possible (using one node)
            None => *remaining.iter().max_by_key(|id| degree[id]).unwrap(),
        };

        // Now remove the candidate and decrease the degrees of its neighbors by one
        remaining.remove(&candidate);
        for neighbor in &intf_adj[&candidate]
        {
            if let Some(d) = degree.get_mut(neighbor)
            {
                *d -= 1;
            }
        }

        // Finally push the candidate on the stack
        stack.push(candidate);
    }
}
