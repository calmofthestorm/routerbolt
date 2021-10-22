/// An index into `ops`, the list of IR instructions. This is used when one
/// instruction needs to refer to another. I guess we could do this with Rc
/// instead, but this is fine too.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct IrIndex(usize);

impl std::fmt::Display for IrIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<usize> for IrIndex {
    fn from(other: usize) -> Self {
        IrIndex(other)
    }
}

impl Into<usize> for IrIndex {
    fn into(self) -> usize {
        self.0
    }
}

impl From<&usize> for IrIndex {
    fn from(other: &usize) -> Self {
        IrIndex(*other)
    }
}

impl Into<usize> for &IrIndex {
    fn into(self) -> usize {
        self.0
    }
}

impl std::ops::Deref for IrIndex {
    type Target = usize;
    fn deref(&self) -> &usize {
        &self.0
    }
}
