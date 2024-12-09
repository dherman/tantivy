use std::sync::{Arc, LockResult, Mutex, MutexGuard};

use neon::prelude::Context;
use neon::types::Finalize;

pub struct BoxArc<T>(pub Arc<Mutex<T>>);

impl<T> Finalize for BoxArc<T> {
    fn finalize<'cx, C: Context<'cx>>(self, _: &mut C) {}
}

impl<T> BoxArc<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        self.0.lock()
    }

    pub fn clone(other: &Self) -> Self {
        Self(other.0.clone())
    }
}
