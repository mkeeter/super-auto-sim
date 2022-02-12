use log::trace;
use serde::{Deserialize, Serialize};

use crate::{
    dice::Dice,
    params::TEAM_SIZE,
    species::Species,
    team::{Team, TeamPrinter},
};

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Winner {
    TeamA,
    TeamB,
    Tied,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Battle(pub Team, pub Team);

impl Battle {
    /// Simulates a battle, returning the winner
    pub fn run<R: Dice>(mut self, rng: &mut R) -> Winner {
        trace!("Initial state:\n{}", self);
        self.before_battle(rng);
        for i in 0.. {
            trace!("Round {}:\n{}", i, self);
            match (self.0.is_empty(), self.1.is_empty()) {
                (true, true) => {
                    trace!("Battle ended with a tie");
                    return Winner::Tied;
                }
                (false, true) => {
                    trace!("Battle ended with a win for Team A");
                    return Winner::TeamA;
                }
                (true, false) => {
                    trace!("Battle ended with a win for Team B");
                    return Winner::TeamB;
                }
                (false, false) => self.step(rng),
            }
        }
        unreachable!();
    }

    /// Performs pre-battle actions, returning all possible states
    fn before_battle<R: Dice>(&mut self, rng: &mut R) {
        for t in [true, false] {
            for i in 0..TEAM_SIZE {
                self.on_battle_start(i, t, rng);
            }
        }
        // XXX This architecture wouldn't work for more complex situations,
        // e.g. a mosquito sniping a hedgehog which then kills other stuff
        self.0.remove_dead(rng);
        self.1.remove_dead(rng);
    }

    fn on_battle_start<R: Dice>(&mut self, i: usize, team: bool, rng: &mut R) {
        let f = match self[team][i] {
            Some(f) => f,
            None => return,
        };
        match f.species {
            Species::Mosquito => {
                for j in self[!team].random_friends(f.level(), rng) {
                    let g = self[!team][j].as_mut().unwrap();
                    trace!(
                        "{} at {} shot {} at {} for 1",
                        f.species,
                        i,
                        g.species,
                        j
                    );
                    g.health = g.health.saturating_sub(1);
                }
            }
            _ => (),
        }
    }

    /// Executes a single step of the battle, returning true if the battle ended
    fn step<R: Dice>(&mut self, rng: &mut R) {
        let f = self.0[0].as_mut().unwrap();
        let g = self.1[0].as_mut().unwrap();
        trace!("{} clashes with {}!", f.species, g.species);
        f.health = f.health.saturating_sub(g.attack);
        g.health = g.health.saturating_sub(f.attack);

        // TODO
        self.0.remove_dead(rng);
        self.1.remove_dead(rng);
    }
}

impl std::ops::Index<bool> for Battle {
    type Output = Team;
    fn index(&self, index: bool) -> &Self::Output {
        if index {
            &self.0
        } else {
            &self.1
        }
    }
}

impl std::ops::IndexMut<bool> for Battle {
    fn index_mut(&mut self, index: bool) -> &mut Self::Output {
        if index {
            &mut self.0
        } else {
            &mut self.1
        }
    }
}

impl std::fmt::Display for Battle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let team = format!("{}", self.0);
        let enemy = format!("{}", TeamPrinter(&self.1, false));

        for (i, (a, b)) in team.split('\n').zip(enemy.split('\n')).enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}   {}", a, b)?;
        }
        Ok(())
    }
}
