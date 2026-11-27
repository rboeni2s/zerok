mod graph_setup;


use crate::{
    annotator::{EnvEntry, Kind},
    spill::{
        Color,
        Liveness,
        RegNum,
        arch::{Arch, ParamLoc, RegFile},
        cfg::CfgRegs,
        reg_helper::GetRegR,
    },
};
use ir::IrChunk;
use std::collections::HashMap;

pub use graph_setup::*;


/// Determines the register class of every register in `chunks` on architecture `A`.
///
/// The class depends on the architecture, not only on the kind of the value: on a soft-float
/// architecture floats live in the general purpose registers (see [`Arch::class_of`]).
pub fn register_classes<A: Arch>(chunks: &[IrChunk<EnvEntry, Kind>]) -> HashMap<RegNum, RegClass>
{
    let mut classes = HashMap::new();

    for (reg, kind) in chunks.iter().flat_map(|c| c.cfg_def_kinds())
    {
        let Some(class) = A::class_of(kind)
        else
        {
            continue;
        };

        let prev = classes.insert(reg, class);

        // Wird ein register einmal als int und einmal als float geschrieben ist das ein codegen bug
        debug_assert!(
            prev.is_none_or(|prev| prev == class),
            "register {reg} is written as {prev:?} and {class:?}"
        );
    }

    classes
}


/// Builds the interference graph of a single function from its liveness for architecture `A`.
///
/// Every register becomes a node of the type of its class on `A` ([`GprNode`] or [`FloatNode`]).
/// Edges only connect registers of the same class, registers of different classes never share a
/// machine register anyway. Registers without a class (see [`register_classes`]) get no node and
/// no edges.
pub fn build_interference_graph<A: Arch>(liveness: &Liveness) -> graph_setup::IntfGraph
{
    let chunks = liveness.chunks();
    let classes = register_classes::<A>(chunks);
    let mut graph = graph_setup::IntfGraph::default();

    let class_of = |reg: RegNum| classes.get(&reg).copied();

    // Fügt die kante (a, b) ein, aber nur zwischen registern derselben klasse.
    // Register ohne klasse brauchen kein register und bekommen keine kanten
    let add_edge = |graph: &IntfGraph, a: RegNum, b: RegNum, edge: IntfEdge| {
        if a != b && class_of(a).is_some() && class_of(a) == class_of(b)
        {
            graph
                .any_link(NodeId::Virt(a.min(b)), edge)
                .add(&NodeId::Virt(a.max(b)));
        }
    };

    // Jedem Register einen Knoten im Graph geben...
    for chunk in chunks
    {
        // Liest alle register die in den allen def und use mengen vorkommen und legt einen Knoten dafür an,
        // kommt ein Knoten häufiger als einmal vor dedupliziert der graph ihn automatisch
        // (wie ein HashSet oder eine HashMap das auch tun würde).
        for reg in chunk.cfg_def().into_iter().chain(chunk.cfg_use())
        {
            match class_of(reg)
            {
                Some(RegClass::Gpr) => graph.insert(GprNode::new(reg)),
                Some(RegClass::Float) => graph.insert(FloatNode::new(reg)),
                None => None,
            };
        }
    }

    for (n, chunk) in chunks.iter().enumerate()
    {
        let defs = chunk.cfg_def();

        // source register von bind instructions oder None
        let move_src = match chunk
        {
            IrChunk::Bind { val, .. } => val.get_r_reg_of(),
            _ => None,
        };

        // Move kanten für das coalescing merken...
        // Bei einem move darf das ziel mit der quelle ein register teilen
        if let Some(src) = move_src
        {
            for &d in &defs
            {
                add_edge(&graph, d, src, IntfEdge::Move);
            }
        }

        // Dinge die von instruction n geschrieben werden,
        // dürfen sich kein register mit etwas was nach n noch gelesen wird teilen.
        for &d in &defs
        {
            for &x in &liveness.live_out[n]
            {
                if Some(x) != move_src
                {
                    add_edge(&graph, d, x, IntfEdge::Interferes);
                }
            }
        }

        // Werden mehrere register gleichzeitig geschrieben (die parameter bei `Func` zb),
        // halten sie verschiedene werte und dürfen sich kein register teilen,
        // selbst wenn eines davon nie gelesen wird
        for (i, &a) in defs.iter().enumerate()
        {
            for &b in &defs[i + 1..]
            {
                add_edge(&graph, a, b, IntfEdge::Interferes);
            }
        }
    }

    graph
}


/// Precolors the graph.
///
/// Marker machine-registers are added to control the pre coloring:
/// * Some pre-colorings are treated as a preference or hint. They are connected to the
///   machine-register node of their respective coloring through a _move_ edge.
///
/// * Some other pre-colorings are negative constraints (not a hint) like caller saved registers.
///   Their nodes are connected using a _interference_ edge the node of a physical register
///   to prevent them from being colored in that color.
pub fn precolor<A: Arch>(graph: &mut IntfGraph, liveness: &Liveness)
{
    let chunks = liveness.chunks();
    let classes = register_classes::<A>(&chunks);

    /// inserts a node for every physical register in the regfile into the graph
    fn insert_phys<A: Arch, N: IntfNode>(graph: &mut IntfGraph)
    {
        for c in 0..<N::File<A> as RegFile>::COLORS
        {
            graph.insert(N::phys(Color(c)));
        }
    }

    // registersätze für float und gpr register einfügen
    insert_phys::<A, GprNode>(graph);
    insert_phys::<A, FloatNode>(graph); // Architekturen ohne zb separate float register machen hier nix, da N=0...

    // Now go over every instruction (chunk) and precolor its registers if needed
    for (n, chunk) in chunks.iter().enumerate()
    {
        match chunk
        {
            IrChunk::Func { args, .. } =>
            {
                // Get the (virtual) register numbers and register class of each function parameter
                let params: Vec<(Option<RegNum>, RegClass)> = args
                    .iter()
                    .filter_map(|arg| Some((arg.get_r_reg_of(), A::class_of(arg.kind)?)))
                    .collect();

                // Fetch the register and stack positions for each function parameters from the architecture...
                let param_locations = {
                    let classes = params.iter().map(|&(_, class)| class).collect::<Vec<_>>();
                    A::param_loc(&classes)
                };

                for ((reg, _), loc) in params.iter().zip(&param_locations)
                {
                    // Skip the pre-coloring for arguments passed on the stack
                    // Arguments not passed via the satck should have:
                    // * reg: the virtual register number
                    // * class: the register class of the virtual register (float, gpr, ...)
                    // * color: the index of the machine register that the calling convention wants this parameters to have
                    let (Some(reg), ParamLoc::Reg { class, color }) = (reg, loc)
                    else
                    {
                        continue;
                    };

                    // Add a move-edge between the virtual register and the physical register suggested by the architecture.
                    graph
                        .any_link(NodeId::Virt(*reg), IntfEdge::Move)
                        .add(&NodeId::Phys(*class, *color));
                }
            }

            IrChunk::Call { name, args, reg } =>
            {
                todo!();
            }

            IrChunk::Ret { val } =>
            {
                todo!();
            }

            // All other chunks do not have registers that would need precoloring
            _ => (),
        }
    }
}
