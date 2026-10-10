use super::{IrBuilder, IrChunk, IrOp};
use crate::{
    Ast,
    Env,
    annotator::EnvEntry,
    parser::{
        Expr,
        atom::Atom,
        ops::{Binop, Unaop},
    },
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
    match &expr.inner
    {
        Expr::Atom(atom) =>
        {
            if matches!(atom, Atom::Nop)
            {
                return;
            }

            let entry = entry_of(expr, env);
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

        Expr::Call { .. } => todo!("Function calls are not implemented in the ir yet"),

        Expr::ParseError => unreachable!("The typechecker does not accept asts with parse errors"),
    }
}


/// Generates the ir for `expr` and returns the entry holding its value, so it can be used as an operand.
/// Literals do not need any ir, they are used directly as operands
fn generate_operand_ir<'a>(expr: &Ast<'a>, env: &Env<'a>, ir: &mut IrBuilder) -> EnvEntry
{
    if !matches!(expr.inner, Expr::Atom(_))
    {
        generate_expr_ir(expr, env, ir);
    }

    entry_of(expr, env)
}


/// The store cell of an annotated node
fn cell_of(expr: &Ast<'_>) -> Option<usize>
{
    let Some(annotation) = &expr.data
    else
    {
        panic!("Trying to generate ir for an ast that has not yet been annotated");
    };

    annotation.cell
}


/// The store entry holding the value of `expr`
fn entry_of<'a>(expr: &Ast<'a>, env: &Env<'a>) -> EnvEntry
{
    env.get_unchecked(cell_of(expr))
}


/// The register the value of `expr` is stored in
fn register_of<'a>(expr: &Ast<'a>, env: &Env<'a>) -> usize
{
    match entry_of(expr, env)
    {
        EnvEntry::Register(reg) | EnvEntry::Atom { reg, .. } => reg,
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
        // The ir has no mod and pow operators, they have to be built from other chunks (e.g. a loop for pow)
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
