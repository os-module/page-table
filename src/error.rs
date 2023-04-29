use core::error::Error;
use core::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum PTableError{
    AllocError,
    NotValid,
}


impl Display for PTableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            PTableError::AllocError => write!(f, "alloc error"),
            PTableError::NotValid => write!(f, "not valid"),
        }
    }
}

impl Error for PTableError{}