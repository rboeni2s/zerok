use anyhow::{Context, Result, anyhow};
use std::{
    cell::RefCell,
    collections::HashMap,
    num::NonZero,
    rc::Rc,
    str::EncodeUtf16,
    sync::atomic::AtomicUsize,
};

use crate::{
    annotator::{Annotation, Kind},
    parser::atom::Atom,
};


#[derive(Default)]
pub struct Store<T>
{
    mem: RefCell<HashMap<usize, Option<T>>>,
    freed: RefCell<Vec<usize>>,
    next_addr: AtomicUsize,
}


impl<T> Store<T>
{
    /// Writes to `val` to  `cell` returning the previous value. Fails if `cell` does not exist
    pub fn write(&self, cell: usize, val: T) -> Result<Option<T>>
    {
        if !self.mem.borrow().contains_key(&cell)
        {
            return Err(anyhow!("Invalid cell {cell:?}"));
        }

        Ok(self.mem.borrow_mut().insert(cell, Some(val)).flatten())
    }

    /// Reserves one cell and returns its address
    pub fn reserve_one(&self) -> usize
    {
        self.next_addr
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Reserves `n` cells and returns the first address
    pub fn reserve_many(&self, n: NonZero<usize>) -> usize
    {
        self.next_addr
            .fetch_add(n.into(), std::sync::atomic::Ordering::Relaxed)
    }

    /// Reads from `cell`, fails if `cell` does not exist
    pub fn read(&self, cell: usize) -> Result<Option<T>>
    where
        T: Clone,
    {
        if self.mem.borrow().contains_key(&cell)
        {
            return Err(anyhow!("Invalid cell {cell:?}"));
        }

        Ok(match self.mem.borrow().get(&cell)
        {
            Some(Some(val)) => Some(val.clone()),
            _ => None,
        })
    }

    #[deprecated = "Einfach durchnummerieren anstatt irgendwas wieder frei zu machen..."]
    pub fn free_cell(&self, cell: usize)
    {
        self.freed.borrow_mut().push(cell);
    }

    // Fetches the next free address
    fn next_addr(&self) -> usize
    {
        match self.freed.borrow_mut().pop()
        {
            Some(addr) => addr,
            None =>
            {
                self.next_addr
                    .fetch_and(1, std::sync::atomic::Ordering::Relaxed)
            }
        }
    }
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
            store: Store {
                mem: Default::default(),
                freed: Default::default(),
                next_addr: Default::default(),
            }
            .into(),
            parent: None,
        }
    }
}


impl<'a, T> Env<'a, T>
{
    /// Reserves one entry in the store, write `entry` to it and then returns its cell index
    pub fn reserve_and_put(&self, entry: T) -> usize
    {
        let cell = self.store.reserve_one();
        self.store.write(cell, entry);
        cell
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
    pub fn fetch_bound(&self, name: &str) -> Option<(Kind, usize)>
    {
        match self.ident_stack.borrow().get(name).copied()
        {
            Some(binding) => Some(binding),
            None =>
            {
                match &self.parent
                {
                    Some(parent) => parent.fetch_bound(name),
                    None => None,
                }
            }
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum EnvEntry
{
    Atom(Atom),
    Register(usize),
}
