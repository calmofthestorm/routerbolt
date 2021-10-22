/// Index relative to the start of a stack frame. Subtract from the frame size
/// to get the stack depth.
#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameIndex(usize);

impl std::fmt::Display for FrameIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<usize> for FrameIndex {
    fn from(other: usize) -> Self {
        FrameIndex(other)
    }
}

impl Into<usize> for FrameIndex {
    fn into(self) -> usize {
        self.0
    }
}

impl From<&usize> for FrameIndex {
    fn from(other: &usize) -> Self {
        FrameIndex(*other)
    }
}

impl Into<usize> for &FrameIndex {
    fn into(self) -> usize {
        self.0
    }
}
