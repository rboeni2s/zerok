mod control;
mod conversions;
mod ops;

use crate::{
    Env,
    ExprAst,
    ProgAst,
    annotator::{Annotation, EnvEntry, Kind},
    parser::{Expr, Function, atom::Atom},
    reg_util::Reserved,
};
use ir::{IrBuilder, IrChunk, IrReg};


pub fn generate_ir<'a>(prog: &ProgAst<'a>, env: &Env<'a>) -> Vec<IrChunk<EnvEntry>>
{
    let mut ir = IrBuilder::<EnvEntry>::default();

    for func in &prog.functions
    {
        GenEnv { env, func }.generate_func(&mut ir);
    }

    ir.build()
}


struct GenEnv<'g, 'a>
{
    env: &'g Env<'a>,
    func: &'g Function<'a, Option<Annotation>>,
}


impl<'a> GenEnv<'_, 'a>
{
    fn generate_func(&self, ir: &mut IrBuilder<EnvEntry>)
    {
        let params = self
            .func
            .params
            .iter()
            .map(|param| {
                let cell = param
                    .data
                    .as_ref()
                    .expect("A function parameter must be annotated")
                    .cell;

                match self.env.get_unchecked(cell)
                {
                    EnvEntry::Register(reg) => IrReg::R(reg),
                    EnvEntry::Atom(_) => unreachable!("Parameters are always stored in registers"),
                }
            })
            .collect::<Vec<_>>();

        ir.func(self.func.name, &params); // Genrate function head ir
        self.generate_expr(&self.func.body, ir); // Generate function body ir

        let Some((ret, _)) = self.env.get_function(self.func.name)
        else
        {
            unreachable!("This function must exists because it is already generating ir...")
        };

        let ret_val = {
            // Functions without a return value or body always return 0 (in the IR), so the return register is not needed
            if ret == Kind::None
            {
                // In this case, the function returns a placeholder 0 wich is never allowed to be used
                EnvEntry::Atom(Atom::Int(0))
            }
            else
            {
                // If the function does have a body that evaluates to "something", thena that is written into a register...
                if cell_of(&self.func.body).is_some()
                {
                    //... then move that value to the return register of the function
                    ir.bind(self.entry_of(&self.func.body), Reserved::Return.into());
                }

                // In this case the function returns the return register
                EnvEntry::Register(Reserved::Return.into())
            }
        };

        // Returning is done jumping to this return label (with a return in zk) or reaching it naturally trough the function control flow
        ir.label(self.func.return_label())
            // Maybe some side_effect or deinit code comes here
            .ret(ret_val);
    }


    fn generate_expr(&self, expr: &ExprAst<'a>, ir: &mut IrBuilder<EnvEntry>)
    {
        match &expr.inner
        {
            // Atom: Do not generate for atoms that are unused, because these will never have side effects and will also never be read...
            //
            // Binding: The value of a binding already is in the register of its declaration...
            //
            // So in both cases no ir needs to be generated...
            Expr::Atom(_) | Expr::Binding { .. } =>
            {}

            Expr::Chain(ast_nodes) =>
            {
                for node in ast_nodes
                {
                    self.generate_expr(node, ir);
                }
            }

            Expr::Binop { op, lhs, rhs } => self.generate_binop(expr, op, lhs, rhs, ir),
            Expr::Unaop { op, val } => self.generate_unaop(expr, op, val, ir),

            // The ir is not typed, so a cast only copies the value into the register of the cast
            Expr::Decl { val, .. } | Expr::Cast { val, .. } =>
            {
                let val = self.generate_operand(val, ir);
                ir.bind(val, self.register_of(expr));
            }

            Expr::Assign { target, source } =>
            {
                self.generate_expr(target, ir);
                let source = self.generate_operand(source, ir);
                ir.bind(source, self.register_of(expr));
            }

            Expr::Call { name, args } =>
            {
                let args = args
                    .iter()
                    .map(|arg| self.generate_operand(arg, ir))
                    .collect::<Vec<_>>();

                ir.call(*name, &args, self.register_of(expr));
            }

            Expr::Return { val } => self.generate_return(val.as_deref(), ir),

            Expr::If {
                condition,
                body,
                elifs,
                else_body,
            } => self.generate_if(expr, condition, body, elifs, else_body.as_deref(), ir),

            Expr::Loop { name, head, body } =>
            {
                self.generate_loop(name.as_deref(), head.as_deref(), body, ir)
            }

            Expr::Break { .. } => self.generate_break(expr, ir),
            Expr::Continue { .. } => self.generate_continue(expr, ir),

            Expr::ParseError => unreachable!("The typechecker already denies parser errors"),
        }
    }


    /// Generates the ir for `expr` and returns the entry holding its value, so it can be used as an operand
    fn generate_operand(&self, expr: &ExprAst<'a>, ir: &mut IrBuilder<EnvEntry>) -> EnvEntry
    {
        self.generate_expr(expr, ir);

        // Values of type none (e.g. "keenop") have no cell, they are represented by 0
        match cell_of(expr)
        {
            Some(_) => self.entry_of(expr),
            None => EnvEntry::Atom(Atom::Int(0)),
        }
    }


    /// The store entry holding the value of `expr`
    fn entry_of(&self, expr: &ExprAst<'a>) -> EnvEntry
    {
        self.env.get_unchecked(cell_of(expr))
    }


    /// The register the value of `expr` is stored in
    fn register_of(&self, expr: &ExprAst<'a>) -> usize
    {
        match self.entry_of(expr)
        {
            EnvEntry::Register(reg) => reg,
            EnvEntry::Atom(atom) => panic!("The literal {atom} does not have a register"),
        }
    }
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
