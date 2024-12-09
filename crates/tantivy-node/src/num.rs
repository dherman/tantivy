use std::fmt;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
pub struct u53(u64);

impl u53 {
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(0x1fffffffffffff_u64);

    const fn new(x: u64) -> u53 {
        u53(x)
    }
}

impl fmt::Display for u53 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for u53 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

pub trait Project<T>: From<T> {
    fn project(self) -> Option<T>;
}

impl From<u53> for f64 {
    fn from(x: u53) -> f64 {
        x.0 as f64
    }
}

impl From<u53> for u64 {
    fn from(x: u53) -> u64 {
        x.0
    }
}

impl Project<u53> for f64 {
    fn project(self) -> Option<u53> {
        if self.trunc() != self {
            return None;
        }
        if self < u53::MIN.into() || self > u53::MAX.into() {
            return None;
        }
        return Some(u53::new(self as u64));
    }
}

macro_rules! impl_project {
    ($t:ident) => {
        impl Project<$t> for f64 {
            fn project(self) -> Option<$t> {
                if self.trunc() != self {
                    return None;
                }
                if self < $t::MIN.into() || self > $t::MAX.into() {
                    return None;
                }
                return Some(self as $t);
            }
        }
    }
}

impl_project!(u32);
impl_project!(u16);
impl_project!(u8);
