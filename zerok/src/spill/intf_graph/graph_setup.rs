use crate::spill::{
    Color,
    RegNum,
    arch::{Arch, RegFile},
};
use cache_map::ident::IdentRef;
use cache_map::prelude::*;


/// The register class of a value, decided by the architecture (see `Arch::class_of`).
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegClass
{
    Gpr,
    Float,
}


/// The key of a node in the interference graph
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeId
{
    /// A virtual register of the ir
    Virt(RegNum),

    /// A machine register, given by its class and its color in the register file of the class.
    Phys(RegClass, Color),
}


impl NodeId
{
    /// The virtual register of this node, `None` for a machine register
    pub fn virt(self) -> Option<RegNum>
    {
        match self
        {
            Self::Virt(reg) => Some(reg),
            Self::Phys(..) => None,
        }
    }
}


impl From<RegNum> for NodeId
{
    fn from(reg: RegNum) -> Self
    {
        Self::Virt(reg)
    }
}


#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum IntfEntries
{
    GprNode(GprNode),
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


/// A virtual or machine register of class [`RegClass::Gpr`]
#[derive(CacheEntry, IdentRef, Debug, Clone, PartialEq, Eq, Hash)]
#[cache_entry(IntfEntries)]
pub struct GprNode
{
    #[id]
    pub id: NodeId,
    pub color: NodeColor,
}


/// A virtual or machine register of class [`RegClass::Float`]
#[derive(CacheEntry, IdentRef, Debug, Clone, PartialEq, Eq, Hash)]
#[cache_entry(IntfEntries)]
pub struct FloatNode
{
    #[id]
    pub id: NodeId,
    pub color: NodeColor,
}


/// A node type of the interference graph, one per register class.
///
/// Lets the coloring work generically on one class at a time, e.g. through
/// `graph.variants::<N>()` and [`neighbours::<N>`].
///
/// The node type also selects the register file its nodes are colored from ([`Self::File`]).
/// The number of colors and the translation of a color back into a machine register therefore
/// both depend on the same `N` as the nodes and their neighbours, so a node can never be colored
/// with the colors of another class:
///
/// ```ignore
/// let k = <N::File<A> as RegFile>::COLORS;
/// let reg = <N::File<A> as RegFile>::reg_of(color);
/// ```
pub trait IntfNode: CacheEntry<IntfEntries> + IdentRef<NodeId>
{
    const CLASS: RegClass;

    /// The register file the nodes of this type are colored from on architecture `A`
    type File<A: Arch>: RegFile;

    /// Creates an uncolored node for the virtual register `reg`
    fn new(reg: RegNum) -> Self;

    /// Creates the node of the machine register with color `color` of this class.
    /// It is precolored with its own color.
    ///
    /// The class of the key is always [`Self::CLASS`], so a machine register can only ever be a
    /// node of the type of its own class.
    fn phys(color: Color) -> Self;

    /// The key of this node in the graph
    fn id(&self) -> NodeId
    {
        *self.ident_ref()
    }

    fn color(&self) -> NodeColor;
    fn set_color(&mut self, color: NodeColor);
}


impl IntfNode for GprNode
{
    const CLASS: RegClass = RegClass::Gpr;
    type File<A: Arch> = A::Gpr;

    fn new(reg: RegNum) -> Self
    {
        Self {
            id: NodeId::Virt(reg),
            color: NodeColor::Uncolored,
        }
    }

    fn phys(color: Color) -> Self
    {
        Self {
            id: NodeId::Phys(Self::CLASS, color),
            color: NodeColor::Precolored(color),
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
    type File<A: Arch> = A::Float;

    fn new(reg: RegNum) -> Self
    {
        Self {
            id: NodeId::Virt(reg),
            color: NodeColor::Uncolored,
        }
    }

    fn phys(color: Color) -> Self
    {
        Self {
            id: NodeId::Phys(Self::CLASS, color),
            color: NodeColor::Precolored(color),
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
    /// the move could be removed (coalescing).
    Move,
}


/// The interference graph
///
/// Holds the virtual and machine registers of all classes, but edges only ever connect registers
/// of the same class.
pub type IntfGraph = Cache<NodeId, IntfEntries, IntfEdge>;


/// Returns all nodes of type `N` connected to `node` by an edge of kind `edge`, regardless of the
/// direction the link is stored in. For `IntfEdge::Interferes` its count is the degree of `node`.
pub fn neighbours<'a, N>(
    graph: &'a IntfGraph,
    node: impl Into<NodeId>,
    edge: IntfEdge,
) -> impl Iterator<Item = NodeId> + 'a
where
    N: IntfNode + 'a,
{
    graph
        .any_link(node.into(), edge)
        .blinks(graph)
        .filter_map(N::unpack)
        .map(|node| node.id())
}
