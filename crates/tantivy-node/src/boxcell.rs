use std::cell::{Ref, RefMut};
use std::cell::RefCell;

use neon::prelude::Context;
use neon::types::Finalize;

#[derive(Clone)]
pub struct BoxCell<T>(pub RefCell<T>);

impl<T> Finalize for BoxCell<T> {
    fn finalize<'cx, C: Context<'cx>>(self, _: &mut C) {}
}

impl<T> BoxCell<T> {
    pub fn new(value: T) -> Self {
        Self(RefCell::new(value))
    }

    pub fn as_ref(&self) -> Ref<T> {
        self.0.borrow()
    }

    pub fn as_mut(&self) -> RefMut<T> {
        self.0.borrow_mut()
    }
}
