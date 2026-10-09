use super::{IrChunk, IrOp, IrReg};
use crate::annotator::EnvEntry;


#[derive(Default)]
pub struct IrBuilder
{
    chunks: Vec<IrChunk>,
}


impl IrBuilder
{
    pub fn chunk(&mut self, chunk: IrChunk) -> &mut Self
    {
        self.chunks.push(chunk);
        self
    }

    pub fn binop(&mut self, op: IrOp, lhs: &EnvEntry, rhs: &EnvEntry, reg: usize) -> &mut Self
    {
        self.chunks.push(IrChunk::Bin {
            op,
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            reg: IrReg::R(reg),
        });

        self
    }

    pub fn unaop(&mut self, op: IrOp, arg: &EnvEntry, reg: usize) -> &mut Self
    {
        self.chunks.push(IrChunk::Una {
            op,
            arg: arg.clone(),
            reg: IrReg::R(reg),
        });

        self
    }

    pub fn bind(&mut self, val: &EnvEntry, reg: usize) -> &mut Self
    {
        self.chunks.push(IrChunk::Bind {
            val: val.clone(),
            reg: IrReg::R(reg),
        });

        self
    }

    pub fn func(&mut self, name: impl Into<String>, args: &[IrReg]) -> &mut Self
    {
        self.chunks.push(IrChunk::Func {
            name: name.into(),
            args: args.to_vec(),
        });

        self
    }

    /// Returns the built chunks
    pub fn build(self) -> Vec<IrChunk>
    {
        self.chunks
    }
}
