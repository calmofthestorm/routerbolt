use std::convert::TryFrom;
use std::fmt::Write;
use std::rc::Rc;

use crate::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MindustryCommand(Vec<Rc<String>>);

impl std::fmt::Display for MindustryCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if !self.0.is_empty() {
            self.0[0].fmt(f)?;
            for token in self.0[1..].iter() {
                f.write_char(' ')?;
                token.fmt(f)?;
            }
        }

        Ok(())
    }
}

impl TryFrom<Vec<Rc<String>>> for MindustryCommand {
    type Error = Error;
    fn try_from(other: Vec<Rc<String>>) -> Result<Self> {
        for token in other.iter() {
            if token.starts_with("*") {
                bail!("Mindustry commands and their args may not start with * since we don't currently support stack vars there so it would be confusing");
            }
        }
        // FIXME: Should probably validate further.
        Ok(MindustryCommand(other))
    }
}

impl Into<Vec<Rc<String>>> for MindustryCommand {
    fn into(self) -> Vec<Rc<String>> {
        self.0
    }
}
