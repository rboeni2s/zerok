use super::{Annotation, Env, EnvEntry, Kind};
use crate::parser::{Function, Node, Program};
use chumsky::span::SimpleSpan;
use std::rc::Rc;


impl<'a> Program<'a, Option<Annotation>>
{
    /// Annotates all functions of this program.
    pub fn annotate(&mut self, env: &Rc<Env<'a, EnvEntry>>) -> Result<(), (SimpleSpan, String)>
    {
        for function in &self.functions
        {
            let (ret, params) = function.signature()?;

            if env.put_function(function.name, ret, params).is_some()
            {
                return Err({
                    let span = function.span;
                    let msg = format!("Function {:?} is defined more than once", function.name);
                    (span, msg.to_string())
                });
            }
        }

        self.check_main()?;

        for function in &mut self.functions
        {
            function.annotate(env)?;
        }

        Ok(())
    }

    /// Makes sure there is a main function, which does not take any parameters
    fn check_main(&self) -> Result<(), (SimpleSpan, String)>
    {
        let Some(main) = self
            .functions
            .iter()
            .find(|function| function.name == "main")
        else
        {
            return Err({
                let span = SimpleSpan::from(0..0);
                (span, "Missing main function".to_string())
            });
        };

        if !main.params.is_empty()
        {
            return Err({
                let span = main.span;
                (
                    span,
                    "The main function can not take any parameters".to_string(),
                )
            });
        }

        Ok(())
    }
}


impl<'a> Function<'a, Option<Annotation>>
{
    /// The return type and parameter types of this function
    fn signature(&self) -> Result<(Kind, Vec<Kind>), (SimpleSpan, String)>
    {
        // A function without a return type does not return anything
        let ret = match self.ret
        {
            Some(ret) =>
            {
                Kind::from_str(ret).ok_or_else(|| {
                    let span = self.span;
                    let msg = format!("Unknown type {ret:?}");
                    (span, msg.to_string())
                })?
            }
            None => Kind::None,
        };

        let params = self
            .params
            .iter()
            .map(|param| {
                Kind::from_str(param.kind).ok_or_else(|| {
                    let span = param.span;
                    let msg = format!("Unknown type {:?}", param.kind);
                    (span, msg.to_string())
                })
            })
            .collect::<Result<_, _>>()?;

        Ok((ret, params))
    }

    /// Annotates the parameters and body of this function.
    /// The signature has to be in `env` already, if it does not then thats a programmer error
    fn annotate(&mut self, env: &Rc<Env<'a, EnvEntry>>) -> Result<(), (SimpleSpan, String)>
    {
        let (ret, params) = env
            .get_function(self.name)
            .expect("The signature of a function has to be collected before its body is annotated");

        // Every function has its own env, so it can only see its own parameters
        let fn_env = env.child_env();

        for (param, kind) in self.params.iter_mut().zip(params)
        {
            if fn_env.fetch_bound_cell(param.name).is_some()
            {
                return Err({
                    let span = param.span;
                    let msg = format!("Parameter {:?} is defined more than once", param.name);
                    (span, msg.to_string())
                });
            }

            let annotation = Annotation {
                kind,
                cell: Some(fn_env.put(EnvEntry::Register(Node::reg()))),
            };

            fn_env.bind_cell(param.name, &annotation);
            param.data = Some(annotation);
        }

        let body = self.body.annotate_expecting(&fn_env, Some(ret))?;

        if body.kind != ret
        {
            return Err({
                let span = self.body.span;
                let msg = format!(
                    "Function {:?} has to return {}, but its body returns {}",
                    self.name, ret, body.kind
                );
                (span, msg.to_string())
            });
        }

        Ok(())
    }
}
