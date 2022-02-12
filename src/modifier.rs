use serde::{Deserialize, Serialize};

#[derive(
    Copy,
    Clone,
    Debug,
    Deserialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
pub enum Modifier {
    Honey,
}

impl Modifier {
    pub fn emoji(&self) -> char {
        match self {
            Self::Honey => '🍯',
        }
    }
}

impl std::fmt::Display for Modifier {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.emoji())
    }
}
