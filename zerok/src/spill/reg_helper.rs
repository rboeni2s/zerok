use crate::annotator::{EnvEntry, Kind};
use crate::spill::RegNum;
use ir::{IrReg, IrTypedReg};


/// Extracts the R register number from a value (like `EnvEntry` or `IrTypedReg`)
pub trait GetRegR
{
    fn get_r_reg_of(&self) -> Option<RegNum>;
}


impl GetRegR for EnvEntry
{
    fn get_r_reg_of(&self) -> Option<RegNum>
    {
        match self
        {
            EnvEntry::Register(reg) => Some(*reg),
            EnvEntry::Atom(_) => None,
        }
    }
}

impl GetRegR for IrTypedReg<Kind>
{
    fn get_r_reg_of(&self) -> Option<RegNum>
    {
        match self.reg
        {
            IrReg::R(reg) => Some(reg),
            IrReg::G(_) => None,
        }
    }
}

pub fn operands<'a, G: GetRegR + 'a>(vals: impl IntoIterator<Item = &'a G>) -> Vec<RegNum>
{
    vals.into_iter()
        .filter_map(|item| item.get_r_reg_of())
        .collect()
}
