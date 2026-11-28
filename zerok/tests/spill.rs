//! Tests of the register allocation: liveness and interference graph

use ir::{IrChunk, IrOp, IrReg, IrTypedReg};
use std::collections::{BTreeSet, HashMap};
use zerok::annotator::{EnvEntry, Kind};
use zerok::parser::atom::Atom;
use zerok::spill::cfg::{CfgRegs, cfg_successors, split_functions};
use zerok::spill::arch::target::{SysV64, SysV64Gpr, SysV64Xmm};
use zerok::spill::arch::{Arch, EmptyFile, ParamLoc, RegFile};
use zerok::spill::intf_graph::{precolor, register_classes};
use zerok::spill::{
    CmdIdx,
    Color,
    FloatNode,
    GprNode,
    IntfEdge,
    IntfEntries,
    IntfGraph,
    IntfNode,
    Liveness,
    NodeColor,
    NodeId,
    RegClass,
    RegNum,
    build_interference_graph,
    neighbours,
};


fn r(reg: usize) -> EnvEntry
{
    EnvEntry::Register(reg)
}

fn tr(reg: usize) -> IrTypedReg<Kind>
{
    trk(reg, Kind::I64)
}

fn trk(reg: usize, kind: Kind) -> IrTypedReg<Kind>
{
    IrTypedReg {
        reg: IrReg::R(reg),
        kind,
    }
}

fn set(regs: &[usize]) -> BTreeSet<RegNum>
{
    regs.iter().copied().map(RegNum).collect()
}

fn regs(regs: &[usize]) -> Vec<RegNum>
{
    regs.iter().copied().map(RegNum).collect()
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

/// Der schlüssel des virtuellen registers `reg`
fn v(reg: usize) -> NodeId
{
    NodeId::Virt(RegNum(reg))
}

/// Die virtuellen register von `ids`, bricht ab, wenn ein maschinenregister dabei ist
fn virt(ids: impl IntoIterator<Item = NodeId>) -> BTreeSet<RegNum>
{
    ids.into_iter()
        .map(|id| id.virt().expect("only virtual registers expected"))
        .collect()
}

fn interferes(graph: &IntfGraph, reg: usize) -> BTreeSet<RegNum>
{
    virt(neighbours::<GprNode>(graph, v(reg), IntfEdge::Interferes))
}

fn move_partners(graph: &IntfGraph, reg: usize) -> BTreeSet<RegNum>
{
    virt(neighbours::<GprNode>(graph, v(reg), IntfEdge::Move))
}

fn nodes<N: IntfNode>(graph: &IntfGraph) -> BTreeSet<RegNum>
{
    virt(graph.variants::<N>().map(|n| n.id()))
}

fn graph_of(chunks: &[IrChunk<EnvEntry, Kind>]) -> IntfGraph
{
    build_interference_graph::<SysV64>(&Liveness::new(chunks))
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
    assert_eq!(bin.cfg_use(), regs(&[1]));
    assert_eq!(bin.cfg_def(), regs(&[2]));
    assert_eq!(bin.cfg_def_kinds(), vec![(RegNum(2), Kind::I64)]);

    // `base` ist ein `IrTypedReg`, wird aber nur gelesen
    let write = IrChunk::MemWrite {
        value: r(1),
        base: tr(2),
        offset: r(3),
    };
    let mut uses = write.cfg_use();
    uses.sort();
    assert_eq!(uses, regs(&[1, 2, 3]));
    assert!(write.cfg_def().is_empty());

    // Parameter werden am funktionseingang geschrieben
    let func = IrChunk::<EnvEntry, Kind>::Func {
        name: "f".into(),
        args: vec![tr(1), trk(2, Kind::F64)],
    };
    assert!(func.cfg_use().is_empty());
    assert_eq!(
        func.cfg_def_kinds(),
        vec![(RegNum(1), Kind::I64), (RegNum(2), Kind::F64)]
    );
}


#[test]
fn cfg_successors_loop()
{
    assert_eq!(
        cfg_successors(&loop_chunks()),
        [
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
        .map(|succ| succ.into_iter().map(CmdIdx).collect::<Vec<_>>())
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
        register_classes::<SysV64>(&chunks),
        HashMap::from([
            (RegNum(1), RegClass::Gpr),
            (RegNum(2), RegClass::Float),
            (RegNum(3), RegClass::Gpr),
            (RegNum(4), RegClass::Float),
        ])
    );
}


#[test]
fn interference_loop()
{
    let chunks = loop_chunks();

    let graph = graph_of(&chunks);

    assert_eq!(graph.iter().count(), 4);
    assert_eq!(interferes(&graph, 1), set(&[2, 3, 4]));
    assert_eq!(interferes(&graph, 2), set(&[1, 3, 4]));
    // R3 und R4 leben nie gleichzeitig und dürfen sich ein register teilen
    assert_eq!(interferes(&graph, 3), set(&[1, 2]));
    assert_eq!(interferes(&graph, 4), set(&[1, 2]));

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
    assert_eq!(move_partners(&graph, 1), set(&[2]));
    assert_eq!(move_partners(&graph, 2), set(&[1]));
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
    assert_eq!(interferes(&graph, 1), set(&[2]));
    assert_eq!(move_partners(&graph, 1), set(&[2]));
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

    assert_eq!(interferes(&graph, 1), set(&[2]));
    assert_eq!(interferes(&graph, 2), set(&[1]));
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

    assert_eq!(nodes::<GprNode>(&graph), set(&[1, 3, 5]));
    assert_eq!(nodes::<FloatNode>(&graph), set(&[2, 4]));

    // Die float register R2 und R4 leben gleichzeitig und interferieren
    let float_neighbours = |reg| {
        virt(neighbours::<FloatNode>(&graph, v(reg), IntfEdge::Interferes))
    };
    assert_eq!(float_neighbours(2), set(&[4]));
    assert_eq!(float_neighbours(4), set(&[2]));

    // Zwischen den klassen gibt es keine kanten, obwohl z.b. R2 und R3 gleichzeitig leben
    for reg in (1..=5).map(v)
    {
        assert!(
            graph
                .any_link(reg, IntfEdge::Interferes)
                .blinks(&graph)
                .all(|n| {
                    matches!(
                        (graph.get_entry(reg).unwrap(), n),
                        (IntfEntries::GprNode(_), IntfEntries::GprNode(_))
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
        graph.get::<GprNode>(v(1)).unwrap().color(),
        NodeColor::Uncolored
    );
    assert_eq!(
        graph.get::<FloatNode>(v(2)).unwrap().color(),
        NodeColor::Uncolored
    );

    graph
        .get_mut::<GprNode>(v(1))
        .unwrap()
        .set_color(NodeColor::Colored(Color(3)));
    graph
        .get_mut::<FloatNode>(v(2))
        .unwrap()
        .set_color(NodeColor::Precolored(Color(0)));

    assert_eq!(
        graph.get::<GprNode>(v(1)).unwrap().color().color(),
        Some(Color(3))
    );
    assert_eq!(
        graph.get::<FloatNode>(v(2)).unwrap().color().color(),
        Some(Color(0))
    );

    // Der knotentyp muss zur klasse passen
    assert!(graph.get::<FloatNode>(v(1)).is_none());
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

    let funcs: Vec<_> = split_functions(chunks.clone()).collect();

    assert_eq!(funcs.len(), 2);
    assert_eq!(funcs[0], &chunks[0..2]);
    assert_eq!(funcs[1], &chunks[2..5]);

    // R256 ist in f ein int, in g ein float: pro funktion analysiert gibt es keinen konflikt
    let f = build_interference_graph::<SysV64>(&Liveness::new(&funcs[0]));
    let g = build_interference_graph::<SysV64>(&Liveness::new(&funcs[1]));
    assert_eq!(nodes::<GprNode>(&f), set(&[256]));
    assert_eq!(nodes::<FloatNode>(&g), set(&[256, 257]));
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

    let classes = register_classes::<SysV64>(&chunks);
    assert_eq!(classes.get(&RegNum(3)), None);
    assert_eq!(classes.get(&RegNum(4)), None);

    let graph = graph_of(&chunks);

    // Nur R1 und R2 brauchen ein register
    assert_eq!(nodes::<GprNode>(&graph), set(&[1, 2]));
    assert!(graph.get_entry(v(3)).is_none());
    assert!(graph.get_entry(v(4)).is_none());
}


/// Eine soft-float architektur: floats liegen in den allgemeinen registern,
/// eigene float register gibt es nicht
struct SoftFloat;

impl Arch for SoftFloat
{
    type Gpr = SysV64Gpr;
    type Float = EmptyFile;

    fn class_of(kind: Kind) -> Option<RegClass>
    {
        match kind
        {
            Kind::None => None,
            _ => Some(RegClass::Gpr),
        }
    }

    fn param_loc(_params: &[RegClass]) -> Vec<ParamLoc>
    {
        unimplemented!("not needed for the graph")
    }
}


/// f(R1: i64, R2: f64): return g(R1, R2)
/// -- auf soft-float landen floats in der gpr-datei und interferieren mit den ints
#[test]
fn interference_classes_by_arch()
{
    let chunks = vec![
        IrChunk::Func {
            name: "f".into(),
            args: vec![trk(1, Kind::I64), trk(2, Kind::F64)],
        },
        IrChunk::Call {
            name: "g".into(),
            args: vec![r(1), r(2)],
            reg: trk(3, Kind::I64),
        },
        IrChunk::Ret { val: r(3) },
    ];

    let liveness = Liveness::new(&chunks);

    // x86-64: getrennte dateien, keine kante zwischen R1 und R2
    let sysv = build_interference_graph::<SysV64>(&liveness);
    assert_eq!(nodes::<GprNode>(&sysv), set(&[1, 3]));
    assert_eq!(nodes::<FloatNode>(&sysv), set(&[2]));
    assert!(interferes(&sysv, 1).is_empty());

    // Soft-float: alles in einer datei, R1 und R2 leben gleichzeitig und interferieren
    assert_eq!(
        register_classes::<SoftFloat>(&chunks),
        HashMap::from([
            (RegNum(1), RegClass::Gpr),
            (RegNum(2), RegClass::Gpr),
            (RegNum(3), RegClass::Gpr),
        ])
    );

    let soft = build_interference_graph::<SoftFloat>(&liveness);
    assert_eq!(nodes::<GprNode>(&soft), set(&[1, 2, 3]));
    assert!(nodes::<FloatNode>(&soft).is_empty());
    assert_eq!(interferes(&soft, 1), set(&[2]));
    assert_eq!(interferes(&soft, 2), set(&[1]));
}


/// Maschinenregister haben eigene schlüssel: sie kollidieren weder mit virtuellen registern
/// noch mit maschinenregistern anderer klassen, die dieselbe farbe haben
#[test]
fn phys_nodes()
{
    let mut graph = IntfGraph::default();

    graph.insert(GprNode::new(RegNum(0)));
    graph.insert(GprNode::phys(Color(0)));
    graph.insert(FloatNode::phys(Color(0)));

    // Drei verschiedene knoten, obwohl überall die 0 drin steckt
    assert_eq!(graph.iter().count(), 3);

    let rax = graph
        .get::<GprNode>(NodeId::Phys(RegClass::Gpr, Color(0)))
        .unwrap();
    let xmm0 = graph
        .get::<FloatNode>(NodeId::Phys(RegClass::Float, Color(0)))
        .unwrap();

    // Ein maschinenregister ist mit seiner eigenen farbe vorgefärbt
    assert_eq!(rax.color(), NodeColor::Precolored(Color(0)));
    assert_eq!(xmm0.color(), NodeColor::Precolored(Color(0)));

    // Die klasse im schlüssel ist immer die des knotentyps
    assert_eq!(rax.id(), NodeId::Phys(GprNode::CLASS, Color(0)));
    assert_eq!(xmm0.id(), NodeId::Phys(FloatNode::CLASS, Color(0)));
    assert_eq!(rax.id().virt(), None);

    // Das virtuelle register R0 ist ein eigener, ungefärbter knoten
    let r0 = graph.get::<GprNode>(v(0)).unwrap();
    assert_eq!(r0.color(), NodeColor::Uncolored);
    assert_eq!(r0.id().virt(), Some(RegNum(0)));
    assert_eq!(NodeId::from(RegNum(0)), v(0));
}


// --- Vorfärbung ---


/// Interferenzgraph von `chunks` inklusive vorfärbung (SysV64)
fn precolored_graph_of(chunks: &[IrChunk<EnvEntry, Kind>]) -> IntfGraph
{
    let liveness = Liveness::new(chunks);
    let mut graph = build_interference_graph::<SysV64>(&liveness);
    precolor::<SysV64>(&mut graph, &liveness);
    graph
}

fn gpr(reg: SysV64Gpr) -> NodeId
{
    NodeId::Phys(RegClass::Gpr, SysV64Gpr::color_of(reg).unwrap())
}

fn xmm(reg: SysV64Xmm) -> NodeId
{
    NodeId::Phys(RegClass::Float, SysV64Xmm::color_of(reg).unwrap())
}

/// Alle maschinenregister, mit denen `reg` über eine kante vom typ `edge` verbunden ist
fn phys_partners<N: IntfNode>(graph: &IntfGraph, reg: usize, edge: IntfEdge) -> BTreeSet<NodeId>
{
    neighbours::<N>(graph, v(reg), edge)
        .filter(|id| id.virt().is_none())
        .collect()
}


/// Jede verwendbare farbe jeder klasse bekommt genau einen, mit sich selbst vorgefärbten knoten
#[test]
fn precolor_phys_nodes()
{
    let chunks = vec![IrChunk::Func {
        name: "f".into(),
        args: vec![],
    }];

    let graph = precolored_graph_of(&chunks);

    for c in 0..SysV64Gpr::COLORS
    {
        let node = graph
            .get::<GprNode>(NodeId::Phys(RegClass::Gpr, Color(c)))
            .unwrap();
        assert_eq!(node.color(), NodeColor::Precolored(Color(c)));
    }

    for c in 0..SysV64Xmm::COLORS
    {
        let node = graph
            .get::<FloatNode>(NodeId::Phys(RegClass::Float, Color(c)))
            .unwrap();
        assert_eq!(node.color(), NodeColor::Precolored(Color(c)));
    }

    assert_eq!(graph.iter().count(), SysV64Gpr::COLORS + SysV64Xmm::COLORS);
}


/// f(R1: i64, R2: f64, R3: i64, R4..R9: i64)
/// -- gpr und float zählen getrennt, ab dem 7. gpr parameter geht es auf den stack
#[test]
fn precolor_func_params()
{
    let mut args = vec![trk(1, Kind::I64), trk(2, Kind::F64), trk(3, Kind::I64)];
    args.extend((4..=9).map(tr));

    let chunks = vec![
        IrChunk::Func {
            name: "f".into(),
            args,
        },
        IrChunk::Label { name: "L".into() },
    ];

    let graph = precolored_graph_of(&chunks);
    let hint = |reg| phys_partners::<GprNode>(&graph, reg, IntfEdge::Move);

    assert_eq!(hint(1), BTreeSet::from([gpr(SysV64Gpr::Rdi)]));
    assert_eq!(
        phys_partners::<FloatNode>(&graph, 2, IntfEdge::Move),
        BTreeSet::from([xmm(SysV64Xmm::Xmm0)])
    );
    assert_eq!(hint(3), BTreeSet::from([gpr(SysV64Gpr::Rsi)]));
    assert_eq!(hint(7), BTreeSet::from([gpr(SysV64Gpr::R9)]));

    // Stack parameter
    assert!(hint(8).is_empty());
    assert!(hint(9).is_empty());

    // Die kante ist auch vom maschinenregister aus sichtbar
    assert_eq!(
        virt(neighbours::<GprNode>(&graph, gpr(SysV64Gpr::Rdi), IntfEdge::Move)),
        set(&[1])
    );
}


/// f(R1): R2 = g(R1); R3 = R1 + R2; return R3
/// -- R1 lebt über den call und darf in keinem caller-saved register liegen
#[test]
fn precolor_call()
{
    let chunks = vec![
        /* 0 */
        IrChunk::Func {
            name: "f".into(),
            args: vec![tr(1)],
        },
        /* 1 */
        IrChunk::Call {
            name: "g".into(),
            args: vec![r(1)],
            reg: tr(2),
        },
        /* 2 */
        IrChunk::Bin {
            op: IrOp::Add,
            lhs: r(1),
            rhs: r(2),
            reg: tr(3),
        },
        /* 3 */ IrChunk::Ret { val: r(3) },
    ];

    let graph = precolored_graph_of(&chunks);

    // Clobber: R1 interferiert mit allen caller-saved und mit keinem callee-saved register
    let caller_saved = SysV64Gpr::CALLER_SAVED.iter().map(|&reg| gpr(reg)).collect();
    assert_eq!(phys_partners::<GprNode>(&graph, 1, IntfEdge::Interferes), caller_saved);

    // Das ergebnis wird vom call selbst geschrieben, lebt also nicht über ihn hinweg
    assert!(phys_partners::<GprNode>(&graph, 2, IntfEdge::Interferes).is_empty());

    // Hinweise: R1 ist parameter von f und argument von g (beide male rdi),
    // R2 kommt in rax zurück, R3 wird in rax zurückgegeben
    assert_eq!(
        phys_partners::<GprNode>(&graph, 1, IntfEdge::Move),
        BTreeSet::from([gpr(SysV64Gpr::Rdi)])
    );
    assert_eq!(
        phys_partners::<GprNode>(&graph, 2, IntfEdge::Move),
        BTreeSet::from([gpr(SysV64Gpr::Rax)])
    );
    assert_eq!(
        phys_partners::<GprNode>(&graph, 3, IntfEdge::Move),
        BTreeSet::from([gpr(SysV64Gpr::Rax)])
    );
}


/// f(R1: f64): R2 = g(1, 2.0, R1); return R2
/// -- literale haben kein register, zählen aber für die positionen der argumente mit
#[test]
fn precolor_call_literal_args()
{
    let chunks = vec![
        IrChunk::Func {
            name: "f".into(),
            args: vec![trk(1, Kind::F64)],
        },
        IrChunk::Call {
            name: "g".into(),
            args: vec![
                EnvEntry::Atom(Atom::Int(1)),
                EnvEntry::Atom(Atom::Float(2.0)),
                r(1),
            ],
            reg: trk(2, Kind::F64),
        },
        IrChunk::Ret { val: r(2) },
    ];

    let graph = precolored_graph_of(&chunks);

    // R1 ist parameter von f (xmm0) und das zweite float argument von g (xmm1)
    assert_eq!(
        phys_partners::<FloatNode>(&graph, 1, IntfEdge::Move),
        BTreeSet::from([xmm(SysV64Xmm::Xmm0), xmm(SysV64Xmm::Xmm1)])
    );

    // Float ergebnis und rückgabewert liegen in xmm0
    assert_eq!(
        phys_partners::<FloatNode>(&graph, 2, IntfEdge::Move),
        BTreeSet::from([xmm(SysV64Xmm::Xmm0)])
    );
}
