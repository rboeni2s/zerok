use crate::spill::{
    Color,
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
///
/// Returns a vector containing the ids of all spilled nodes
pub fn color<A: Arch>(graph: &mut IntfGraph) -> Vec<NodeId>
{
    let mut spilled = vec![];

    color_class::<A, GprNode>(graph, &mut spilled);
    color_class::<A, FloatNode>(graph, &mut spilled);

    spilled
}


// Colors a specific class of registers in the given Architecture
fn color_class<A: Arch, N: IntfNode>(graph: &mut IntfGraph, spilled: &mut Vec<NodeId>)
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
                spilled.push(*id);
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
    while !remaining.is_empty()
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

    // Now color the graph based on the stack contents
    while let Some(id) = stack.pop()
    {
        // Collect the colors of the neighbors into a hash set
        let forbidden: HashSet<Color> = intf_adj[&id]
            .iter()
            .filter_map(|&n| graph.get::<N>(n).and_then(|n| n.color().color()))
            .collect();

        // Prefer the color hints given by move-edges (calling convention, coalescing light)
        let hinted = move_adj[&id]
            .iter()
            // get an iterator over every hinted color
            .filter_map(|&n| graph.get::<N>(n).and_then(|n| n.color().color()))
            // now chose a color out of these hints that is not already chosen by one of the nodes neighbors
            // if there is no such color, then the hint is invalid and can not be respected when selecting a color.
            .find(|c| !forbidden.contains(c));

        // Now chose a color based on these choices (in order of preference):
        // 1. Use the hinted color if a valid hinted color was found.
        // 2. Use the lowest color that is not already used by a neighbor.
        let color = match hinted.or_else(|| (0..k).map(Color).find(|c| !forbidden.contains(c)))
        {
            // Great: A color was selected! Return it!
            Some(c) => NodeColor::Colored(c),

            // No color could be selected, this node needs to be spilled.
            None =>
            {
                spilled.push(id);
                NodeColor::Spilled
            }
        };

        // Set the nodes color on the graph
        if let Some(node) = graph.get_mut::<N>(id)
        {
            node.set_color(color);
        }
    }
}
