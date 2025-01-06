use std::sync::{LockResult, Mutex, MutexGuard};

use neon::prelude::Context;
use neon::types::Finalize;

pub struct BoxMutex<T>(pub Mutex<T>);

impl<T: Clone> Clone for BoxMutex<T> {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.0.lock().unwrap().clone()))
    }
}

// impl<T> Clone for BoxMutex<T> {
//     fn clone(&self) -> Self {
//         Self(self.0.clone())
//     }
// }

impl<T> Finalize for BoxMutex<T> {
    fn finalize<'cx, C: Context<'cx>>(self, _: &mut C) {}
}

impl<T> BoxMutex<T> {
    pub fn new(value: T) -> Self {
        Self(Mutex::new(value))
    }

    pub fn lock(&self) -> LockResult<MutexGuard<T>> {
        self.0.lock()
    }
    // pub fn clone(other: &Self) -> Self {
    //     Self(other.0.clone())
    // }
}
