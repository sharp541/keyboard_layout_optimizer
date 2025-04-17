use bitflags::bitflags;

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

bitflags! {
    #[derive(Default, Debug, Clone, Copy)]
    pub struct Finger: u8 {
        const I = 0b0001;
        const M = 0b0010;
        const R = 0b0100;
        const P = 0b1000;
    }
}
