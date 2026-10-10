use super::{IrBuilder, IrChunk, IrOp};
use crate::{
    Env,
    ExprAst,
    ProgAst,
    annotator::EnvEntry,
    ir::IrReg,
    parser::{
        Expr,
        atom::Atom,
        ops::{Binop, Unaop},
    },
};


pub fn generate_ir<'a>(prog: &ProgAst<'a>, env: &Env<'a>) -> Vec<IrChunk>
{
    let mut ir = IrBuilder::default();

    for func in &prog.functions
    {
        let params = func
            .params
            .iter()
            .map(|param| {
                let cell = param
                    .data
                    .expect("A function parameter must be annotated")
                    .cell;

                match env.get_unchecked(cell)
                {
                    EnvEntry::Register(reg) => IrReg::R(reg),
                    EnvEntry::Atom(_) => unreachable!("Parameters are always stored in registers"),
                }
            })
            .collect::<Vec<_>>();

        ir.func(func.name, &params);
        generate_expr_ir(&func.body, env, &mut ir);
    }

    ir.build()
}


fn generate_expr_ir<'a>(expr: &ExprAst<'a>, env: &Env<'a>, ir: &mut IrBuilder)
{
    match &expr.inner
    {
        // Do not generate for atoms that are unused, because these will never have side effects and will also never be read
        Expr::Atom(_) =>
        {}

        Expr::Chain(ast_nodes) =>
        {
            for node in ast_nodes
            {
                generate_expr_ir(node, env, ir);
            }
        }

        Expr::Binop { op, lhs, rhs } =>
        {
            let lhs = generate_operand_ir(lhs, env, ir);
            let rhs = generate_operand_ir(rhs, env, ir);

            ir.binop(binop_to_ir(op), lhs, rhs, register_of(expr, env));
        }

        Expr::Unaop { op, val } =>
        {
            let val = generate_operand_ir(val, env, ir);

            ir.unaop(unaop_to_ir(op), val, register_of(expr, env));
        }

        Expr::Decl { val, .. } =>
        {
            // A binding of type none has no value that could be stored
            if cell_of(val).is_none()
            {
                generate_expr_ir(val, env, ir);
                return;
            }

            let val = generate_operand_ir(val, env, ir);

            ir.bind(val, register_of(expr, env));
        }

        // The value of a binding already is in the register of its declaration
        Expr::Binding { .. } =>
        {}

        // The ir is not typed, so a cast only copies the value into the register of the cast
        Expr::Cast { val, .. } =>
        {
            let val = generate_operand_ir(val, env, ir);
            ir.bind(val, register_of(expr, env));
        }

        Expr::Call { name, args } =>
        {
            let args = args
                .iter()
                .map(|arg| generate_operand_ir(arg, env, ir))
                .collect::<Vec<_>>();

            ir.call(*name, &args, register_of(expr, env));
        }

        Expr::ParseError => unreachable!("The typechecker already denies parser errors"),
    }
}


/// Generates the ir for `expr` and returns the entry holding its value, so it can be used as an operand
fn generate_operand_ir<'a>(expr: &ExprAst<'a>, env: &Env<'a>, ir: &mut IrBuilder) -> EnvEntry
{
    generate_expr_ir(expr, env, ir);
    entry_of(expr, env)
}


/// The store cell of an annotated node
fn cell_of(expr: &ExprAst<'_>) -> Option<usize>
{
    let Some(annotation) = &expr.data
    else
    {
        panic!("Trying to generate ir for an ast that has not yet been annotated");
    };

    annotation.cell
}


/// The store entry holding the value of `expr`
fn entry_of<'a>(expr: &ExprAst<'a>, env: &Env<'a>) -> EnvEntry
{
    env.get_unchecked(cell_of(expr))
}


/// The register the value of `expr` is stored in
fn register_of<'a>(expr: &ExprAst<'a>, env: &Env<'a>) -> usize
{
    match entry_of(expr, env)
    {
        EnvEntry::Register(reg) => reg,
        EnvEntry::Atom(atom) => panic!("The literal {atom} does not have a register"),
    }
}


fn binop_to_ir(op: &Binop) -> IrOp
{
    match op
    {
        Binop::Add => IrOp::Add,
        Binop::Sub => IrOp::Sub,
        Binop::Mul => IrOp::Mul,
        Binop::Div => IrOp::Div,

        //TODO: implement loops and conditions in the ir and then implement these
        Binop::Mod => todo!("The mod operator is not implemented in the ir yet"),
        Binop::Pow => todo!("The pow operator is not implemented in the ir yet"),
    }
}


fn unaop_to_ir(op: &Unaop) -> IrOp
{
    match op
    {
        Unaop::Neg => IrOp::Sub,
    }
}
