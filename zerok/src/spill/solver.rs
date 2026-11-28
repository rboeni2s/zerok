use crate::{
    annotator::{EnvEntry, Kind},
    spill::{
        IntfGraph,
        Liveness,
        arch::Arch,
        build_interference_graph,
        intf_graph::{color, debug, precolor},
    },
};
use ir::IrChunk;


/// Determines the register colors and possibly their spilling.
///
/// Inserts spill code into `func_chunk` if necessary.
pub fn color_and_spill<A: Arch>(func_chunk: &mut Vec<IrChunk<EnvEntry, Kind>>) -> IntfGraph
{
    // if "-d" is passed: output the debug graphs
    let should_debug = std::env::args().find(|v| v == "-d").is_some();
    let dbg_graph = |prefix: &str, graph: &IntfGraph| {
        if should_debug
        {
            debug::save_graph_as_dot::<A>(
                format!("{prefix}_{}", debug::get_fn_name_or(func_chunk, "UNKNOWN")),
                graph,
            )
            .expect("Failed to write debug graph");
        }
    };

    let mut pass = 0;
    let mut old_spills = vec![];

    loop
    {
        // Perform the liveness analysis
        let liveness = Liveness::new(func_chunk);

        // Built the interference graph based on the liveness analysis
        let mut graph = build_interference_graph::<A>(&liveness);

        // Now Precolor the graph based on the architecture A's calling conventions
        precolor::<A>(&mut graph, &liveness);
        dbg_graph(&format!("PRE_{pass}"), &graph);

        // Finally try and color the graph
        let spills = color::<A>(&mut graph);
        dbg_graph(&format!("COL_{pass}",), &graph);

        // Return the graph if there are no spills
        if spills.is_empty()
        {
            return graph;
        }

        // Panic if the spill code generation would never resolve
        assert_ne!(old_spills, spills, "Spilled registers will not resolve");

        // If there are spills: Generate spill code and try again
        //TODO: Generate spill code...

        old_spills = spills;
        pass += 1;
    }
}
