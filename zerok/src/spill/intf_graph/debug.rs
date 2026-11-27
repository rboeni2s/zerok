use std::io::{BufWriter, Result, Write};
use std::path::PathBuf;

use ir::IrChunk;

use crate::annotator::{EnvEntry, Kind};
use crate::spill::arch::{Arch, RegFile};
use crate::spill::{
    FloatNode,
    GprNode,
    IntfEdge,
    IntfGraph,
    IntfNode,
    NodeColor,
    NodeId,
    RegClass,
};


const DEFAULT_DEBUG_OUT_DIR: &str = "debug";


/// Background colors of the nodes in the dot output, indexed by the color of a node.
const DOT_COLORS: &[&str] = &[
    "gold",
    "deepskyblue",
    "tomato",
    "limegreen",
    "orchid",
    "darkorange",
    "turquoise",
    "hotpink",
    "yellowgreen",
    "mediumpurple",
    "khaki",
    "lightcoral",
    "cadetblue",
    "sandybrown",
    "palegreen",
    "plum",
    "lightblue",
    "salmon",
    "darkseagreen",
    "thistle",
    "wheat",
    "aquamarine",
    "lightpink",
    "lightsteelblue",
    "burlywood",
    "greenyellow",
    "lightskyblue",
    "rosybrown",
    "paleturquoise",
    "lavender",
    "lightgoldenrod",
    "mediumaquamarine",
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

    // graphviz settings
    writeln!(file, "    layout=neato;")?;
    writeln!(file, "    overlap=false;")?;
    writeln!(file, "    splines=true;")?;
    writeln!(file, "    sep=\"+6\";")?;

    // Draw one graph per featured register file of the architecture
    write_class::<A, GprNode>(&mut file, graph)?;
    write_class::<A, FloatNode>(&mut file, graph)?;

    let mut keys: Vec<NodeId> = graph.iter().map(|(&key, _)| key).collect();
    keys.sort();

    // Loop over every edge and set its style to either dashed (move) or solid (interferes)
    for &from in &keys
    {
        for (edge, style) in [
            (IntfEdge::Interferes, "style=solid"),
            (IntfEdge::Move, "style=dashed, color=blue, penwidth=2"),
        ]
        {
            let mut targets: Vec<NodeId> = graph.any_link(from, edge).ids().collect();
            targets.sort();

            for to in targets
            {
                writeln!(file, "    {} -- {} [{style}];", dot_id(from), dot_id(to))?;
            }
        }
    }

    writeln!(file, "}}")?;
    file.flush()
}


/// Writes all nodes of type `N`, in the shape of their class
fn write_class<A: Arch, N: IntfNode>(out: &mut impl Write, graph: &IntfGraph) -> Result<()>
{
    // Maschinenregister ohne kanten weglassen, virtuelle register immer zeichnen
    let mut nodes: Vec<&N> = graph
        .variants::<N>()
        .filter(|node| node.id().virt().is_some() || has_edges(graph, node.id()))
        .collect();
    nodes.sort_by_key(|node| node.id());

    // get the physical register names like `rdi`
    let reg_name = |color| format!("{:?}", <N::File<A> as RegFile>::reg_of(color)).to_lowercase();

    // unique shape per register file
    let shape = match N::CLASS
    {
        RegClass::Gpr => "circle",
        RegClass::Float => "box",
    };

    for node in nodes
    {
        let (label, peripheries) = match node.id()
        {
            NodeId::Virt(reg) =>
            {
                match node.color().color()
                {
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
            "    {} [label=\"{label}\", shape={shape}, peripheries={peripheries}{fill}];",
            dot_id(node.id())
        )?;
    }

    Ok(())
}


fn has_edges(graph: &IntfGraph, id: NodeId) -> bool
{
    [IntfEdge::Interferes, IntfEdge::Move]
        .into_iter()
        .any(|edge| {
            let link = graph.any_link(id, edge);
            link.ids().next().is_some() || link.ids_inverse().next().is_some()
        })
}


/// maps the node names to node names in the graphviz file
fn dot_id(id: NodeId) -> String
{
    match id
    {
        NodeId::Virt(reg) => format!("v{}", reg.0),
        NodeId::Phys(class, color) => format!("{class:?}_{}", color.0),
    }
}
