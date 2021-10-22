use std::convert::{TryFrom, TryInto};
use std::rc::Rc;

use crate::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LabelName(Rc<String>);

impl std::fmt::Display for LabelName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<String> for LabelName {
    type Error = Error;
    fn try_from(other: String) -> Result<Self> {
        Rc::new(other).try_into()
    }
}

impl TryFrom<&str> for LabelName {
    type Error = Error;
    fn try_from(other: &str) -> Result<Self> {
        other.to_string().try_into()
    }
}

impl TryFrom<Rc<String>> for LabelName {
    type Error = Error;
    fn try_from(other: Rc<String>) -> Result<Self> {
        // FIXME: Should probably limit the characters that may be used.
        Ok(LabelName(other))
    }
}

impl TryFrom<&Rc<String>> for LabelName {
    type Error = Error;
    fn try_from(other: &Rc<String>) -> Result<Self> {
        other.clone().try_into()
    }
}

impl Into<Rc<String>> for LabelName {
    fn into(self) -> Rc<String> {
        self.0.clone()
    }
}

impl AsRef<str> for LabelName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
