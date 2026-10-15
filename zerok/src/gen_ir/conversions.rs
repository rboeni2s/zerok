use crate::{
    annotator::{EnvEntry, Kind},
    parser::atom::Atom,
};
use ir::{ToIIC, ToIrReg, ToIrVal};


macro_rules! impl_to_ir_val_for_T {
    ($($ty:ty),*) => {
        $(
            impl ToIrVal<EnvEntry> for $ty
            {
                fn to_ir_val(self) -> EnvEntry
                {
                    EnvEntry::Atom(Atom::Int(self as u64))
                }
            }
        )*
    };
}

impl_to_ir_val_for_T!(i32, u8, u16, u32, u64, usize);


impl ToIrVal<EnvEntry> for bool
{
    fn to_ir_val(self) -> EnvEntry
    {
        EnvEntry::Atom(Atom::Bool(self))
    }
}


impl ToIrReg<EnvEntry> for usize
{
    fn to_ir_reg(self) -> EnvEntry
    {
        EnvEntry::Register(self)
    }
}


impl ToIIC for EnvEntry
{
    fn to_iic(&self) -> String
    {
        match self
        {
            EnvEntry::Register(reg) => format!("R{reg}"),
            EnvEntry::Atom(atom) =>
            {
                match atom
                {
                    Atom::Str(_) =>
                    {
                        todo!(
                            "IIC string repräsentationen sind noch nicht implementiert, aber wahrscheinlich [char]"
                        )
                    }
                    Atom::Int(num) => num.to_string(),
                    Atom::Float(num) => (*num as u64).to_string(),
                    Atom::Bool(b) => u64::from(*b).to_string(),
                    Atom::Nop => unreachable!("Für ein nop sollte nie code generiert werden"),
                }
            }
        }
    }
}


impl ToIIC for Kind
{
    fn to_iic(&self) -> String
    {
        self.to_string()
    }
}
