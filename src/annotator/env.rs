use anyhow::{Context, Result, anyhow};
use std::{cell::RefCell, collections::HashMap, num::NonZero, sync::atomic::AtomicUsize};

use crate::parser::atom::Atom;


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
        let addr = self
            .next_addr
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        addr
    }

    /// Reserves `n` cells and returns the first address
    pub fn reserve_many(&self, n: NonZero<usize>) -> usize
    {
        let addr = self
            .next_addr
            .fetch_add(n.into(), std::sync::atomic::Ordering::Relaxed);

        addr
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

    #[deprecated = "Einfach durchnummerieren..."]
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
            None => self
                .next_addr
                .fetch_and(1, std::sync::atomic::Ordering::Relaxed),
        }
    }
}


pub struct Env<'a, T>
{
    ident_stack: Vec<(&'a str, T)>,
    store: Store<T>,
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
            },
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
}


pub enum EnvEntry
{
    Atom(Atom),
    Register(usize),
}
