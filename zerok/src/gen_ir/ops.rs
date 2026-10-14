use super::GenEnv;
use crate::{
    ExprAst,
    annotator::EnvEntry,
    parser::ops::{Binop, Unaop},
    reg_util,
};
use iic_quote::iic_quote;
use ir::{IrBuilder, IrOp};


impl<'a> GenEnv<'_, 'a>
{
    pub(super) fn generate_binop(
        &self,
        expr: &ExprAst<'a>,
        op: &Binop,
        lhs: &ExprAst<'a>,
        rhs: &ExprAst<'a>,
        ir: &mut IrBuilder<EnvEntry>,
    )
    {
        let lhs = self.generate_operand(lhs, ir);
        let rhs = self.generate_operand(rhs, ir);
        let reg = self.register_of(expr);

        // Operators that map directly onto an ir operator are emitted at the end,
        // all others are built from multiple chunks
        let op = match op
        {
            Binop::Add => IrOp::Add,
            Binop::Sub => IrOp::Sub,
            Binop::Mul => IrOp::Mul,
            Binop::Div => IrOp::Div,
            Binop::Eq => IrOp::Eq,
            Binop::Neq => IrOp::Neq,
            Binop::Lt => IrOp::Lt,
            Binop::Gt => IrOp::Gt,
            Binop::Lte => IrOp::Lte,
            Binop::Gte => IrOp::Gte,
            Binop::BoolAnd => IrOp::And,
            Binop::BoolOr => IrOp::Or,

            Binop::Mod | Binop::Pow if is_float(expr) =>
            {
                todo!("The {op:?} operator is not implemented for floats in the ir yet")
            }

            Binop::Mod => return generate_mod(lhs, rhs, reg, ir),
            Binop::Pow => return self.generate_pow(lhs, rhs, reg, ir),
        };

        ir.binop(op, lhs, rhs, reg);
    }


    pub(super) fn generate_unaop(
        &self,
        expr: &ExprAst<'a>,
        op: &Unaop,
        val: &ExprAst<'a>,
        ir: &mut IrBuilder<EnvEntry>,
    )
    {
        let val = self.generate_operand(val, ir);

        let op = match op
        {
            Unaop::Neg => IrOp::Sub,
            Unaop::BoolNot => IrOp::Not,
        };

        ir.unaop(op, val, self.register_of(expr));
    }


    fn generate_pow(&self, base: EnvEntry, exp: EnvEntry, reg: usize, ir: &mut IrBuilder<EnvEntry>)
    {
        let loop_label = &self.func.unique_label();
        let end_label = &self.func.unique_label();

        let counter = reg_util::next_register();
        let done = reg_util::next_register();

        ir.append(iic_quote! {
            R(reg) = 1;
            R(counter) = exp;

            loop_label:

                R(done) = R(counter) <= 0;
                if R(done) goto end_label;

                R(reg) = R(reg) * base;
                R(counter) = R(counter) - 1;
                goto loop_label;

            end_label:
        });
    }
}


/// a % b = a - (a / b) * b
fn generate_mod(lhs: EnvEntry, rhs: EnvEntry, reg: usize, ir: &mut IrBuilder<EnvEntry>)
{
    ir.append(iic_quote! {
        R(reg) = lhs.clone() / rhs.clone();
        R(reg) = R(reg) * rhs;
        R(reg) = lhs - R(reg);
    });
}


fn is_float(expr: &ExprAst<'_>) -> bool
{
    expr.data
        .as_ref()
        .is_some_and(|annotation| annotation.kind.is_float())
}
