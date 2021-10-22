use std::convert::{TryFrom, TryInto};
use std::rc::Rc;

use crate::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Condition {
    cond: Rc<String>,
    arg1: MindustryTerm,
    arg2: MindustryTerm,
}

impl Condition {
    pub fn always() -> Condition {
        Condition {
            cond: Rc::new(String::from("always")),

            // By convention. These are the defaults in Mindustry.
            arg1: "x".try_into().unwrap(),
            arg2: "false".try_into().unwrap(),
        }
    }

    pub fn never() -> Condition {
        Condition {
            cond: Rc::new(String::from("equal")),
            arg1: "0".try_into().unwrap(),
            arg2: "1".try_into().unwrap(),
        }
    }
}

impl std::fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} {} {}", &self.cond, self.arg1, self.arg2)
    }
}

impl TryFrom<(Rc<String>, MindustryTerm, MindustryTerm)> for Condition {
    type Error = Error;
    fn try_from(other: (Rc<String>, MindustryTerm, MindustryTerm)) -> Result<Self> {
        let (cond, arg1, arg2) = other;

        // FIXME: validate the condition
        if cond.is_empty() {
            bail!("Invalid condition: <empty>");
        }

        Ok(Condition { cond, arg1, arg2 })
    }
}
