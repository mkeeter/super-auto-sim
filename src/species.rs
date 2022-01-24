use crate::{modifier::Modifier, rng::RangeRng};
use serde::{Deserialize, Serialize};

/// Tier 1 speciess in the free-to-play pack
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Species {
    Ant,
    Beaver,
    Cricket,
    Duck,
    Fish,
    Horse,
    Mosquito,
    Otter,
    Pig,

    GhostCricket,
    Bee,
}

impl Species {
    pub fn emoji(&self) -> char {
        match self {
            Self::Ant => '🐜',
            Self::Beaver => '🦫',
            Self::Cricket => '🦗',
            Self::Duck => '🦆',
            Self::Fish => '🐟',
            Self::Horse => '🐴',
            Self::Mosquito => '🦟',
            Self::Otter => '🦦',
            Self::Pig => '🐷',
            Self::GhostCricket => '🦗',
            Self::Bee => '🐝',
        }
    }

    /// Returns the default health and attack for an species; panics if the
    /// selected species can't be purchased from the shop.
    pub fn default_power(&self) -> (usize, usize) {
        match self {
            Self::Ant => (2, 1),
            Self::Beaver => (2, 2),
            Self::Cricket => (1, 2),
            Self::Duck => (1, 2),
            Self::Fish => (2, 3),
            Self::Horse => (2, 1),
            Self::Mosquito => (2, 2),
            Self::Otter => (1, 2),
            Self::Pig => (3, 1),

            Self::GhostCricket | Self::Bee => {
                panic!("Cannot purchase {:?}", self)
            }
        }
    }

    pub fn can_purchase(&self) -> bool {
        match self {
            Self::Ant
            | Self::Beaver
            | Self::Cricket
            | Self::Duck
            | Self::Fish
            | Self::Horse
            | Self::Mosquito
            | Self::Otter
            | Self::Pig => true,

            Self::GhostCricket | Self::Bee => false,
        }
    }

    /// Returns the default modifier for the species, which is `None` for all
    /// Tier 1 units.
    pub fn default_modifier(&self) -> Option<Modifier> {
        None
    }

    pub fn sample<R: RangeRng>(rng: &mut R) -> Self {
        match rng.gen_range(0..9) {
            0 => Species::Ant,
            1 => Species::Beaver,
            2 => Species::Cricket,
            3 => Species::Duck,
            4 => Species::Fish,
            5 => Species::Horse,
            6 => Species::Mosquito,
            7 => Species::Otter,
            8 => Species::Pig,
            _ => panic!("Invalid random number"),
        }
    }
}

impl std::fmt::Display for Species {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.emoji())?;
        if *self == Self::Beaver {
            write!(f, " ")?; // Work around an iTerm2 bug (#10186)
        }
        Ok(())
    }
}
