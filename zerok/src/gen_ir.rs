use crate::{
    Env,
    ExprAst,
    ProgAst,
    annotator::{Annotation, Effect, EnvEntry, Kind},
    parser::{
        Expr,
        Function,
        atom::Atom,
        ops::{Binop, Unaop},
    },
    reg_util::{self, Reserved},
};
use iic_quote::iic_quote;
use ir::{IrBuilder, IrOp, IrReg, ToIIC};


impl ToIIC for EnvEntry
{
    fn to_iic(&self) -> String
    {
        match self
        {
            EnvEntry::Register(reg) => format!("R{reg}"),
            EnvEntry::Atom(atom) =>
            {
                match atom
                {
                    Atom::Str(_) =>
                    {
                        todo!(
                            "IIC string repräsentationen sind noch nicht implementiert, aber wahrscheinlich [char]"
                        )
                    }
                    Atom::Int(num) => num.to_string(),
                    Atom::Float(num) => (*num as u64).to_string(),
                    Atom::Bool(b) => u64::from(*b).to_string(),
                    Atom::Nop => unreachable!("Für ein nop sollte nie code generiert werden"),
                }
            }
        }
    }
}

pub fn generate_ir<'a>(prog: &ProgAst<'a>, env: &Env<'a>) -> Vec<ir::IrChunk<EnvEntry>>
{
    let mut ir = IrBuilder::<EnvEntry>::default();

    for func in &prog.functions
    {
        let params = func
            .params
            .iter()
            .map(|param| {
                let cell = param
                    .data
                    .as_ref()
                    .expect("A function parameter must be annotated")
                    .cell;

                match env.get_unchecked(cell)
                {
                    EnvEntry::Register(reg) => IrReg::R(reg),
                    EnvEntry::Atom(_) => unreachable!("Parameters are always stored in registers"),
                }
            })
            .collect::<Vec<_>>();

        ir.func(func.name, &params); // Genrate function head ir
        generate_expr_ir(&func.body, env, &mut ir, func); // Generate function body ir

        let Some((ret, _)) = env.get_function(func.name)
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
                // If the function does have a body that evaluates to "something" that is written into a register...
                if cell_of(&func.body).is_some()
                {
                    //... then move that value to the return register of the function
                    ir.bind(entry_of(&func.body, env), Reserved::Return.into());
                }

                // In this case the function returns the return register
                EnvEntry::Register(Reserved::Return.into())
            }
        };

        // Returning is done jumping to this return label (with a return in zk) or reaching it naturally trough the function control flow
        ir.label(func.return_label())
            // Maybe some side_effect or deinit code comes here
            .ret(ret_val);
    }

    ir.build()
}


fn generate_expr_ir<'a>(
    expr: &ExprAst<'a>,
    env: &Env<'a>,
    ir: &mut IrBuilder<EnvEntry>,
    func: &Function<'a, Option<Annotation>>,
)
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
                generate_expr_ir(node, env, ir, func);
            }
        }

        Expr::Binop { op, lhs, rhs } =>
        {
            let lhs = generate_operand_ir(lhs, env, ir, func);
            let rhs = generate_operand_ir(rhs, env, ir, func);
            let reg = register_of(expr, env);

            if matches!(op, Binop::Mod | Binop::Pow)
                && expr
                    .data
                    .as_ref()
                    .is_some_and(|annotation| annotation.kind.is_float())
            {
                todo!("The {op:?} operator is not implemented for floats in the ir yet");
            }

            match op
            {
                // a % b = a - (a / b) * b
                Binop::Mod =>
                {
                    let res = EnvEntry::Register(reg);

                    ir.binop(IrOp::Div, lhs.clone(), rhs.clone(), reg)
                        .binop(IrOp::Mul, res.clone(), rhs, reg)
                        .binop(IrOp::Sub, lhs, res, reg);
                }

                Binop::Pow => generate_pow_ir(lhs, rhs, reg, ir, func),

                op =>
                {
                    ir.binop(binop_to_ir(op), lhs, rhs, reg);
                }
            }
        }

        Expr::Unaop { op, val } =>
        {
            let val = generate_operand_ir(val, env, ir, func);

            ir.unaop(unaop_to_ir(op), val, register_of(expr, env));
        }

        Expr::Decl { val, .. } =>
        {
            let val = generate_operand_ir(val, env, ir, func);

            ir.bind(val, register_of(expr, env));
        }

        // The ir is not typed, so a cast only copies the value into the register of the cast
        Expr::Cast { val, .. } =>
        {
            let val = generate_operand_ir(val, env, ir, func);
            ir.bind(val, register_of(expr, env));
        }

        Expr::Call { name, args } =>
        {
            let args = args
                .iter()
                .map(|arg| generate_operand_ir(arg, env, ir, func))
                .collect::<Vec<_>>();

            ir.call(*name, &args, register_of(expr, env));
        }

        Expr::Return { val } =>
        {
            if let Some(val) = val
            {
                let val = generate_operand_ir(val, env, ir, func);
                ir.bind(val, Reserved::Return.into());
            }

            ir.jump(func.return_label());
        }

        Expr::If {
            condition,
            body,
            elifs,
            else_body,
        } =>
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


            let reg = cell_of(expr).map(|_| register_of(expr, env));
            let mut body_builder = IrBuilder::default();

            let label_body = &func.unique_label();
            let label_finished = &func.unique_label();
            let label_else = &func.unique_label();

            /*
                Der Code ist SEHR repetativ, ich lasse ihn aber trotzdem
                so, weil er dann vom verständnis her, näher am im kommentar beschriebenen
                muster IR für if/elif/else verzweigungen ist...
            */

            // if condition and body
            let condition = generate_operand_ir(condition, env, ir, func);
            ir.conditional_jump(condition, label_body);
            body_builder.label(label_body);
            let body = generate_operand_ir(body, env, &mut body_builder, func);

            // Wenn dieses if statement als expression mit einem wert benutzt wird, muss der
            // wert dieses zweiges in das ergebnis register geschrieben werden
            if let Some(reg) = reg
            {
                body_builder.bind(body, reg);
            }

            // Sprung zu finished, wenn der körper ausgeführt wurde
            body_builder.jump(label_finished);

            // jetzt das selbe wie für die if/if_body aber für jedes elif
            for (cond, body) in elifs
            {
                let label_body = &func.unique_label();
                let condition = generate_operand_ir(cond, env, ir, func);
                ir.conditional_jump(condition, label_body);
                body_builder.label(label_body);
                let body = generate_operand_ir(body, env, &mut body_builder, func);
                if let Some(reg) = reg
                {
                    body_builder.bind(body, reg);
                }

                body_builder.jump(label_finished);
            }

            // else body mit label
            body_builder.label(label_else);
            if let Some(else_body) = else_body
            {
                let else_body = generate_operand_ir(else_body, env, &mut body_builder, func);
                if let Some(reg) = reg
                {
                    body_builder.bind(else_body, reg);
                }
            }

            ir.jump(label_else)
                .append(body_builder.build())
                .label(label_finished);
        }

        Expr::Loop { name, head, body } =>
        {
            // HEAD: head
            // A: Loop Start
            // START_LABEL: Loop Body
            // CONTINUE_LABEL: goto START_LABEL
            // DONE_LABEL: ...

            // fetch this loops label
            let label = name
                .as_ref()
                .expect("I am inside a loop or the typechecker broke");

            // use the loops label instead of `func.unique_label()` for all of these,
            // even thought it is actually just needed for return and continue. This way all
            // the loops labels have a consistent naming scheme. (But ofc this does not matter for the start label...)
            let start_label = &format!("start_{label}");
            let continue_label = &format!("continue_{label}");
            let done_label = &format!("done_{label}");

            if let Some(head) = head
            {
                generate_expr_ir(head, env, ir, func);
            }


            ir.label(start_label)
                .within(|ir| generate_expr_ir(body, env, ir, func))
                .append(iic_quote! {
                    continue_label:
                        goto start_label;
                    done_label:
                });
        }

        Expr::Break { name: _ } =>
        {
            // fetch the label of the loop, that should be "breaked"
            let Effect::Break { ref label } = expr.data.as_ref().expect("This is annotated").effect
            else
            {
                unreachable!("This break must have a break effect or the typechecker broke")
            };

            ir.jump(format!("done_{label}"));
        }

        Expr::Continue { name: _ } =>
        {
            // fetch the label of the loop, that should be continued
            let Effect::Continue { ref label } =
                expr.data.as_ref().expect("This is annotated").effect
            else
            {
                unreachable!("This continue must have a continue effect or the typechecker broke")
            };

            ir.jump(format!("continue_{label}"));
        }

        Expr::Assign { target, source } =>
        {
            generate_expr_ir(target, env, ir, func);
            let source = generate_operand_ir(source, env, ir, func);
            ir.bind(source, register_of(expr, env));
        }

        Expr::ParseError => unreachable!("The typechecker already denies parser errors"),
    }
}


/// Generates the ir for `expr` and returns the entry holding its value, so it can be used as an operand
fn generate_operand_ir<'a>(
    expr: &ExprAst<'a>,
    env: &Env<'a>,
    ir: &mut IrBuilder<EnvEntry>,
    func: &Function<'a, Option<Annotation>>,
) -> EnvEntry
{
    generate_expr_ir(expr, env, ir, func);

    // Values of type none (e.g. "keenop") have no cell, they are represented by 0
    match cell_of(expr)
    {
        Some(_) => entry_of(expr, env),
        None => EnvEntry::Atom(Atom::Int(0)),
    }
}


fn generate_pow_ir<'a>(
    base: EnvEntry,
    exp: EnvEntry,
    reg: usize,
    ir: &mut IrBuilder<EnvEntry>,
    func: &Function<'a, Option<Annotation>>,
)
{
    let loop_label = &func.unique_label();
    let end_label = &func.unique_label();

    let counter = reg_util::next_register();
    let done = reg_util::next_register();

    ir.append(iic_quote! {
        R(reg) = EnvEntry::Atom(Atom::Int(1));
        R(counter) = exp;

        loop_label:
            R(done) = EnvEntry::Register(counter) <= EnvEntry::Atom(Atom::Int(0));
            if EnvEntry::Register(done) goto end_label;

            R(reg) = EnvEntry::Register(reg) * base;
            R(counter) = EnvEntry::Register(counter) - EnvEntry::Atom(Atom::Int(1));
            goto loop_label;
        end_label:
    });
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
        Binop::Eq => IrOp::Eq,
        Binop::Neq => IrOp::Neq,
        Binop::Lt => IrOp::Lt,
        Binop::Gt => IrOp::Gt,
        Binop::Lte => IrOp::Lte,
        Binop::Gte => IrOp::Gte,

        Binop::Mod | Binop::Pow => unreachable!("{op:?} is built from multiple chunks"),
    }
}


fn unaop_to_ir(op: &Unaop) -> IrOp
{
    match op
    {
        Unaop::Neg => IrOp::Sub,
    }
}
