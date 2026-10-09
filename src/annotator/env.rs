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
    Atom
    {
        atom: Atom,
        reg: usize,
    },

    Register(usize),
}


pub struct Env<'a, T>
{
    parent: Option<Rc<Self>>,
    ident_stack: RefCell<HashMap<&'a str, (Kind, usize)>>,
    store: Rc<Store<T>>,
}


impl<'a, T> Default for Env<'a, T>
{
    fn default() -> Self
    {
        Self {
            ident_stack: Default::default(),
            store: Default::default(),
            parent: None,
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

    /// Gets the entry of `cell`, panics if there is no cell or it does not exist
    pub fn get_unchecked(&self, cell: Option<usize>) -> T
    where
        T: Clone,
    {
        let cell = cell.expect("Trying to read an entry without a cell");

        self.get(cell)
            .unwrap_or_else(|| panic!("Cell {cell} does not exist in the store"))
    }


    /// Creates child environment of `Self`
    pub fn child_env(self: &Rc<Self>) -> Rc<Self>
    {
        Rc::new(Self {
            parent: Some(self.clone()),
            ident_stack: Default::default(),
            store: self.store.clone(),
        })
    }

    /// Binds a store cell to a store entry
    pub fn bind_cell(&self, name: &'a str, annotation: &Annotation)
    {
        self.ident_stack.borrow_mut().insert(
            name,
            (
                annotation.kind,
                annotation
                    .cell
                    .expect("Malformed annotation without a store cell bound to a binding"),
            ),
        );
    }

    /// Gets the store cell `name`
    pub fn fetch_bound_cell(&self, name: &str) -> Option<(Kind, usize)>
    {
        match self.ident_stack.borrow().get(name).copied()
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
}
