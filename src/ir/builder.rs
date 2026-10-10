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

    pub fn binop(&mut self, op: IrOp, lhs: EnvEntry, rhs: EnvEntry, reg: usize) -> &mut Self
    {
        self.chunks.push(IrChunk::Bin {
            op,
            lhs,
            rhs,
            reg: IrReg::R(reg),
        });

        self
    }

    pub fn unaop(&mut self, op: IrOp, arg: EnvEntry, reg: usize) -> &mut Self
    {
        self.chunks.push(IrChunk::Una {
            op,
            arg,
            reg: IrReg::R(reg),
        });

        self
    }

    pub fn bind(&mut self, val: EnvEntry, reg: usize) -> &mut Self
    {
        self.chunks.push(IrChunk::Bind {
            val,
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

    pub fn call(&mut self, name: impl Into<String>, args: &[EnvEntry], reg: usize) -> &mut Self
    {
        self.chunks.push(IrChunk::Call {
            name: name.into(),
            args: args.to_vec(),
            reg: IrReg::R(reg),
        });

        self
    }

    pub fn ret(&mut self, val: EnvEntry) -> &mut Self
    {
        self.chunks.push(IrChunk::Ret { val });
        self
    }

    /// Returns the built chunks
    pub fn build(self) -> Vec<IrChunk>
    {
        self.chunks
    }
}
