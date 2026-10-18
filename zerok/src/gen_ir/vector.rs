use super::{GenEnv, kind_of};
use crate::{
    ExprAst,
    annotator::{EnvEntry, Kind},
    parser::{
        Type,
        ops::{Binop, Unaop},
    },
    reg_util::next_register,
};
use iic_quote::iic_quote;
use ir::{IrBuilder, IrOp, ToIrVal};


impl<'a> GenEnv<'_, 'a>
{
    /// `[length: default]`
    pub(super) fn generate_new_vector(
        &self,
        expr: &ExprAst<'a>,
        length: &ExprAst<'a>,
        default: &ExprAst<'a>,
        ir: &mut IrBuilder<EnvEntry, Kind>,
    )
    {
        let length = self.generate_operand(length, ir);
        // let default = self.generate_operand(default, ir);

        let (result_reg, result_kind) = (self.register_of(expr), kind_of(expr));
        let loop_start = &self.func.unique_label();
        let loop_end = &self.func.unique_label();
        let index = next_register();
        let condition = next_register();


        ir.append(iic_quote! {
            // Reserve memory for the vector in result
            R(result_reg, result_kind) = <mk, length.clone()>;

            // Now generate the default value into every index of the vector
            R(index, Kind::U32) = 0;
            loop_start:
                // while index < length
                R(condition, Kind::U32) = R(index) >= length;
                if R(condition) goto loop_end;
        });

        let default_val = self.generate_operand(default, ir);

        ir.append(iic_quote! {
                // result[index] = default
                R(result_reg, result_kind)<R(index)> = default_val;

                // index++
                R(index, Kind::U32) = R(index) + 1;

                goto loop_start;
            loop_end:
        });
    }

    /// `"abc"`, a string is a `[char]`
    pub(super) fn generate_string(
        &self,
        expr: &ExprAst<'a>,
        string: &str,
        ir: &mut IrBuilder<EnvEntry, Kind>,
    )
    {
        let (result_reg, result_kind) = (self.register_of(expr), kind_of(expr));
        let chars = string.chars().collect::<Vec<_>>();

        // Reserve space for the string
        ir.alloc(result_reg, chars.len().to_ir_val(), result_kind);

        for (index, c) in chars.into_iter().enumerate()
        {
            ir.write(result_reg, Kind::Char, c.to_ir_val(), index.to_ir_val());
        }
    }

    /// `[1, 2, 3]`
    pub(super) fn generate_vector_shorthand(
        &self,
        expr: &ExprAst<'a>,
        items: &[ExprAst<'a>],
        ir: &mut IrBuilder<EnvEntry, Kind>,
    )
    {
        let (result_reg, result_kind) = (self.register_of(expr), kind_of(expr));

        // Reserve space for the vector
        ir.alloc(result_reg, items.len().to_ir_val(), result_kind);

        for (index, item) in items.iter().enumerate()
        {
            let kind = kind_of(item);
            let item = self.generate_operand(item, ir);

            // Write the value to the vector
            ir.write(result_reg, kind, item, index.to_ir_val());
        }
    }
}
