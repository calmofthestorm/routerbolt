pub mod address;
pub mod condition;
pub mod frame_index;
pub mod function_name;
pub mod ir_index;
pub mod label_name;
pub mod mindustry_command;
pub mod stack_depth;

pub use address::*;
pub use condition::*;
pub use frame_index::*;
pub use function_name::*;
pub use ir_index::*;
pub use label_name::*;
pub use mindustry_command::*;
pub use stack_depth::*;

use std::convert::{AsRef, TryFrom};
use std::rc::Rc;

use crate::*;

/// Mindustry, as a rule, does not (statically) distinguish between Lhs and Rhs.
/// You can write "op add 1 1 1" and it just won't do anything.
///
/// As such, this type can hold anything Mindustry will accept there.
/// Essentially, where we can, we pass this ambiguity along unchanged rather
/// than attempting to impose structure that it lacks.
///
/// That's not to say that introducing a type system would be a bad thing, it's
/// just out of scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    StackVar(StackVar),

    // Anything Mindustry will accept as the LHS of an assignment/math. In other
    // words, literal or symbol. We just pass that along as is for now.
    Mindustry(MindustryTerm),
}

impl Term {
    pub fn accumulator() -> Term {
        MindustryTerm::accumulator().into()
    }
}

impl MindustryTerm {
    // FIXME: It would be nice to use this more, and have others for constants
    // like the stack.
    pub fn accumulator() -> MindustryTerm {
        Self::try_from("MF_acc").unwrap()
    }

    pub fn stack_sz() -> MindustryTerm {
        Self::try_from("MF_stack_sz").unwrap()
    }

    pub fn stack_tmp() -> MindustryTerm {
        Self::try_from("MF_stack_tmp").unwrap()
    }

    pub fn zero() -> MindustryTerm {
        Self::try_from("0").unwrap()
    }
}

/// A Mindustry term.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MindustryTerm(Rc<String>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StackVar(Rc<String>);

impl From<MindustryTerm> for Term {
    fn from(other: MindustryTerm) -> Self {
        Term::Mindustry(other)
    }
}

impl From<StackVar> for Term {
    fn from(other: StackVar) -> Self {
        Term::StackVar(other)
    }
}

impl TryFrom<&str> for StackVar {
    type Error = Error;
    fn try_from(other: &str) -> Result<Self> {
        match Term::try_from(other)? {
            Term::StackVar(v) => Ok(v),
            Term::Mindustry(..) => bail!("Stack var required here (starts with *)"),
        }
    }
}

impl TryFrom<&str> for MindustryTerm {
    type Error = Error;
    fn try_from(other: &str) -> Result<Self> {
        match Term::try_from(other)? {
            Term::Mindustry(v) => Ok(v),
            Term::StackVar(..) => bail!("Stack var forbidden here (starts with *)"),
        }
    }
}

impl TryFrom<&str> for Term {
    type Error = Error;
    fn try_from(other: &str) -> Result<Self> {
        if other.is_empty() {
            bail!("Symbol may not be empty");
        }

        let value = Rc::new(other.to_string());

        if other.starts_with("*") {
            // Technically I think Mindustry will permit this, but I have to
            // draw the line somewhere.
            Ok(Term::StackVar(StackVar(value)))
        } else {
            Ok(Term::Mindustry(MindustryTerm(value)))
        }
    }
}

impl std::fmt::Display for MindustryTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl std::fmt::Display for StackVar {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl AsRef<str> for Term {
    fn as_ref(&self) -> &str {
        match self {
            Term::Mindustry(t) => t.as_ref(),
            Term::StackVar(t) => t.as_ref(),
        }
    }
}

impl AsRef<str> for MindustryTerm {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for StackVar {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
