struct Reachable {
    current_position: (u32, u32),
    force: f32,
    left: Move,
    right: Move,
    up: Move,
    down: Move,
}

struct Move {
    reachable: bool,
    cost: f32,
}
struct Finger {
    // thumb: Reachable,
    index: Reachable,
    middle: Reachable,
    ring: Reachable,
    little: Reachable,
}

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

pub const HAND_MODEL: Finger = Finger {
    index: Reachable {
        force: 1.0,
        left: Move {
            reachable: true,
            cost: 1.0,
        },
        right: Move {
            reachable: false,
            cost: 10e10,
        },
        up: Move {
            reachable: true,
            cost: 1.5,
        },
        down: Move {
            reachable: true,
            cost: 1.25,
        },
    },
    middle: Reachable {
        force: 1.0,
        left: Move {
            reachable: true,
            cost: 1.0,
        },
        right: Move {
            reachable: false,
            cost: 10e10,
        },
        up: Move {
            reachable: true,
            cost: 1.05,
        },
        down: Move {
            reachable: true,
            cost: 1.5,
        },
    },
    ring: Reachable {
        force: 1.1,
        left: Move {
            reachable: false,
            cost: 10e10,
        },
        right: Move {
            reachable: true,
            cost: 1.5,
        },
        up: Move {
            reachable: true,
            cost: 1.05,
        },
        down: Move {
            reachable: true,
            cost: 1.7,
        },
    },
    little: Reachable {
        force: 1.5,
        left: Move {
            reachable: false,
            cost: 10e10,
        },
        right: Move {
            reachable: true,
            cost: 1.3,
        },
        up: Move {
            reachable: true,
            cost: 1.6,
        },
        down: Move {
            reachable: true,
            cost: 1.7,
        },
    },
};
