use super::{GenEnv, kind_of};
use crate::{
    ExprAst,
    annotator::{EnvEntry, Kind},
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
        ir: &mut IrBuilder<EnvEntry, Kind>,
    )
    {
        let exp_kind = kind_of(rhs);

        let lhs = self.generate_operand(lhs, ir);
        let rhs = self.generate_operand(rhs, ir);
        let reg = self.register_of(expr);
        let kind = kind_of(expr);

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

            Binop::Mod | Binop::Pow if kind.is_float() =>
            {
                todo!("The {op:?} operator is not implemented for floats in the ir yet")
            }

            Binop::Mod => return generate_mod(lhs, rhs, reg, kind, ir),
            Binop::Pow => return self.generate_pow(lhs, rhs, exp_kind, reg, kind, ir),
        };

        ir.binop(op, lhs, rhs, reg, kind);
    }


    pub(super) fn generate_unaop(
        &self,
        expr: &ExprAst<'a>,
        op: &Unaop,
        val: &ExprAst<'a>,
        ir: &mut IrBuilder<EnvEntry, Kind>,
    )
    {
        let val = self.generate_operand(val, ir);

        let op = match op
        {
            Unaop::Neg => IrOp::Sub,
            Unaop::BoolNot => IrOp::Not,

            Unaop::Ref =>
            {
                let reg = self.register_of(expr);

                ir.append(iic_quote! {
                    R(reg, kind_of(expr)) = <mk, 1>;
                    R(reg, kind_of(expr))<0> = val;
                });

                return;
            }

            Unaop::Deref =>
            {
                let reg = self.register_of(expr);

                ir.append(iic_quote! {
                   R(reg, kind_of(expr)) = val<0>;
                });

                return;
            }
        };

        ir.unaop(op, val, self.register_of(expr), kind_of(expr));
    }


    fn generate_pow(
        &self,
        base: EnvEntry,
        exp: EnvEntry,
        exp_kind: Kind,
        reg: usize,
        kind: Kind,
        ir: &mut IrBuilder<EnvEntry, Kind>,
    )
    {
        let loop_label = &self.func.unique_label();
        let end_label = &self.func.unique_label();

        let counter = reg_util::next_register();
        let done = reg_util::next_register();

        ir.append(iic_quote! {
            R(reg, kind) = 1;
            R(counter, exp_kind) = exp;

            loop_label:

                R(done, Kind::Bool) = R(counter) <= 0;
                if R(done) goto end_label;

                R(reg, kind) = R(reg) * base;
                R(counter, exp_kind) = R(counter) - 1;
                goto loop_label;

            end_label:
        });
    }
}


/// a % b = a - (a / b) * b
fn generate_mod(
    lhs: EnvEntry,
    rhs: EnvEntry,
    reg: usize,
    kind: Kind,
    ir: &mut IrBuilder<EnvEntry, Kind>,
)
{
    ir.append(iic_quote! {
        R(reg, kind) = lhs.clone() / rhs.clone();
        R(reg, kind) = R(reg) * rhs;
        R(reg, kind) = lhs - R(reg);
    });
}
