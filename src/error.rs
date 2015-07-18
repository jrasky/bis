use std::error::Error;
use std::borrow::Borrow;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub struct StringError {
    description: String,
    cause: Option<Box<Error>>
}

impl Display for StringError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}

impl Error for StringError {
    fn description(&self) -> &str {
        self.description.as_ref()
    }

    fn cause(&self) -> Option<&Error> {
        match self.cause {
            None => None,
            Some(ref error) => Some(error.borrow())
        }
    }
}

impl StringError {
    pub fn new<T: Into<String>>(description: T, cause: Option<Box<Error>>) -> StringError {
        StringError {
            description: description.into(),
            cause: cause
        }
    }
}
