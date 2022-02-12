use itertools::Itertools;
use log::trace;
use serde::{Deserialize, Serialize};

use crate::{
    dice::Dice, friend::Friend, modifier::Modifier, params::TEAM_SIZE,
    species::Species,
};

/// Up to five species friends.  The front of the team is at index 0, i.e.
/// attacking and defending first.
#[derive(
    Copy,
    Clone,
    Hash,
    Debug,
    Deserialize,
    Eq,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
)]
pub struct Team([Option<Friend>; TEAM_SIZE]);

impl Team {
    pub fn new() -> Self {
        Team([None; TEAM_SIZE])
    }

    pub fn compact_permutations(&self) -> impl Iterator<Item = Self> + '_ {
        let count = self.count();
        self.0
            .iter()
            .cloned()
            .flatten()
            .permutations(count)
            .map(|t| {
                let mut fs = [None; 5];
                for (i, f) in t.into_iter().enumerate() {
                    fs[i] = Some(f);
                }
                Team(fs)
            })
    }

    /// Checks to see whether the given team is dumb.  A dumb team has fewer
    /// than three members and all of its members have their stock HP, i.e.
    /// there's no reason not to have three members
    pub fn is_dumb(&self) -> bool {
        self.count() < 3
            && self
                .0
                .iter()
                .flatten()
                .all(|f| f.modifier.is_none() && f.has_default_power())
    }

    /// Asks the species at position `i` to perform on-summon actions, with
    /// a new species summoned at position `pos`
    pub fn on_summon(&mut self, i: usize, pos: usize) {
        assert!(i != pos);
        assert!(self[i].is_some());
        assert!(self[pos].is_some());

        match self[i].unwrap().species {
            Species::Horse => {
                // This is technically a temporary buf, but we're only
                // simulating a single turn here, so it doesn't matter.
                let f = self[pos].as_mut().unwrap();
                trace!(
                    "    🐴 at {} bufs {}  at {} by ❤️  +1, ⚔️  +1",
                    i,
                    f.species,
                    pos
                );
                f.attack += 1;
                f.health += 1;
            }
            _ => (),
        }
    }

    pub fn count(&self) -> usize {
        self.0.iter().filter(|i| i.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Picks some number of random friends from the team, returning an iterator
    pub fn random_friends<'a, 'b, R: Dice>(
        &'a self,
        n: usize,
        rng: &'b mut R,
    ) -> impl Iterator<Item = usize> + 'b {
        crate::dice::pick_some(rng, n, &self.0)
    }

    /// Returns a random friend's index, or `None` if the team is empty
    pub fn random_friend<R: Dice>(&self, rng: &mut R) -> Option<usize> {
        crate::dice::pick_one(rng, &self.0)
    }

    /// Sets experience to 0 for all team members, which is useful when
    /// deduplicating teams (because exp doesn't matter in battle)
    pub fn without_exp(&self) -> Self {
        let mut out = *self;
        for i in out.0.iter_mut().flatten() {
            i.exp = 0;
        }
        out
    }

    /// Shuffles team members so they're tightly packed against 0
    pub fn compact(&mut self) {
        let mut i = 0;
        loop {
            while i < TEAM_SIZE && self[i].is_some() {
                i += 1;
            }
            let mut j = i;
            while j < TEAM_SIZE && self[j].is_none() {
                j += 1;
            }
            if i >= TEAM_SIZE || j >= TEAM_SIZE {
                break;
            }
            self[i] = self[j].take();
        }
    }

    fn fmt_with_range<I: Iterator<Item = usize> + Clone>(
        &self,
        f: &mut std::fmt::Formatter,
        range: I,
    ) -> std::fmt::Result {
        for i in range.clone() {
            write!(f, "{} ───┐ ", i)?;
        }
        writeln!(f)?;
        for i in range.clone() {
            if let Some(m) = self[i].and_then(|a| a.modifier) {
                write!(f, "│ {} │ ", m)?;
            } else {
                write!(f, "│    │ ")?;
            }
        }
        writeln!(f)?;
        for i in range.clone() {
            if let Some(a) = self[i] {
                write!(f, "│ {} │ ", a.species)?;
            } else {
                write!(f, "│    │ ")?;
            }
        }
        writeln!(f)?;
        for i in range.clone() {
            if let Some(a) = self[i] {
                write!(f, "│❤️  {}│ ", a.health)?;
            } else {
                write!(f, "│    │ ")?;
            }
        }
        writeln!(f)?;
        for i in range.clone() {
            if let Some(a) = self[i] {
                write!(f, "│⚔️  {}│ ", a.attack)?;
            } else {
                write!(f, "│    │ ")?;
            }
        }
        writeln!(f)?;
        for _ in range.clone() {
            write!(f, "└────┘ ")?;
        }
        Ok(())
    }
    /// Removes dead speciess from the team, performing their on-death actions
    /// then compacting the team afterwards.
    pub fn remove_dead<R: Dice>(&mut self, rng: &mut R) {
        let mut changed = false;
        for i in 0..TEAM_SIZE {
            if self[i].is_some() && self[i].unwrap().health == 0 {
                let f = self[i].take().unwrap();
                trace!("{} at {} is dead, removing", f.species, i);
                self.on_death(f, i, rng);
                changed = true;
            }
        }
        if changed {
            trace!("Compacting team");
            self.compact();
        }
    }

    pub fn summon(&mut self, friend: Friend, team_pos: usize) {
        self[team_pos] = Some(friend);

        for i in 0..TEAM_SIZE {
            if i != team_pos && self[i].is_some() {
                self.on_summon(i, team_pos);
            }
        }
    }

    pub fn on_death<R: Dice>(&mut self, f: Friend, i: usize, rng: &mut R) {
        assert!(self[i].is_none());
        match f.species {
            Species::Cricket => {
                trace!("Summoning ghost cricket at {}", i);
                self[i] = Some(Friend {
                    species: Species::GhostCricket,
                    attack: f.level(),
                    health: f.level(),
                    modifier: None,
                    exp: 0,
                });
                for j in 0..TEAM_SIZE {
                    if i != j && self[j].is_some() {
                        self.on_summon(j, i);
                    }
                }
            }
            Species::Ant => {
                if let Some(j) = self.random_friend(rng) {
                    let g = self[j].as_mut().unwrap();
                    let attack = f.level() * 2;
                    let health = f.level();
                    trace!(
                        "{} on death is buffing {} at {} by ❤️  +{}, ⚔️  + {}",
                        f.species,
                        g.species,
                        j,
                        health,
                        attack
                    );
                    g.attack += attack;
                    g.health += health;
                }
            }
            _ => (),
        }
        match f.modifier {
            Some(Modifier::Honey) => {
                let bee = Friend {
                    species: Species::Bee,
                    attack: 1,
                    health: 1,
                    modifier: None,
                    exp: 0,
                };
                if self.make_space_at(i) {
                    trace!("Summoning {} at {}", bee.species, i);
                    self[i] = Some(bee);
                    for j in 0..TEAM_SIZE {
                        if i != j && self[j].is_some() {
                            self.on_summon(j, i);
                        }
                    }
                } else {
                    trace!("No room to summon {}", bee.species);
                }
            }
            None => (),
        }
    }
    /// Attempts to make space at the given position.  Returns the empty
    /// position, after shoving speciess around, or None if the team is full.
    pub fn make_space_at(&mut self, i: usize) -> bool {
        if self[i].is_none() {
            return true;
        }
        // Look for an empty slot behind the target slot, and shift
        // friends backwards (away from 0) to free up slot i
        for j in (i + 1)..TEAM_SIZE {
            if self[j].is_none() {
                for k in (i..j).rev() {
                    assert!(self[k + 1].is_none());
                    self[k + 1] = self[k].take();
                }
                assert!(self[i].is_none());
                return true;
            }
        }
        // Otherwise, look for an empty slot in front of the target
        // slot, and shift friends forwards (towards 0)
        for j in 0..i {
            if self[j].is_none() {
                for k in j..i {
                    assert!(self[k].is_none());
                    self[k] = self[k + 1].take();
                }
                assert!(self[i].is_none());
                return true;
            }
        }

        assert!(self[i].is_some());
        false
    }
}

impl std::ops::Index<usize> for Team {
    type Output = Option<Friend>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Team {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

pub struct TeamPrinter<'a>(pub &'a Team, pub bool);

impl std::fmt::Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", TeamPrinter(self, true))
    }
}

impl std::fmt::Display for TeamPrinter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.1 {
            self.0.fmt_with_range(f, (0..TEAM_SIZE).rev())
        } else {
            self.0.fmt_with_range(f, 0..TEAM_SIZE)
        }
    }
}
