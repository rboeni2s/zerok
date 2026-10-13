use super::{IrChunk, IrOp, IrReg};


pub struct IrBuilder<T>
{
    chunks: Vec<IrChunk<T>>,
}


impl<T> Default for IrBuilder<T>
{
    fn default() -> Self
    {
        Self {
            chunks: Default::default(),
        }
    }
}


impl<T> IrBuilder<T>
{
    pub fn chunk(&mut self, chunk: IrChunk<T>) -> &mut Self
    {
        self.chunks.push(chunk);
        self
    }

    pub fn binop(&mut self, op: IrOp, lhs: T, rhs: T, reg: usize) -> &mut Self
    {
        self.chunks.push(IrChunk::Bin {
            op,
            lhs,
            rhs,
            reg: IrReg::R(reg),
        });

        self
    }

    pub fn unaop(&mut self, op: IrOp, arg: T, reg: usize) -> &mut Self
    {
        self.chunks.push(IrChunk::Una {
            op,
            arg,
            reg: IrReg::R(reg),
        });

        self
    }

    pub fn bind(&mut self, val: T, reg: usize) -> &mut Self
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

    pub fn call(&mut self, name: impl Into<String>, args: &[T], reg: usize) -> &mut Self
    where
        T: Clone,
    {
        self.chunks.push(IrChunk::Call {
            name: name.into(),
            args: args.to_vec(),
            reg: IrReg::R(reg),
        });

        self
    }

    pub fn ret(&mut self, val: T) -> &mut Self
    {
        self.chunks.push(IrChunk::Ret { val });
        self
    }

    pub fn label(&mut self, name: impl Into<String>) -> &mut Self
    {
        self.chunks.push(IrChunk::Label { name: name.into() });
        self
    }

    pub fn jump(&mut self, name: impl Into<String>) -> &mut Self
    {
        self.chunk(IrChunk::Jump { name: name.into() });
        self
    }

    pub fn conditional_jump(&mut self, val: T, name: impl Into<String>) -> &mut Self
    {
        self.chunk(IrChunk::CondJump {
            val,
            name: name.into(),
        });
        self
    }

    pub fn append(&mut self, chunks: Vec<IrChunk<T>>) -> &mut Self
    {
        self.chunks.extend(chunks);
        self
    }

    pub fn within(&mut self, f: impl Fn(&mut Self)) -> &mut Self
    {
        f(self);
        self
    }

    /// Returns the built chunks
    pub fn build(self) -> Vec<IrChunk<T>>
    {
        self.chunks
    }
}
