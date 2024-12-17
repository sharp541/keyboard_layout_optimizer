#[derive(Debug, PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
    Other,
}

impl Hand {
    pub fn same(&self, other: Hand) -> bool {
        matches!(self, Hand::Left) && matches!(other, Hand::Left)
            || matches!(self, Hand::Right) && matches!(other, Hand::Right)
    }
}
