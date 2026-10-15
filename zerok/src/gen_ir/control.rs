use super::{GenEnv, kind_of};
use crate::{
    ExprAst,
    annotator::{Effect, EnvEntry, Kind},
    reg_util::Reserved,
};
use iic_quote::iic_quote;
use ir::IrBuilder;
use std::iter;


impl<'a> GenEnv<'_, 'a>
{
    pub(super) fn generate_return(
        &self,
        val: Option<&ExprAst<'a>>,
        ir: &mut IrBuilder<EnvEntry, Kind>,
    )
    {
        if let Some(val) = val
        {
            let val = self.generate_operand(val, ir);
            ir.bind(val, Reserved::Return.into(), self.return_kind());
        }

        ir.jump(self.func.return_label());
    }


    pub(super) fn generate_if(
        &self,
        expr: &ExprAst<'a>,
        condition: &ExprAst<'a>,
        body: &ExprAst<'a>,
        elifs: &[(ExprAst<'a>, ExprAst<'a>)],
        else_body: Option<&ExprAst<'a>>,
        ir: &mut IrBuilder<EnvEntry, Kind>,
    )
    {
        // COND: IF COND
        //    if COND GOTO BODY
        //
        // B: ELIF COND
        //    if cond GOTO B_BODY
        //
        // C: ELIF COND
        //    if cond GOTO C_BODY
        //
        // GOTO ELSE_BODY
        //
        // BODY: IF BODY
        //    GOTO FINISHED
        //
        // B_BODY: ELIF-B BODY
        //    GOTO FINISHED
        //
        // C_BODY: ELIF-C BODY
        //    GOTO FINISHED
        //
        // ELSE_BODY: ELSE BODY
        //
        // FINISHED: ...


        // Der wert jedes zweiges wird in das ergebnis register geschrieben. Hat das if keinen
        // wert (z.B. ohne else), ist das register vom typ none und kann wegoptimiert werden
        let reg = self.register_of(expr);
        let kind = kind_of(expr);

        // IR builder für den körper der zweige
        let mut body_builder = IrBuilder::default();

        let label_finished = &self.func.unique_label();
        let label_else = &self.func.unique_label();

        // Vereignigt den if kopf und körper mit denen der elifs,
        // zu einem iterator mit [(condition, body), (condition2, body2), ...]
        let branches =
            iter::once((condition, body)).chain(elifs.iter().map(|(cond, body)| (cond, body)));

        for (condition, body) in branches
        {
            let label_body = &self.func.unique_label();

            let condition = self.generate_operand(condition, ir);
            ir.conditional_jump(condition, label_body);

            body_builder.label(label_body);
            {
                let val = self.generate_operand(body, &mut body_builder);

                body_builder.bind(val, reg, kind);
            };

            body_builder.jump(label_finished);
        }

        // else body mit label
        body_builder.label(label_else);
        if let Some(else_body) = else_body
        {
            {
                let val = self.generate_operand(else_body, &mut body_builder);
                body_builder.bind(val, reg, kind);
            };
        }

        ir.jump(label_else)
            .append(body_builder.build())
            .label(label_finished);
    }

    pub(super) fn generate_loop(
        &self,
        name: Option<&str>,
        head: Option<&ExprAst<'a>>,
        body: &ExprAst<'a>,
        ir: &mut IrBuilder<EnvEntry, Kind>,
    )
    {
        // HEAD: head
        // A: Loop Start
        // START_LABEL: Loop Body
        // CONTINUE_LABEL: goto START_LABEL
        // DONE_LABEL: ...

        // fetch this loops label
        let label = name.expect("I am inside a loop or the typechecker broke");

        // use the loops label instead of `func.unique_label()` for all of these,
        // even thought it is actually just needed for return and continue. This way all
        // the loops labels have a consistent naming scheme. (But ofc this does not matter for the start label...)
        let start_label = &format!("start_{label}");
        let continue_label = &format!("continue_{label}");
        let done_label = &format!("done_{label}");

        if let Some(head) = head
        {
            self.generate_expr(head, ir);
        }

        ir.label(start_label)
            .within(|ir| self.generate_expr(body, ir))
            .append(iic_quote! {
                continue_label:
                    goto start_label;
                done_label:
            });
    }


    /// Jumps to the end of the loop, that `expr` (a break) refers to
    pub(super) fn generate_break(&self, expr: &ExprAst<'a>, ir: &mut IrBuilder<EnvEntry, Kind>)
    {
        let Effect::Break { label } = effect_of(expr)
        else
        {
            unreachable!("This break must have a break effect or the typechecker broke")
        };

        ir.jump(format!("done_{label}"));
    }


    /// Jumps to the continue label of the loop, that `expr` (a continue) refers to
    pub(super) fn generate_continue(&self, expr: &ExprAst<'a>, ir: &mut IrBuilder<EnvEntry, Kind>)
    {
        let Effect::Continue { label } = effect_of(expr)
        else
        {
            unreachable!("This continue must have a continue effect or the typechecker broke")
        };

        ir.jump(format!("continue_{label}"));
    }
}


fn effect_of<'e>(expr: &'e ExprAst<'_>) -> &'e Effect
{
    &expr.data.as_ref().expect("This is annotated").effect
}
