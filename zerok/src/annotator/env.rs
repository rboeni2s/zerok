#![allow(clippy::type_complexity)]


use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    annotator::{Annotation, Kind},
    parser::atom::Atom,
};


pub struct Store<T>
{
    mem: RefCell<Vec<T>>,
}


impl<T> Default for Store<T>
{
    fn default() -> Self
    {
        Self {
            mem: Default::default(),
        }
    }
}


impl<T> Store<T>
{
    /// Puts `val` into a new cell and returns the cell
    pub fn put(&self, val: T) -> usize
    {
        let mut mem = self.mem.borrow_mut();
        mem.push(val);
        mem.len() - 1
    }

    /// Reads the value of `cell`, returns `None` if the cell does not exist
    pub fn get(&self, cell: usize) -> Option<T>
    where
        T: Clone,
    {
        self.mem.borrow().get(cell).cloned()
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum EnvEntry
{
    /// A literal, which is used directly as an operand and does not need a register
    Atom(Atom),

    Register(usize),
}


pub struct Env<'a, T>
{
    parent: Option<Rc<Self>>,
    bindings: RefCell<HashMap<&'a str, (Kind, usize)>>,
    functions: Rc<RefCell<HashMap<&'a str, (Kind, Vec<Kind>)>>>,
    loops: Rc<RefCell<Vec<(String, String)>>>, // (Name, Label)
    store: Rc<Store<T>>,

    /// The return type of the function this environment belongs to, `None` outside of functions
    ret: Option<Kind>,
}


impl<'a, T> Default for Env<'a, T>
{
    fn default() -> Self
    {
        let builtins = [("ice_print", (Kind::None, vec![Kind::Char.referenced()]))];

        Self {
            bindings: Default::default(),
            functions: Rc::new(RefCell::new(HashMap::from(builtins))),
            store: Default::default(),
            parent: None,
            ret: None,
            loops: Default::default(),
        }
    }
}


impl<'a, T> Env<'a, T>
{
    /// Puts `entry` into a new store cell and returns the cell
    pub fn put(&self, entry: T) -> usize
    {
        self.store.put(entry)
    }

    /// Gets the entry of `cell`, returns `None` if the cell does not exist
    pub fn get(&self, cell: usize) -> Option<T>
    where
        T: Clone,
    {
        self.store.get(cell)
    }

    /// Writes a function to the environment, if a function by that same name was already present it returns the old one
    pub fn put_function(
        &self,
        name: &'a str,
        ret: Kind,
        args: Vec<Kind>,
    ) -> Option<(Kind, Vec<Kind>)>
    {
        self.functions.borrow_mut().insert(name, (ret, args))
    }

    /// Returns a functions argument and return types, if it exists
    pub fn get_function(&self, name: &'a str) -> Option<(Kind, Vec<Kind>)>
    {
        self.functions.borrow().get(name).cloned()
    }

    /// Gets the entry of `cell`, panics if it does not exist
    pub fn get_unchecked(&self, cell: usize) -> T
    where
        T: Clone,
    {
        self.get(cell)
            .unwrap_or_else(|| panic!("Cell {cell} does not exist in the store"))
    }


    /// Creates child environment of `Self`
    pub fn child_env(self: &Rc<Self>) -> Rc<Self>
    {
        Rc::new(Self {
            parent: Some(self.clone()),
            bindings: Default::default(),
            store: self.store.clone(),
            functions: self.functions.clone(),
            ret: self.ret,
            loops: self.loops.clone(),
        })
    }

    /// Creates the child environment for the body of a function, which returns `ret`
    pub fn function_env(self: &Rc<Self>, ret: Kind) -> Rc<Self>
    {
        Rc::new(Self {
            parent: Some(self.clone()),
            bindings: Default::default(),
            store: self.store.clone(),
            functions: self.functions.clone(),
            ret: Some(ret),
            loops: self.loops.clone(),
        })
    }

    /// The return type of the function this environment belongs to, `None` outside of functions
    pub fn ret(&self) -> Option<Kind>
    {
        self.ret
    }

    /// Binds a store cell to a store entry
    pub fn bind_cell(&self, name: &'a str, annotation: &Annotation)
    {
        self.bindings
            .borrow_mut()
            .insert(name, (annotation.kind, annotation.cell));
    }

    /// Gets the store cell `name`
    pub fn fetch_bound_cell(&self, name: &str) -> Option<(Kind, usize)>
    {
        match self.bindings.borrow().get(name).copied()
        {
            Some(binding) => Some(binding),
            None =>
            {
                match &self.parent
                {
                    Some(parent) => parent.fetch_bound_cell(name),
                    None => None,
                }
            }
        }
    }

    /// Enters a loop and returns its label
    pub fn enter_loop(&self, name: String) -> String
    {
        self.loops
            .borrow_mut()
            .push_mut((name, format!("loop_{}", uuid::Uuid::new_v4().simple())))
            .1
            .to_string()
    }

    /// Checks whether or not the env is in loop `name` and returns its unique label,
    /// `None` refers to the inner most loop, regardless of its name
    pub fn is_in_loop(&self, name: Option<&str>) -> Option<String>
    {
        let loops = self.loops.borrow();

        match name
        {
            Some(name) => loops.iter().rev().find(|(n, _)| n.as_str() == name),
            None => loops.last(),
        }
        .map(|(_, label)| label.to_string())
    }

    /// Exits the inner most loop, returns `Some((loop_name, loop_label))` or `None` if this was called outside a loop
    pub fn exit_loop(&self) -> Option<(String, String)>
    {
        self.loops.borrow_mut().pop()
    }
}
