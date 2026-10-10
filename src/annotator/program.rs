use super::{Annotation, Env, EnvEntry, Kind, new_register};
use crate::{
    parser::{Function, Program},
    reg_util,
};
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
                return err!(
                    function.span,
                    "Function {:?} is defined more than once",
                    function.name
                );
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
            return err!(SimpleSpan::from(0..0), "Missing main function");
        };

        if !main.params.is_empty()
        {
            return err!(main.span, "The main function can not take any parameters");
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
                Kind::from_str(ret).ok_or_else(|| (self.span, format!("Unknown type {ret:?}")))?
            }
            None => Kind::None,
        };

        let params = self
            .params
            .iter()
            .map(|param| {
                match Kind::from_str(param.kind)
                {
                    Some(Kind::None) =>
                    {
                        Err((param.span, "Parameters can not be of type none".into()))
                    }
                    Some(kind) => Ok(kind),
                    None => Err((param.span, format!("Unknown type {:?}", param.kind))),
                }
            })
            .collect::<Result<_, _>>()?;

        Ok((ret, params))
    }

    /// Annotates the parameters and body of this function.
    /// The signature has to be in `env` already, if it does not then thats a programmer error
    fn annotate(&mut self, env: &Rc<Env<'a, EnvEntry>>) -> Result<(), (SimpleSpan, String)>
    {
        reg_util::reset_register();

        let (ret, params) = env
            .get_function(self.name)
            .expect("The signature of a function has to be collected before its body is annotated");

        // Every function has its own env, so it can only see its own parameters
        let fn_env = env.function_env(ret);

        for (param, kind) in self.params.iter_mut().zip(params)
        {
            if fn_env.fetch_bound_cell(param.name).is_some()
            {
                return err!(
                    param.span,
                    "Parameter {:?} is defined more than once",
                    param.name
                );
            }

            let annotation = Annotation {
                kind,
                cell: Some(new_register(&fn_env)),
            };

            fn_env.bind_cell(param.name, &annotation);
            param.data = Some(annotation);
        }

        let body = self.body.annotate_expecting(&fn_env, Some(ret))?;

        // The value of a body that ends in a return is never used, e.g. "{ geve 1; }"
        if body.kind != ret && !self.body.ends_with_return()
        {
            return err!(
                self.body.span,
                "Function {:?} has to return {}, but its body returns {}",
                self.name,
                ret,
                body.kind
            );
        }

        Ok(())
    }
}
