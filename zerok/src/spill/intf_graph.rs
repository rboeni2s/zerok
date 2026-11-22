use crate::spill::Liveness;
use cache_map::{ident::IdentRef, prelude::*};


#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum IntfEntries
{
    RegNum(RegNum),
}


#[derive(CacheEntry, Debug, Clone, PartialEq, Eq, Hash)]
#[cache_entry(IntfEntries)]
pub struct RegNum(usize);
impl IdentRef<usize> for RegNum
{
    fn ident_ref(&self) -> &usize
    {
        &self.0
    }
}


pub fn build_interference_graph(liveness: &Liveness) -> Cache<IntfEntries, usize>
{
    Cache::default()
}
