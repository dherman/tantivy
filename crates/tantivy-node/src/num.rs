use std::fmt::{self, Display};

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

#[derive(Debug)]
pub struct ProjectionError(pub String);

pub trait Project<T>: From<T> {
    fn project(self) -> Result<T, ProjectionError>;
}

impl Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ProjectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        &self.0
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
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
    fn project(self) -> Result<u53, ProjectionError> {
        if self.trunc() != self {
            return Err(ProjectionError(format!("{self} is not an integer")));
        }
        if self < u53::MIN.into() || self > u53::MAX.into() {
            return Err(ProjectionError(format!("{self} is out of range for u53")));
        }
        return Ok(u53::new(self as u64));
    }
}

macro_rules! impl_project {
    ($t:ident) => {
        impl Project<$t> for f64 {
            fn project(self) -> Result<$t, ProjectionError> {
                if self.trunc() != self {
                    return Err(ProjectionError(format!("{self} is not an integer")));
                }
                if self < $t::MIN.into() || self > $t::MAX.into() {
                    return Err(ProjectionError(format!("{self} is out of range for {}", stringify!($t))));
                }
                return Ok(self as $t);
            }
        }
    }
}

impl_project!(u32);
impl_project!(u16);
impl_project!(u8);
