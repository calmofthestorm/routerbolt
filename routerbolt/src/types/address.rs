/// Address in the generated program. This is the same as the number used in
/// "jump", and is just the line number in the program.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Address(usize);

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<usize> for Address {
    fn from(other: usize) -> Self {
        Address(other)
    }
}

impl Into<usize> for Address {
    fn into(self) -> usize {
        self.0
    }
}

impl From<&usize> for Address {
    fn from(other: &usize) -> Self {
        Address(*other)
    }
}

impl Into<usize> for &Address {
    fn into(self) -> usize {
        self.0
    }
}

impl AsRef<usize> for Address {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

/// The difference between two addresses.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AddressDelta(usize);

impl AddressDelta {
    pub const fn new(n: usize) -> AddressDelta {
        AddressDelta(n)
    }
}

impl std::fmt::Display for AddressDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<usize> for AddressDelta {
    fn from(other: usize) -> Self {
        AddressDelta(other)
    }
}

impl Into<usize> for AddressDelta {
    fn into(self) -> usize {
        self.0
    }
}

impl From<&usize> for AddressDelta {
    fn from(other: &usize) -> Self {
        AddressDelta(*other)
    }
}

impl Into<usize> for &AddressDelta {
    fn into(self) -> usize {
        self.0
    }
}

impl std::ops::AddAssign<AddressDelta> for Address {
    fn add_assign(&mut self, other: AddressDelta) {
        *self = *self + other
    }
}

impl std::ops::Add<AddressDelta> for Address {
    type Output = Self;
    fn add(self, other: AddressDelta) -> Self {
        Self(self.0.checked_add(other.0).unwrap())
    }
}

impl std::ops::Sub<AddressDelta> for Address {
    type Output = Self;
    fn sub(self, other: AddressDelta) -> Self {
        Self(self.0.checked_sub(other.0).unwrap())
    }
}

impl std::ops::Sub<Address> for Address {
    type Output = AddressDelta;
    fn sub(self, other: Address) -> AddressDelta {
        AddressDelta(self.0.checked_sub(other.0).unwrap())
    }
}

impl std::ops::Sub for AddressDelta {
    type Output = Self;
    fn sub(self, other: AddressDelta) -> Self {
        Self(self.0.checked_sub(other.0).unwrap())
    }
}

impl std::ops::AddAssign for AddressDelta {
    fn add_assign(&mut self, other: AddressDelta) {
        *self = *self + other;
    }
}

impl std::ops::Add for AddressDelta {
    type Output = Self;
    fn add(self, other: AddressDelta) -> Self {
        Self(self.0.checked_add(other.0).unwrap())
    }
}

impl std::ops::MulAssign<usize> for AddressDelta {
    fn mul_assign(&mut self, other: usize) {
        *self = *self * other;
    }
}

impl std::ops::Mul<usize> for AddressDelta {
    type Output = Self;
    fn mul(self, other: usize) -> Self {
        Self(self.0.checked_mul(other).unwrap())
    }
}

impl std::iter::Sum for AddressDelta {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.map(|d| d.0).sum::<usize>().into()
    }
}

impl AsRef<usize> for AddressDelta {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}
