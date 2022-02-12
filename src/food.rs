use crate::dice::Dice;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Food {
    Apple,
    Honey,
}

impl Food {
    pub fn emoji(&self) -> char {
        match self {
            Self::Apple => '🍎',
            Self::Honey => '🍯',
        }
    }
    pub fn sample<R: Dice>(rng: &mut R) -> Self {
        match rng.roll(0..2) {
            0 => Food::Apple,
            1 => Food::Honey,
            _ => panic!("Invalid random number"),
        }
    }
}

impl std::fmt::Display for Food {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.emoji())
    }
}
