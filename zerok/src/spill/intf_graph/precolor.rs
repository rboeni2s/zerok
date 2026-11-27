use super::graph_setup::*;
use crate::{
    annotator::{EnvEntry, Kind},
    spill::{
        Color,
        Liveness,
        RegNum,
        arch::{Arch, ParamLoc, RegFile},
        cfg::CfgRegs,
        intf_graph::register_classes,
        reg_helper::GetRegR,
    },
};
use ir::IrChunk;
use std::collections::HashMap;


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
    let classes = register_classes::<A>(chunks);

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

                hint_params::<A>(graph, &params);
            }

            IrChunk::Call { args, reg, .. } =>
            {
                // Function calls can overwrite caller saved register. Every register that is still
                // live after the call can therefore not be a caller saved register...
                let defs = chunk.cfg_def();
                for &x in liveness.live_out[n].iter().filter(|x| !defs.contains(x))
                {
                    let Some(&class) = classes.get(&x)
                    else
                    {
                        continue;
                    };

                    // Create a interferes edge between the (live) register and all of the
                    // physical, caller saved registers to prevent it being colored in a caller saved color
                    for color in caller_saved_colors::<A>(class)
                    {
                        graph
                            .any_link(NodeId::Virt(x), IntfEdge::Interferes)
                            .add(&NodeId::Phys(class, color));
                    }
                }

                // Function arguments passed to the call have the same layout as `IrChunk::Func`
                // and must be put into the architecture defined registers or stack positions
                let params: Vec<(Option<RegNum>, RegClass)> = args
                    .iter()
                    .filter_map(|arg| Some((arg.get_r_reg_of(), entry_class::<A>(arg, &classes)?)))
                    .collect();

                hint_params::<A>(graph, &params);

                // If the function call has a return value it should be in return register defined by the architecture.
                // Add a move edge to that register as a hint.
                if let (Some(reg), Some(class)) = (reg.get_r_reg_of(), A::class_of(reg.kind))
                {
                    if let Some(color) = return_color::<A>(class)
                    {
                        graph
                            .any_link(NodeId::Virt(reg), IntfEdge::Move)
                            .add(&NodeId::Phys(class, color));
                    };
                }
            }

            IrChunk::Ret { val } =>
            {
                // The return value must be put into the Architecture defined return register of its register class.
                if let Some(reg) = val.get_r_reg_of()
                    && let Some(&class) = classes.get(&reg)
                {
                    if let Some(color) = return_color::<A>(class)
                    {
                        graph
                            .any_link(NodeId::Virt(reg), IntfEdge::Move)
                            .add(&NodeId::Phys(class, color));
                    };
                }
            }

            // All other chunks do not have registers that would need precoloring
            _ => (),
        }
    }
}

/// inserts a node for every physical register in the regfile into the graph
fn insert_phys<A: Arch, N: IntfNode>(graph: &mut IntfGraph)
{
    for c in 0..<N::File<A> as RegFile>::COLORS
    {
        graph.insert(N::phys(Color(c)));
    }
}


/// Adds a move hint from every parameter to the machine register based on the calling convention of `A`
fn hint_params<A: Arch>(graph: &IntfGraph, params: &[(Option<RegNum>, RegClass)])
{
    // Fetch the register and stack positions for each function parameters from the architecture...
    let param_locations = {
        let classes = params.iter().map(|&(_, class)| class).collect::<Vec<_>>();
        A::param_loc(&classes)
    };

    for (&(reg, _), loc) in params.iter().zip(param_locations)
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
            .any_link(NodeId::Virt(reg), IntfEdge::Move)
            .add(&NodeId::Phys(class, color));
    }
}


/// The color of the return register of `class` on `A`
fn return_color<A: Arch>(class: RegClass) -> Option<Color>
{
    fn of<F: RegFile>() -> Option<Color>
    {
        F::color_of(F::RETURN)
    }

    match class
    {
        RegClass::Gpr => of::<A::Gpr>(),
        RegClass::Float => of::<A::Float>(),
    }
}


/// The colors of all caller saved registers of `class` on `A`
fn caller_saved_colors<A: Arch>(class: RegClass) -> Vec<Color>
{
    fn of<F: RegFile>() -> Vec<Color>
    {
        F::CALLER_SAVED
            .iter()
            .filter_map(|&reg| F::color_of(reg))
            .collect()
    }

    match class
    {
        RegClass::Gpr => of::<A::Gpr>(),
        RegClass::Float => of::<A::Float>(),
    }
}


/// The register class of an operand (e.g. an argument of a `Call`), `None` if it needs no register
fn entry_class<A: Arch>(entry: &EnvEntry, classes: &HashMap<RegNum, RegClass>) -> Option<RegClass>
{
    match entry
    {
        EnvEntry::Register(reg) => classes.get(&RegNum(*reg)).copied(),

        // Die genaue breite (i32/i64, f32/f64) ist für die klasse egal
        EnvEntry::Atom(atom) => A::class_of(Kind::of_atom(atom, None)),

        // Adressen von funktionen und labels
        EnvEntry::Label(_) => A::class_of(Kind::Intern),

        // TODO: Der kind eines globalen registers steht nicht im operanden.
        // Bisher halten globals nur interne daten (runtime data), sind also adressen
        EnvEntry::Global(_) => A::class_of(Kind::Intern),
    }
}
