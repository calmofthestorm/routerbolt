use std::convert::{TryFrom, TryInto};
use std::rc::Rc;

use crate::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionName(Rc<String>);

impl std::fmt::Display for FunctionName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<String> for FunctionName {
    type Error = Error;
    fn try_from(other: String) -> Result<Self> {
        Rc::new(other).try_into()
    }
}

impl TryFrom<&str> for FunctionName {
    type Error = Error;
    fn try_from(other: &str) -> Result<Self> {
        other.to_string().try_into()
    }
}

impl TryFrom<Rc<String>> for FunctionName {
    type Error = Error;
    fn try_from(other: Rc<String>) -> Result<Self> {
        // FIXME: Should probably limit the characters that may be used.
        Ok(FunctionName(other))
    }
}

impl TryFrom<&Rc<String>> for FunctionName {
    type Error = Error;
    fn try_from(other: &Rc<String>) -> Result<Self> {
        other.clone().try_into()
    }
}

impl Into<Rc<String>> for FunctionName {
    fn into(self) -> Rc<String> {
        self.0.clone()
    }
}

impl AsRef<str> for FunctionName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
