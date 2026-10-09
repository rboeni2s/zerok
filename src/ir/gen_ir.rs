use super::{IrBuilder, IrChunk};
use crate::{
    Ast,
    Env,
    annotator::EnvEntry,
    parser::{Expr, atom::Atom},
};


pub fn generate_ir<'a>(expr: &Ast<'a>, env: &Env<'a>) -> Vec<IrChunk>
{
    let mut builder = IrBuilder::default();
    builder.func("main", &[]);

    generate_expr_ir(expr, env, &mut builder);

    builder.build()
}

fn generate_expr_ir<'a>(expr: &Ast<'a>, env: &Env<'a>, ir: &mut IrBuilder)
{
    let Some(annotation) = &expr.data
    else
    {
        panic!("Trying to generate ir for an ast that has not yet been annotated");
    };

    match &expr.inner
    {
        Expr::Atom(atom) =>
        {
            if matches!(atom, Atom::Nop)
            {
                return;
            }

            let entry = env.get_unchecked(annotation.cell);
            let EnvEntry::Atom { reg, .. } = entry
            else
            {
                panic!("Register EnvEntry assigned to atom!!!")
            };

            ir.bind(entry, reg);
        }

        Expr::Chain(ast_nodes) =>
        {
            for node in ast_nodes
            {
                generate_expr_ir(node, env, ir);
            }
        }

        Expr::Binop { op, lhs, rhs } => todo!(),
        Expr::Unaop { op, val } => todo!(),
        Expr::Decl { name, kind, val } => todo!(),
        Expr::Binding { name } => todo!(),
        Expr::Cast { val, kind } => todo!(),
        Expr::ParseError => todo!(),
    }
}
