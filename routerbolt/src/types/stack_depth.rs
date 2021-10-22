/// Stack depth relative to the top.
#[derive(Copy, Clone, PartialEq, Debug, Eq, Hash)]
pub struct StackDepth(usize);

impl std::fmt::Display for StackDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<usize> for StackDepth {
    fn from(other: usize) -> Self {
        StackDepth(other)
    }
}

impl Into<usize> for StackDepth {
    fn into(self) -> usize {
        self.0
    }
}

impl From<&usize> for StackDepth {
    fn from(other: &usize) -> Self {
        StackDepth(*other)
    }
}

impl Into<usize> for &StackDepth {
    fn into(self) -> usize {
        self.0
    }
}
