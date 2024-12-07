use std::cell::{Ref, RefMut};
use std::cell::RefCell;

use neon::prelude::Context;
use neon::types::Finalize;

pub struct BoxCell<T>(pub RefCell<Option<T>>);

impl<T> Finalize for BoxCell<T> {
    fn finalize<'cx, C: Context<'cx>>(self, _: &mut C) {}
}

impl<T> BoxCell<T> {
    pub fn new(value: T) -> Self {
        Self(RefCell::new(Some(value)))
    }

    pub fn as_ref(&self) -> Ref<T> {
        let r: Ref<'_, Option<T>> = self.0.borrow();
        Ref::map(r, |ref_opt: &Option<T>| {
            let opt_ref: Option<&T> = ref_opt.as_ref();
            opt_ref.unwrap()
        })
    }

    pub fn as_mut(&self) -> RefMut<T> {
        let r: RefMut<'_, Option<T>> = self.0.borrow_mut();
        RefMut::map(r, |ref_mut: &mut Option<T>| {
            let opt_mut: Option<&mut T> = ref_mut.as_mut();
            opt_mut.unwrap()
        })
    }

    pub fn take(&self) -> T {
        self.0.take().unwrap()
    }
}
