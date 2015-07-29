// Copyright 2015 Jerome Rasky <jerome@rasky.co>
//
// Licensed under the Apache License, version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at
//
//     <http://www.apache.org/licenses/LICENSE-2.0>
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either expressed or implied. See the
// License for the specific language concerning governing permissions and
// limitations under the License.
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
