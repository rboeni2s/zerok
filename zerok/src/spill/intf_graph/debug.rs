use std::io::{BufWriter, Result, Write};
use std::path::PathBuf;

use ir::IrChunk;

use crate::annotator::{EnvEntry, Kind};
use crate::spill::arch::{Arch, RegFile};
use crate::spill::{FloatNode, GprNode, IntfEdge, IntfGraph, IntfNode, NodeColor, NodeId};


const DEFAULT_DEBUG_OUT_DIR: &str = "debug";


/// Background colors of the nodes in the dot output, indexed by the color of a node.
const DOT_COLORS: &[&str] = &[
    "lightblue",
    "lightcoral",
    "palegreen",
    "khaki",
    "plum",
    "lightsalmon",
    "aquamarine",
    "lightpink",
    "wheat",
    "lightskyblue",
    "darkseagreen",
    "thistle",
    "lightgoldenrod",
    "paleturquoise",
    "burlywood",
    "lightsteelblue",
];


fn get_debug_dir() -> Result<PathBuf>
{
    let pwd = std::env::current_dir()?;
    let debug_dir = std::env::var("ZEROK_DEBUG_DIR").unwrap_or(DEFAULT_DEBUG_OUT_DIR.into());
    let path = pwd.join(debug_dir);

    // Ensure that the debug dir exists
    std::fs::create_dir_all(&path)?;

    Ok(path)
}


pub fn get_fn_name_or<'a>(chunks: &'a [IrChunk<EnvEntry, Kind>], or: &'a str) -> &'a str
{
    match chunks.get(0)
    {
        Some(IrChunk::Func { name, .. }) => name,
        _ => or,
    }
}


/// Outputs `graph` as graphviz file `<debug dir>/<name>.dot`.
pub fn save_graph_as_dot<A: Arch>(name: impl std::fmt::Display, graph: &IntfGraph) -> Result<()>
{
    let file_name = get_debug_dir()?.join(format!("{name}.dot"));
    let mut file = BufWriter::new(std::fs::File::create(file_name)?);

    writeln!(file, "graph \"{name}\" {{")?;
    writeln!(file, "    node [shape=circle];")?;

    // Knoten, pro registersatz ein cluster
    write_class::<A, GprNode>(&mut file, graph)?;
    write_class::<A, FloatNode>(&mut file, graph)?;

    // Kanten. Jede ungerichtete kante ist genau einmal (min → max) gespeichert,
    // `ids` liefert also jede kante genau einmal. Sortiert, damit die ausgabe stabil bleibt.
    let mut keys: Vec<NodeId> = graph.iter().map(|(&key, _)| key).collect();
    keys.sort();

    for &from in &keys
    {
        for (edge, style) in [(IntfEdge::Interferes, "solid"), (IntfEdge::Move, "dashed")]
        {
            let mut targets: Vec<NodeId> = graph.any_link(from, edge).ids().collect();
            targets.sort();

            for to in targets
            {
                writeln!(
                    file,
                    "    {} -- {} [style={style}];",
                    dot_id(from),
                    dot_id(to)
                )?;
            }
        }
    }

    writeln!(file, "}}")?;
    file.flush()
}


/// Writes all nodes of type `N` as one cluster
fn write_class<A: Arch, N: IntfNode>(out: &mut impl Write, graph: &IntfGraph) -> Result<()>
{
    let mut nodes: Vec<&N> = graph.variants::<N>().collect();
    nodes.sort_by_key(|node| node.id());

    // Klassen ohne register (zb float auf soft-float) gar nicht erst zeichnen
    if nodes.is_empty()
    {
        return Ok(());
    }

    // Name eines maschinenregisters dieser klasse, zb `rdi`
    let reg_name = |color| format!("{:?}", <N::File<A> as RegFile>::reg_of(color)).to_lowercase();

    writeln!(out, "    subgraph cluster_{:?} {{", N::CLASS)?;
    writeln!(out, "        label=\"{:?}\";", N::CLASS)?;

    for node in nodes
    {
        let (label, peripheries) = match node.id()
        {
            NodeId::Virt(reg) =>
            {
                match node.color().color()
                {
                    // Gefärbte virtuelle register zeigen zusätzlich ihr maschinenregister
                    Some(color) => (format!("{reg}\\n{}", reg_name(color)), 1),
                    None => (reg.to_string(), 1),
                }
            }
            NodeId::Phys(_, color) => (reg_name(color), 2),
        };

        let fill = match node.color()
        {
            NodeColor::Precolored(color) | NodeColor::Colored(color) =>
            {
                format!(
                    ", style=filled, fillcolor={}",
                    DOT_COLORS[color.0 % DOT_COLORS.len()]
                )
            }
            NodeColor::Spilled => ", style=filled, fillcolor=black, fontcolor=white".into(),
            NodeColor::Uncolored => String::new(),
        };

        writeln!(
            out,
            "        {} [label=\"{label}\", peripheries={peripheries}{fill}];",
            dot_id(node.id())
        )?;
    }

    writeln!(out, "    }}")
}


/// Eindeutiger name eines knotens in der dot datei
fn dot_id(id: NodeId) -> String
{
    match id
    {
        NodeId::Virt(reg) => format!("v{}", reg.0),
        NodeId::Phys(class, color) => format!("{class:?}_{}", color.0),
    }
}
