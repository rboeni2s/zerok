//! Tests of the register allocation: liveness and interference graph

use ir::{IrChunk, IrOp, IrReg, IrTypedReg};
use std::collections::{BTreeSet, HashMap};
use zerok::annotator::{EnvEntry, Kind};
use zerok::parser::atom::Atom;
use zerok::spill::cfg::{CfgRegs, cfg_successors, split_functions};
use zerok::spill::intf_graph::register_classes;
use zerok::spill::{
    FloatNode,
    IntNode,
    IntfEdge,
    IntfEntries,
    IntfGraph,
    IntfNode,
    Liveness,
    NodeColor,
    RegClass,
    RegNum,
    build_interference_graph,
    neighbours,
};


fn r(reg: RegNum) -> EnvEntry
{
    EnvEntry::Register(reg)
}

fn tr(reg: RegNum) -> IrTypedReg<Kind>
{
    trk(reg, Kind::I64)
}

fn trk(reg: RegNum, kind: Kind) -> IrTypedReg<Kind>
{
    IrTypedReg {
        reg: IrReg::R(reg),
        kind,
    }
}

fn set(regs: &[RegNum]) -> BTreeSet<RegNum>
{
    regs.iter().copied().collect()
}

/// f(R1): R2 = 0; while R2 < R1 { R2 = R2 + 1 }; return R2
fn loop_chunks() -> Vec<IrChunk<EnvEntry, Kind>>
{
    vec![
        /* 0 */
        IrChunk::Func {
            name: "f".into(),
            args: vec![tr(1)],
        },
        /* 1 */
        IrChunk::Bind {
            val: EnvEntry::Atom(Atom::Int(0)),
            reg: tr(2),
        },
        /* 2 */ IrChunk::Label {
            name: "L_loop".into(),
        },
        /* 3 */
        IrChunk::Bin {
            op: IrOp::Lt,
            lhs: r(2),
            rhs: r(1),
            reg: tr(3),
        },
        /* 4 */
        IrChunk::Una {
            op: IrOp::Not,
            arg: r(3),
            reg: tr(4),
        },
        /* 5 */
        IrChunk::CondJump {
            val: r(4),
            name: "L_end".into(),
        },
        /* 6 */
        IrChunk::Bin {
            op: IrOp::Add,
            lhs: r(2),
            rhs: EnvEntry::Atom(Atom::Int(1)),
            reg: tr(2),
        },
        /* 7 */ IrChunk::Jump {
            name: "L_loop".into(),
        },
        /* 8 */ IrChunk::Label {
            name: "L_end".into(),
        },
        /* 9 */ IrChunk::Ret { val: r(2) },
    ]
}

fn interferes(graph: &IntfGraph, reg: RegNum) -> BTreeSet<RegNum>
{
    neighbours::<IntNode>(graph, reg, IntfEdge::Interferes).collect()
}

fn move_partners(graph: &IntfGraph, reg: RegNum) -> BTreeSet<RegNum>
{
    neighbours::<IntNode>(graph, reg, IntfEdge::Move).collect()
}

fn nodes<N: IntfNode>(graph: &IntfGraph) -> BTreeSet<RegNum>
{
    graph.variants::<N>().map(|n| *n.ident_ref()).collect()
}

fn graph_of(chunks: &[IrChunk<EnvEntry, Kind>]) -> IntfGraph
{
    build_interference_graph(&Liveness::new(chunks))
}


#[test]
fn cfg_use_def()
{
    let bin = IrChunk::Bin {
        op: IrOp::Add,
        lhs: r(1),
        rhs: EnvEntry::Atom(Atom::Int(1)),
        reg: tr(2),
    };
    assert_eq!(bin.cfg_use(), vec![1]);
    assert_eq!(bin.cfg_def(), vec![2]);
    assert_eq!(bin.cfg_def_kinds(), vec![(2, Kind::I64)]);

    // `base` ist ein `IrTypedReg`, wird aber nur gelesen
    let write = IrChunk::MemWrite {
        value: r(1),
        base: tr(2),
        offset: r(3),
    };
    let mut uses = write.cfg_use();
    uses.sort();
    assert_eq!(uses, vec![1, 2, 3]);
    assert!(write.cfg_def().is_empty());

    // Parameter werden am funktionseingang geschrieben
    let func = IrChunk::<EnvEntry, Kind>::Func {
        name: "f".into(),
        args: vec![tr(1), trk(2, Kind::F64)],
    };
    assert!(func.cfg_use().is_empty());
    assert_eq!(func.cfg_def_kinds(), vec![(1, Kind::I64), (2, Kind::F64)]);
}


#[test]
fn cfg_successors_loop()
{
    assert_eq!(
        cfg_successors(&loop_chunks()),
        vec![
            vec![1],
            vec![2],
            vec![3],
            vec![4],
            vec![5],
            vec![8, 6],
            vec![7],
            vec![2],
            vec![9],
            vec![]
        ]
    );
}


#[test]
fn liveness_loop()
{
    let chunks = loop_chunks();
    let live = Liveness::new(&chunks);

    let expected_in = [
        set(&[]),
        set(&[1]),
        set(&[1, 2]),
        set(&[1, 2]),
        set(&[1, 2, 3]),
        set(&[1, 2, 4]),
        set(&[1, 2]),
        set(&[1, 2]),
        set(&[2]),
        set(&[2]),
    ];

    let expected_out = [
        set(&[1]),
        set(&[1, 2]),
        set(&[1, 2]),
        set(&[1, 2, 3]),
        set(&[1, 2, 4]),
        set(&[1, 2]),
        set(&[1, 2]),
        set(&[1, 2]),
        set(&[2]),
        set(&[]),
    ];

    assert_eq!(live.live_in, expected_in);
    assert_eq!(live.live_out, expected_out);
}


#[test]
fn register_classes_by_kind()
{
    let chunks = vec![
        IrChunk::Func {
            name: "f".into(),
            args: vec![trk(1, Kind::I64), trk(2, Kind::F32)],
        },
        IrChunk::Bin {
            op: IrOp::Lt,
            lhs: r(1),
            rhs: r(1),
            reg: trk(3, Kind::Bool),
        },
        IrChunk::Bin {
            op: IrOp::Mul,
            lhs: r(2),
            rhs: r(2),
            reg: trk(4, Kind::F64),
        },
        IrChunk::Ret { val: r(3) },
    ];

    assert_eq!(
        register_classes(&chunks),
        HashMap::from([
            (1, RegClass::Gpr),
            (2, RegClass::Float),
            (3, RegClass::Gpr),
            (4, RegClass::Float),
        ])
    );
}


#[test]
fn interference_loop()
{
    let chunks = loop_chunks();

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


/// f(R1: i64, R2: f64): R3 = R1 + R1; R4 = R2 * R2; return g(R3, R4, R2)
/// -- int und float register leben gleichzeitig, interferieren aber nie miteinander
#[test]
fn interference_classes()
{
    let chunks = vec![
        IrChunk::Func {
            name: "f".into(),
            args: vec![trk(1, Kind::I64), trk(2, Kind::F64)],
        },
        IrChunk::Bin {
            op: IrOp::Add,
            lhs: r(1),
            rhs: r(1),
            reg: trk(3, Kind::I64),
        },
        IrChunk::Bin {
            op: IrOp::Mul,
            lhs: r(2),
            rhs: r(2),
            reg: trk(4, Kind::F64),
        },
        IrChunk::Call {
            name: "g".into(),
            args: vec![r(3), r(4), r(2)],
            reg: trk(5, Kind::I64),
        },
        IrChunk::Ret { val: r(5) },
    ];

    let graph = graph_of(&chunks);

    assert_eq!(nodes::<IntNode>(&graph), BTreeSet::from([1, 3, 5]));
    assert_eq!(nodes::<FloatNode>(&graph), BTreeSet::from([2, 4]));

    // Die float register R2 und R4 leben gleichzeitig und interferieren
    let float_neighbours =
        |reg| neighbours::<FloatNode>(&graph, reg, IntfEdge::Interferes).collect::<BTreeSet<_>>();
    assert_eq!(float_neighbours(2), BTreeSet::from([4]));
    assert_eq!(float_neighbours(4), BTreeSet::from([2]));

    // Zwischen den klassen gibt es keine kanten, obwohl z.b. R2 und R3 gleichzeitig leben
    for reg in 1..=5
    {
        assert!(
            graph
                .any_link(reg, IntfEdge::Interferes)
                .blinks(&graph)
                .all(|n| {
                    matches!(
                        (graph.get_entry(reg).unwrap(), n),
                        (IntfEntries::IntNode(_), IntfEntries::IntNode(_))
                            | (IntfEntries::FloatNode(_), IntfEntries::FloatNode(_))
                    )
                })
        );
    }
}


/// Nach dem aufbau ist jeder knoten ungefärbt, die farbe lässt sich am knoten setzen
#[test]
fn interference_node_color()
{
    let chunks = vec![
        IrChunk::Func {
            name: "f".into(),
            args: vec![trk(1, Kind::I64), trk(2, Kind::F64)],
        },
        IrChunk::Ret { val: r(1) },
    ];

    let mut graph = graph_of(&chunks);

    assert_eq!(
        graph.get::<IntNode>(1).unwrap().color(),
        NodeColor::Uncolored
    );
    assert_eq!(
        graph.get::<FloatNode>(2).unwrap().color(),
        NodeColor::Uncolored
    );

    graph
        .get_mut::<IntNode>(1)
        .unwrap()
        .set_color(NodeColor::Colored(3));
    graph
        .get_mut::<FloatNode>(2)
        .unwrap()
        .set_color(NodeColor::Precolored(0));

    assert_eq!(graph.get::<IntNode>(1).unwrap().color().color(), Some(3));
    assert_eq!(graph.get::<FloatNode>(2).unwrap().color().color(), Some(0));

    // Der knotentyp muss zur klasse passen
    assert!(graph.get::<FloatNode>(1).is_none());
}


#[test]
fn split_functions_at_func()
{
    let chunks = vec![
        IrChunk::Func {
            name: "f".into(),
            args: vec![tr(256)],
        },
        IrChunk::Ret { val: r(256) },
        IrChunk::Func {
            name: "g".into(),
            args: vec![trk(256, Kind::F64)],
        },
        IrChunk::Bin {
            op: IrOp::Mul,
            lhs: r(256),
            rhs: r(256),
            reg: trk(257, Kind::F64),
        },
        IrChunk::Ret { val: r(257) },
    ];

    let funcs: Vec<_> = split_functions(&chunks).collect();

    assert_eq!(funcs.len(), 2);
    assert_eq!(funcs[0], &chunks[0..2]);
    assert_eq!(funcs[1], &chunks[2..5]);

    // R256 ist in f ein int, in g ein float: pro funktion analysiert gibt es keinen konflikt
    let f = build_interference_graph(&Liveness::new(funcs[0]));
    let g = build_interference_graph(&Liveness::new(funcs[1]));
    assert_eq!(nodes::<IntNode>(&f), BTreeSet::from([256]));
    assert_eq!(nodes::<FloatNode>(&g), BTreeSet::from([256, 257]));
}


/// Wie `wenn i > 0 { pos = pos + 2; }` im qsort beispiel: der körper hat den wert none,
/// sein wertregister R3 wird nie geschrieben, aber in das ergebnisregister R4 gebunden
#[test]
fn interference_ignores_none_registers()
{
    let chunks = vec![
        IrChunk::Func {
            name: "f".into(),
            args: vec![tr(1)],
        },
        IrChunk::Bin {
            op: IrOp::Add,
            lhs: r(1),
            rhs: EnvEntry::Atom(Atom::Int(2)),
            reg: tr(2),
        },
        IrChunk::Bind {
            val: r(3),
            reg: trk(4, Kind::None),
        },
        IrChunk::Ret { val: r(2) },
    ];

    let classes = register_classes(&chunks);
    assert_eq!(classes.get(&3), None);
    assert_eq!(classes.get(&4), None);

    let graph = graph_of(&chunks);

    // Nur R1 und R2 brauchen ein register
    assert_eq!(nodes::<IntNode>(&graph), BTreeSet::from([1, 2]));
    assert!(graph.get_entry(3).is_none());
    assert!(graph.get_entry(4).is_none());
}
