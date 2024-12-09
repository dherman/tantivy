use std::ops::Deref;
use std::sync::Arc;

use neon::prelude::Context;
use neon::types::Finalize;

pub struct BoxArc<T>(pub Arc<T>);

impl<T> Deref for BoxArc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> Clone for BoxArc<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Finalize for BoxArc<T> {
    fn finalize<'cx, C: Context<'cx>>(self, _: &mut C) {}
}

impl<T> BoxArc<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }

    pub fn clone(other: &Self) -> Self {
        Self(other.0.clone())
    }
}
