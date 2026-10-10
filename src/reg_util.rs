use crate::annotator::{Env, EnvEntry};
use std::{
    ops::Deref,
    sync::atomic::{AtomicUsize, Ordering},
};


const FUNCTION_REGISTER_RANGE_START: usize = u8::MAX as usize + 1;
static REGISTER: AtomicUsize = AtomicUsize::new(FUNCTION_REGISTER_RANGE_START);


#[repr(u8)]
pub enum Reserved
{
    Return,
}


impl Into<usize> for Reserved
{
    fn into(self) -> usize
    {
        self as u8 as usize
    }
}


/// Puts a new register into the store and returns its cell
pub fn new_register(env: &Env<'_, EnvEntry>) -> usize
{
    env.put(EnvEntry::Register(next_register()))
}


/// Returns a new, unused register
pub fn next_register() -> usize
{
    REGISTER.fetch_add(1, Ordering::Relaxed)
}


/// Resets the register counter
pub fn reset_register()
{
    REGISTER.store(FUNCTION_REGISTER_RANGE_START, Ordering::Relaxed);
}
