use crate::annotator::{Env, EnvEntry};
use std::sync::atomic::{AtomicUsize, Ordering};


const REGISTER_RANGE_START: usize = u8::MAX as usize + 1;
static REGISTER: AtomicUsize = AtomicUsize::new(REGISTER_RANGE_START);
static GLOBAL_REGISTER: AtomicUsize = AtomicUsize::new(REGISTER_RANGE_START);


#[repr(u8)]
pub enum FnReserved
{
    Return,
}


impl From<FnReserved> for usize
{
    fn from(val: FnReserved) -> Self
    {
        val as u8 as usize
    }
}


#[repr(u8)]
pub enum GbReserved
{
    /// This symbols is a extern symbol.
    Extern,
}


impl From<GbReserved> for usize
{
    fn from(val: GbReserved) -> Self
    {
        val as u8 as usize
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
    REGISTER.store(REGISTER_RANGE_START, Ordering::Relaxed);
}


/// Returns a new, unused register
pub fn next_global_register() -> usize
{
    GLOBAL_REGISTER.fetch_add(1, Ordering::Relaxed)
}


/// Resets the register counter
pub fn reset_global_register()
{
    GLOBAL_REGISTER.store(REGISTER_RANGE_START, Ordering::Relaxed);
}
