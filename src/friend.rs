use crate::{modifier::Modifier, species::Species};
use serde::{Deserialize, Serialize};

/// A [Friend] is an species embodied onto a team (or in the shop)
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Friend {
    pub species: Species,
    pub attack: usize,
    pub health: usize,
    pub modifier: Option<Modifier>,
    pub exp: usize,
}

impl Friend {
    pub fn new(species: Species) -> Self {
        let (health, attack) = species.default_power();
        let modifier = species.default_modifier();
        Self {
            species,
            health,
            attack,
            modifier,
            exp: 0,
        }
    }
    pub fn level(&self) -> usize {
        match self.exp {
            0..=2 => 1,
            3..=5 => 1,
            6 => 3,
            exp => panic!("Invalid exp: {}", exp),
        }
    }
}
