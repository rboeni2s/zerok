mod graph_setup;

use crate::spill::{Liveness, cfg::CfgRegs, reg_helper::GetRegR};
use ir::IrChunk;

pub use graph_setup::*;


/// Builds the interference graph of a single function from its liveness.
pub fn build_interference_graph(liveness: &Liveness) -> graph_setup::IntfGraph
{
    let chunks = liveness.chunks();
    let mut graph = graph_setup::IntfGraph::default();

    // Jedem Register einen Knoten im Graph geben, auch wenn es mit nichts interferiert,
    // denn es braucht trotzdem eine farbe (register)
    for chunk in chunks
    {
        // Liest alle register die in den allen def und use mengen vorkommen und legt einen Knoten dafür an,
        // kommt ein Knoten häufiger als einmal vor dedupliziert der graph ihn automatisch
        // (wie ein HashSet oder eine HashMap das auch tun würde).
        for reg in chunk.cfg_def().into_iter().chain(chunk.cfg_use())
        {
            graph.insert(graph_setup::RegNode(reg));
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
                // nur kanten zwischen 2 verschiedenen registern einfügen:
                if d != src
                {
                    graph.any_link(d.min(src), IntfEdge::Move).add(&d.max(src));
                }
            }
        }

        // Dinge die von instruction n geschrieben werden, dürfen sich kein register mit etwas was nach n noch gelesen wird teilen.
        for &d in &defs
        {
            for &x in &liveness.live_out[n]
            {
                if x != d && Some(x) != move_src
                {
                    graph
                        .any_link(d.min(x), IntfEdge::Interferes)
                        .add(&d.max(x));
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
                if a != b
                {
                    graph
                        .any_link(a.min(b), IntfEdge::Interferes)
                        .add(&a.max(b));
                }
            }
        }
    }

    graph
}


#[cfg(test)]
mod tests
{
    use super::*;
    use crate::{
        annotator::{EnvEntry, Kind},
        parser::atom::Atom,
        spill::RegNum,
    };
    use ir::{IrOp, IrReg, IrTypedReg};
    use std::collections::BTreeSet;


    fn r(reg: RegNum) -> EnvEntry
    {
        EnvEntry::Register(reg)
    }

    fn tr(reg: RegNum) -> IrTypedReg<Kind>
    {
        IrTypedReg {
            reg: IrReg::R(reg),
            kind: Kind::I64,
        }
    }

    fn interferes(graph: &graph_setup::IntfGraph, reg: RegNum) -> BTreeSet<RegNum>
    {
        graph_setup::neighbours(graph, reg, graph_setup::IntfEdge::Interferes).collect()
    }

    fn move_partners(graph: &graph_setup::IntfGraph, reg: RegNum) -> BTreeSet<RegNum>
    {
        graph_setup::neighbours(graph, reg, graph_setup::IntfEdge::Move).collect()
    }

    fn graph_of(chunks: &[IrChunk<EnvEntry, Kind>]) -> graph_setup::IntfGraph
    {
        build_interference_graph(&Liveness::new(chunks))
    }


    /// f(R1): R2 = 0; while R2 < R1 { R2 = R2 + 1 }; return R2
    #[test]
    fn interference_loop()
    {
        let chunks = vec![
            IrChunk::Func {
                name: "f".into(),
                args: vec![tr(1)],
            },
            IrChunk::Bind {
                val: EnvEntry::Atom(Atom::Int(0)),
                reg: tr(2),
            },
            IrChunk::Label {
                name: "L_loop".into(),
            },
            IrChunk::Bin {
                op: IrOp::Lt,
                lhs: r(2),
                rhs: r(1),
                reg: tr(3),
            },
            IrChunk::Una {
                op: IrOp::Not,
                arg: r(3),
                reg: tr(4),
            },
            IrChunk::CondJump {
                val: r(4),
                name: "L_end".into(),
            },
            IrChunk::Bin {
                op: IrOp::Add,
                lhs: r(2),
                rhs: EnvEntry::Atom(Atom::Int(1)),
                reg: tr(2),
            },
            IrChunk::Jump {
                name: "L_loop".into(),
            },
            IrChunk::Label {
                name: "L_end".into(),
            },
            IrChunk::Ret { val: r(2) },
        ];

        let graph = graph_of(&chunks);

        assert_eq!(graph.iter().count(), 4);
        assert_eq!(interferes(&graph, 1), BTreeSet::from([2, 3, 4]));
        assert_eq!(interferes(&graph, 2), BTreeSet::from([1, 3, 4]));
        // R3 und R4 leben nie gleichzeitig und dürfen sich ein register teilen
        assert_eq!(interferes(&graph, 3), BTreeSet::from([1, 2]));
        assert_eq!(interferes(&graph, 4), BTreeSet::from([1, 2]));

        // `R2 = 0` bindet ein literal, das ist kein move zwischen registern
        assert!((1..=4).all(|reg| move_partners(&graph, reg).is_empty()));
    }


    /// f(R1): R2 = R1; return R1 + R2
    #[test]
    fn interference_move_exception()
    {
        let chunks = vec![
            IrChunk::Func {
                name: "f".into(),
                args: vec![tr(1)],
            },
            IrChunk::Bind {
                val: r(1),
                reg: tr(2),
            },
            IrChunk::Bin {
                op: IrOp::Add,
                lhs: r(1),
                rhs: r(2),
                reg: tr(3),
            },
            IrChunk::Ret { val: r(3) },
        ];

        let graph = graph_of(&chunks);

        // R1 und R2 sind gleichzeitig live, aber durch den move ohne interferenz-kante
        assert!(interferes(&graph, 1).is_empty());
        assert!(interferes(&graph, 2).is_empty());
        assert!(interferes(&graph, 3).is_empty());

        // Stattdessen sind sie über eine move-kante verbunden
        assert_eq!(move_partners(&graph, 1), BTreeSet::from([2]));
        assert_eq!(move_partners(&graph, 2), BTreeSet::from([1]));
        assert!(move_partners(&graph, 3).is_empty());
    }


    /// f(R1): R2 = R1; R1 = R1 + 1; return R1 + R2
    /// -- R1 wird überschrieben, während die kopie R2 noch lebt
    #[test]
    fn interference_constrained_move()
    {
        let chunks = vec![
            IrChunk::Func {
                name: "f".into(),
                args: vec![tr(1)],
            },
            IrChunk::Bind {
                val: r(1),
                reg: tr(2),
            },
            IrChunk::Bin {
                op: IrOp::Add,
                lhs: r(1),
                rhs: EnvEntry::Atom(Atom::Int(1)),
                reg: tr(1),
            },
            IrChunk::Bin {
                op: IrOp::Add,
                lhs: r(1),
                rhs: r(2),
                reg: tr(3),
            },
            IrChunk::Ret { val: r(3) },
        ];

        let graph = graph_of(&chunks);

        // Beide kanten: der move bleibt bekannt, darf aber nicht verschmolzen werden
        assert_eq!(interferes(&graph, 1), BTreeSet::from([2]));
        assert_eq!(move_partners(&graph, 1), BTreeSet::from([2]));
    }


    /// f(R1, R2): return 0 -- unbenutzte parameter interferieren trotzdem
    #[test]
    fn interference_params()
    {
        let chunks = vec![
            IrChunk::Func {
                name: "f".into(),
                args: vec![tr(1), tr(2)],
            },
            IrChunk::Ret {
                val: EnvEntry::Atom(Atom::Int(0)),
            },
        ];

        let graph = graph_of(&chunks);

        assert_eq!(interferes(&graph, 1), BTreeSet::from([2]));
        assert_eq!(interferes(&graph, 2), BTreeSet::from([1]));
    }
}
