#[derive(Debug, PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
}

impl Hand {
    pub fn same(&self, other: &Hand) -> bool {
        self == other
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Finger {
    I,
    M,
    R,
    P,
}

impl Finger {
    pub fn same(&self, other: &Finger) -> bool {
        self == other
    }
}
