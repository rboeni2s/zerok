use crate::{
    annotator::Kind,
    spill::{Color, RegNum},
};
use cache_map::ident::IdentRef;
use cache_map::prelude::*;


/// The register class of a value.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum RegClass
{
    Int,
    Float,
}


impl RegClass
{
    /// The class of a value of `kind`, or `None` for `Kind::None`: a none value carries no data
    /// and therefore needs no machine register.
    pub fn of(kind: Kind) -> Option<Self>
    {
        match kind
        {
            Kind::None => None,
            Kind::F32 | Kind::F64 => Some(Self::Float),
            _ => Some(Self::Int),
        }
    }
}


#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum IntfEntries
{
    IntNode(IntNode),
    FloatNode(FloatNode),
}


/// The coloring state of a node
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
pub enum NodeColor
{
    /// Not colored yet
    #[default]
    Uncolored,

    /// Fixed before the coloring (e.g. by the calling convention).
    /// Such a node is never simplified or spilled.
    Precolored(Color),

    /// Assigned by the coloring (select)
    Colored(Color),

    /// The coloring found no free color, the register has to live in memory
    Spilled,
}


impl NodeColor
{
    /// The color occupied by this node, if any. Precolored and colored nodes are treated alike:
    /// their neighbours must not use the same color.
    pub fn color(self) -> Option<Color>
    {
        match self
        {
            Self::Precolored(color) | Self::Colored(color) => Some(color),
            Self::Uncolored | Self::Spilled => None,
        }
    }
}


/// A (virtual) register of class [`RegClass::Int`]
#[derive(CacheEntry, IdentRef, Debug, Clone, PartialEq, Eq, Hash)]
#[cache_entry(IntfEntries)]
pub struct IntNode
{
    #[id]
    pub reg: RegNum,
    pub color: NodeColor,
}


/// A (virtual) register of class [`RegClass::Float`]
#[derive(CacheEntry, IdentRef, Debug, Clone, PartialEq, Eq, Hash)]
#[cache_entry(IntfEntries)]
pub struct FloatNode
{
    #[id]
    pub reg: RegNum,
    pub color: NodeColor,
}


/// A node type of the interference graph, one per register class.
///
/// Lets the coloring work generically on one class at a time, e.g. through
/// `graph.variants::<N>()` and [`neighbours::<N>`].
pub trait IntfNode: CacheEntry<IntfEntries> + IdentRef<RegNum>
{
    const CLASS: RegClass;

    /// Creates an uncolored node for `reg`
    fn new(reg: RegNum) -> Self;

    fn color(&self) -> NodeColor;

    fn set_color(&mut self, color: NodeColor);
}


impl IntfNode for IntNode
{
    const CLASS: RegClass = RegClass::Int;

    fn new(reg: RegNum) -> Self
    {
        Self {
            reg,
            color: NodeColor::Uncolored,
        }
    }

    fn color(&self) -> NodeColor
    {
        self.color
    }

    fn set_color(&mut self, color: NodeColor)
    {
        self.color = color;
    }
}


impl IntfNode for FloatNode
{
    const CLASS: RegClass = RegClass::Float;

    fn new(reg: RegNum) -> Self
    {
        Self {
            reg,
            color: NodeColor::Uncolored,
        }
    }

    fn color(&self) -> NodeColor
    {
        self.color
    }

    fn set_color(&mut self, color: NodeColor)
    {
        self.color = color;
    }
}


/// The kinds of edges in the interference graph
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum IntfEdge
{
    /// Both registers are live at the same time and must never share a machine register
    Interferes,

    /// The registers are connected by a move (`Bind`). If they get the same machine register,
    /// the move can be removed (coalescing). This is only a hint, not a constraint.
    Move,
}


/// The interference graph
///
/// Holds the registers of all classes, but edges only ever connect registers of the same class.
pub type IntfGraph = Cache<RegNum, IntfEntries, IntfEdge>;


/// Returns all registers of node type `N` connected to `reg` by an edge of kind `edge`, regardless
/// of the direction the link is stored in. For `IntfEdge::Interferes` its count is the degree of
/// `reg`.
pub fn neighbours<'a, N>(
    graph: &'a IntfGraph,
    reg: RegNum,
    edge: IntfEdge,
) -> impl Iterator<Item = RegNum> + 'a
where
    N: IntfNode + 'a,
{
    graph
        .any_link(reg, edge)
        .blinks(graph)
        .filter_map(N::unpack)
        .map(|node| *node.ident_ref())
}
