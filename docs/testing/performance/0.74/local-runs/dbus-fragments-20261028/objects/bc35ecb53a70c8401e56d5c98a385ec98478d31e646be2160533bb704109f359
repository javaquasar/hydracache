use std::{
    any::Any,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::runtime::locks::{RwLockReadGuard, RwLockWriteGuard};

use super::Interface;

/// Opaque structure that derefs to an `Interface` type.
pub struct InterfaceDeref<'d, I> {
    pub(super) iface: RwLockReadGuard<'d, dyn Interface>,
    pub(super) phantom: PhantomData<I>,
}

impl<I> Deref for InterfaceDeref<'_, I>
where
    I: Interface,
{
    type Target = I;

    fn deref(&self) -> &I {
        <dyn Any>::downcast_ref::<I>(&*self.iface).expect("Unexpected interface type")
    }
}

/// Opaque structure that mutably derefs to an `Interface` type.
pub struct InterfaceDerefMut<'d, I> {
    pub(super) iface: RwLockWriteGuard<'d, dyn Interface>,
    pub(super) phantom: PhantomData<I>,
}

impl<I> Deref for InterfaceDerefMut<'_, I>
where
    I: Interface,
{
    type Target = I;

    fn deref(&self) -> &I {
        <dyn Any>::downcast_ref::<I>(&*self.iface).expect("Unexpected interface type")
    }
}

impl<I> DerefMut for InterfaceDerefMut<'_, I>
where
    I: Interface,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        <dyn Any>::downcast_mut::<I>(&mut *self.iface).expect("Unexpected interface type")
    }
}
