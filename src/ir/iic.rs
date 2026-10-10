use super::{IrChunk, IrOp, IrReg};
use crate::annotator::EnvEntry;
use crate::parser::atom::Atom;


pub trait ToIIC
{
    fn to_iic(&self) -> String;
}


impl ToIIC for Vec<IrReg>
{
    fn to_iic(&self) -> String
    {
        self.iter()
            .map(|e| e.to_iic())
            .intersperse(", ".to_string())
            .collect()
    }
}


impl ToIIC for Vec<EnvEntry>
{
    fn to_iic(&self) -> String
    {
        self.iter()
            .map(|e| e.to_iic())
            .intersperse(", ".to_string())
            .collect()
    }
}


impl ToIIC for Vec<IrChunk>
{
    fn to_iic(&self) -> String
    {
        self.iter()
            .map(|e| e.to_iic())
            .intersperse("\n".to_string())
            .collect()
    }
}


impl ToIIC for IrOp
{
    fn to_iic(&self) -> String
    {
        match self
        {
            IrOp::Add => "+",
            IrOp::Sub => "-",
            IrOp::Mul => "*",
            IrOp::Div => "/",
            IrOp::Lt => "<",
            IrOp::Lte => "<=",
            IrOp::Eq => "==",
            IrOp::Neq => "!=",
            IrOp::Gt => ">",
            IrOp::Gte => ">=",
            IrOp::And => "and",
            IrOp::Or => "or",
        }
        .into()
    }
}


impl ToIIC for IrReg
{
    fn to_iic(&self) -> String
    {
        match self
        {
            IrReg::R(val) => format!("R{val}"),
            IrReg::G(val) => format!("G{val}"),
        }
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
                    Atom::Nop => unreachable!("Für ein nop sollte nie code generiert werden"),
                }
            }
        }
    }
}


impl ToIIC for IrChunk
{
    fn to_iic(&self) -> String
    {
        match self
        {
            IrChunk::Bin { op, lhs, rhs, reg } =>
            {
                format!(
                    "    {} = {} {} {}",
                    reg.to_iic(),
                    lhs.to_iic(),
                    op.to_iic(),
                    rhs.to_iic()
                )
            }

            IrChunk::Una { op, arg, reg } =>
            {
                format!("    {} = {} {}", reg.to_iic(), op.to_iic(), arg.to_iic())
            }

            IrChunk::Bind { val, reg } => format!("    {} = {}", reg.to_iic(), val.to_iic()),
            IrChunk::Func { name, args } => format!("\ndefine {name}({}):", args.to_iic()),
            IrChunk::Ret { val } => format!("    ret {}", val.to_iic()),
            IrChunk::Call { name, args, reg } =>
            {
                format!("    {} = call {name}({})", reg.to_iic(), args.to_iic())
            }
        }
    }
}
