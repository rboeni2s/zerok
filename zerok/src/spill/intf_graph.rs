mod graph_setup;

use crate::{
    annotator::{EnvEntry, Kind},
    spill::{Liveness, RegNum, cfg::CfgRegs, reg_helper::GetRegR},
};
use ir::IrChunk;
use std::collections::HashMap;

pub use graph_setup::*;


/// Determines the register class of every register in `chunks`.
///
/// ## Registers without a class are not part of the map:
/// - registers of kind `none`, they carry no data
/// - registers that are read but never written (the codegen reads the never written value
///   register of `none` expressions, e.g. a block ending with `;` as the body of an `if`)
pub fn register_classes(chunks: &[IrChunk<EnvEntry, Kind>]) -> HashMap<RegNum, RegClass>
{
    let mut classes = HashMap::new();

    for (reg, kind) in chunks.iter().flat_map(|c| c.cfg_def_kinds())
    {
        let Some(class) = RegClass::of(kind)
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


/// Builds the interference graph of a single function from its liveness.
///
/// Every register becomes a node of the type of its class ([`IntNode`] or [`FloatNode`]). Edges
/// only connect registers of the same class, registers of different classes never share a machine
/// register anyway. Registers without a class (see [`register_classes`]) get no node and no edges.
pub fn build_interference_graph(liveness: &Liveness) -> graph_setup::IntfGraph
{
    let chunks = liveness.chunks();
    let classes = register_classes(chunks);
    let mut graph = graph_setup::IntfGraph::default();

    let class_of = |reg: RegNum| classes.get(&reg).copied();

    // Fügt die kante a – b ein, aber nur zwischen registern derselben klasse.
    // Register ohne klasse brauchen kein register und bekommen keine kanten
    let add_edge = |graph: &IntfGraph, a: RegNum, b: RegNum, edge: IntfEdge| {
        if a != b && class_of(a).is_some() && class_of(a) == class_of(b)
        {
            graph.any_link(a.min(b), edge).add(&a.max(b));
        }
    };

    // Jedem Register einen Knoten im Graph geben, auch wenn es mit nichts interferiert,
    // denn es braucht trotzdem eine farbe (register)
    for chunk in chunks
    {
        // Liest alle register die in den allen def und use mengen vorkommen und legt einen Knoten dafür an,
        // kommt ein Knoten häufiger als einmal vor dedupliziert der graph ihn automatisch
        // (wie ein HashSet oder eine HashMap das auch tun würde).
        for reg in chunk.cfg_def().into_iter().chain(chunk.cfg_use())
        {
            match class_of(reg)
            {
                Some(RegClass::Gpr) => graph.insert(IntNode::new(reg)),
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

        // Dinge die von instruction n geschrieben werden, dürfen sich kein register mit etwas was nach n noch gelesen wird teilen.
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
