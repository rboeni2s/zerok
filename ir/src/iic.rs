use super::{IrChunk, IrOp, IrReg};


pub trait ToIIC
{
    const SEPERATOR: &str = ", ";
    fn to_iic(&self) -> String;
}


impl<T> ToIIC for Vec<T>
where
    T: ToIIC,
{
    fn to_iic(&self) -> String
    {
        self.iter()
            .map(|e| e.to_iic())
            .intersperse(T::SEPERATOR.to_string())
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
            IrOp::Not => "not",
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

impl<T> ToIIC for IrChunk<T>
where
    T: ToIIC,
    Vec<T>: ToIIC,
{
    const SEPERATOR: &str = "\n";

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

            IrChunk::Label { name } => format!("{name}:"),
            IrChunk::Jump { name } => format!("    goto {name}"),
            IrChunk::CondJump { val, name } => format!("    if {} goto {name}", val.to_iic()),
        }
    }
}
