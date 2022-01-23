use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use enum_iterator::IntoEnumIterator;
use log::{debug, info, trace, LevelFilter};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};

const TEAM_SIZE: usize = 5;
const STORE_ANIMAL_COUNT: usize = 3;
const STORE_FOOD_COUNT: usize = 1;

////////////////////////////////////////////////////////////////////////////////

/// Tier 1 animals in the free-to-play pack
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
enum Animal {
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

impl Animal {
    fn emoji(&self) -> char {
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

    /// Checks whether the given animal type is deterministic in battle
    fn deterministic_in_battle(&self) -> bool {
        match self {
            Self::Ant => false,
            Self::Beaver => true,
            Self::Cricket => true,
            Self::Duck => true,
            Self::Fish => true,
            Self::Horse => true,
            Self::Mosquito => false,
            Self::Otter => true,
            Self::Pig => true,

            Self::GhostCricket => true,
            Self::Bee => true,
        }
    }

    /// Returns the default health and attack for an animal; panics if the
    /// selected animal can't be purchased from the shop.
    fn default_power(&self) -> (usize, usize) {
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

            Self::GhostCricket | Self::Bee => panic!("Cannot purchase {:?}", self),
        }
    }

    fn can_purchase(&self) -> bool {
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

    /// Returns the default modifier for the animal, which is `None` for all
    /// Tier 1 units.
    fn default_modifier(&self) -> Option<Modifier> {
        None
    }
}

impl std::fmt::Display for Animal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.emoji())?;
        if *self == Self::Beaver {
            write!(f, " ")?; // Work around an iTerm2 bug (#10186)
        }
        Ok(())
    }
}

/// We can randomly sample the [Animal] type to get a random animal that's
/// valid for the shop (i.e. skipping animals which can only be summoned
/// through special means).
impl rand::distributions::Distribution<Animal> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Animal {
        match rng.gen_range(0..=8) {
            0 => Animal::Ant,
            1 => Animal::Beaver,
            2 => Animal::Cricket,
            3 => Animal::Duck,
            4 => Animal::Fish,
            5 => Animal::Horse,
            6 => Animal::Mosquito,
            7 => Animal::Otter,
            8 => Animal::Pig,
            _ => panic!("Invalid random number"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

enum Food {
    Apple,
    Honey,
}

impl Food {
    fn emoji(&self) -> char {
        match self {
            Self::Apple => '🍎',
            Self::Honey => '🍯',
        }
    }
}

impl std::fmt::Display for Food {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.emoji())
    }
}

impl rand::distributions::Distribution<Food> for rand::distributions::Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Food {
        match rng.gen_range(0..=1) {
            0 => Food::Apple,
            1 => Food::Honey,
            _ => panic!("Invalid random number"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
enum Modifier {
    Honey,
}

impl Modifier {
    fn emoji(&self) -> char {
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

////////////////////////////////////////////////////////////////////////////////

/// A [Friend] is an animal embodied onto a team (or in the shop)
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
struct Friend {
    animal: Animal,
    attack: usize,
    health: usize,
    modifier: Option<Modifier>,
    exp: usize,
}

impl Friend {
    fn new(animal: Animal) -> Self {
        let (health, attack) = animal.default_power();
        let modifier = animal.default_modifier();
        Self {
            animal,
            health,
            attack,
            modifier,
            exp: 0,
        }
    }
    fn level(&self) -> usize {
        match self.exp {
            0..=2 => 1,
            3..=5 => 1,
            6 => 3,
            exp => panic!("Invalid exp: {}", exp),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

struct Shop {
    // XXX More slots get added to the shop over time
    animals: [Option<Friend>; STORE_ANIMAL_COUNT],
    foods: [Option<Food>; STORE_FOOD_COUNT],
}

impl Shop {
    fn new<R: rand::Rng>(rng: &mut R) -> Self {
        let mut animals = [None; STORE_ANIMAL_COUNT];
        for a in animals.iter_mut() {
            *a = Some(Friend::new(rng.gen()));
        }
        let mut foods = [None; STORE_FOOD_COUNT];
        for f in foods.iter_mut() {
            *f = Some(rng.gen());
        }
        Shop { animals, foods }
    }

    /// Picks a random friend from the shop, returning its index
    fn random_friend<R: rand::Rng>(&self, rng: &mut R) -> Option<usize> {
        if self.animals.iter().all(|i| i.is_none()) {
            return None;
        }
        loop {
            let i = rng.gen_range(0..STORE_ANIMAL_COUNT);
            if self.animals[i].is_some() {
                return Some(i);
            }
        }
    }

    fn random_food<R: rand::Rng>(&self, rng: &mut R) -> Option<usize> {
        if self.foods.iter().all(|i| i.is_none()) {
            return None;
        }
        loop {
            let i = rng.gen_range(0..STORE_FOOD_COUNT);
            if self.foods[i].is_some() {
                return Some(i);
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

struct BuyPhase<R: rand::Rng> {
    team: Team,
    gold: usize,
    shop: Shop,
    rng: R,
}

impl<R: rand::Rng> BuyPhase<R> {
    /// Buys the animal at `shop_pos` and adds it to `team_pos`
    fn buy_animal(&mut self, shop_pos: usize, team_pos: usize) {
        assert!(self.gold >= 3);

        self.gold -= 3;
        let friend = self.shop.animals[shop_pos].take().unwrap();

        match &mut self.team[team_pos] {
            None => {
                trace!("Buying {} at position {}", friend.animal, team_pos);
                self.on_buy(friend);
                self.team[team_pos] = Some(friend);

                for i in 0..TEAM_SIZE {
                    if i != team_pos && self.team[i].is_some() {
                        self.team.on_summon(i, team_pos);
                    }
                }
            }
            Some(target) => {
                assert!(target.animal == friend.animal);
                self.combine_animal(team_pos, friend);

                // The on-buy trigger happens after the animals are combined,
                // which matters in cases where the animal levels up.  For
                // convenience, we remove the animal from the team briefly,
                // then reinstall it.
                let friend = self.team[team_pos].take().unwrap();
                self.on_buy(friend);
                self.team[team_pos] = Some(friend);
            }
        }
        // XXX: There are also "friend is bought" triggers, but nothing in Tier
        // 1 uses them
    }

    fn combine_animal(&mut self, team_pos: usize, g: Friend) {
        let f = self.team[team_pos].as_mut().unwrap();
        assert!(f.animal == g.animal);
        trace!("Combining {} at position {}", f.animal, team_pos);
        f.health = std::cmp::max(f.health, g.health) + 1;
        f.attack = std::cmp::max(f.attack, g.attack) + 1;
        f.exp += 1;
        // TODO: handle level-up here
    }

    fn sell_animal(&mut self, team_pos: usize) {
        assert!(self.team[team_pos].is_some());

        let a = self.team[team_pos].take().unwrap();
        trace!("Selling {} at position {}", a.animal, team_pos);

        self.gold += a.level();
        self.on_sell(a);
        for i in 0..TEAM_SIZE {
            if i != team_pos && self.team[i].is_some() {
                self.on_sold(i);
            }
        }
    }

    /// Purchases food at the given shop position, applying it to the chosen
    /// member of the team.
    fn buy_food(&mut self, shop_pos: usize, team_pos: usize) {
        assert!(self.shop.foods[shop_pos].is_some());
        assert!(self.team[team_pos].is_some());

        let friend = self.team[team_pos].as_mut().unwrap();
        let food = self.shop.foods[shop_pos].take().unwrap();
        self.gold -= 3;
        trace!(
            "Buying {} for {} at position {}",
            food,
            friend.animal,
            team_pos
        );
        match food {
            Food::Apple => {
                trace!("    Buffing by ❤️  +1, ⚔️  +1");
                friend.attack += 1;
                friend.health += 1;
            }
            Food::Honey => {
                trace!("    Applying honey modifier");
                friend.modifier = Some(Modifier::Honey);
            }
        }
    }

    /// Asks an animal to perform its on-buy action.  It has not yet been added
    /// to the team at this point.
    fn on_buy(&mut self, f: Friend) {
        match f.animal {
            Animal::Otter => {
                // Give a random friend (+1, +1)
                for i in self.team.random_friends(1, &mut self.rng) {
                    let g = self.team[i].as_mut().unwrap();
                    trace!(
                        "    {} on buy bufs {} at {} by ❤️  +1, ⚔️  +1",
                        f.animal,
                        g.animal,
                        i
                    );
                    g.health += 1;
                    g.attack += 1;
                }
            }
            _ => (),
        }
    }

    /// Asks an animal to perform its on-sell action.  It has been removed
    /// from the team at this point.
    fn on_sell(&mut self, a: Friend) {
        match a.animal {
            Animal::Beaver => {
                // Give two random friends +1 Health
                let delta = a.level();
                for i in self.team.random_friends(2, &mut self.rng) {
                    let f = self.team[i].as_mut().unwrap();
                    trace!(
                        "    {} on sell bufs {} at {} b❤️  +{} ",
                        a.animal,
                        f.animal,
                        i,
                        delta
                    );
                    f.health += delta;
                }
            }
            Animal::Duck => {
                // Give shop pets +1 Health
                // XXX: this changes with level
                let delta = a.level();
                for f in self.shop.animals.iter_mut().flatten() {
                    trace!(
                        "    {} on sell bufs {} in shop by ❤️  +{}",
                        a.animal,
                        f.animal,
                        delta
                    );
                    f.health += delta;
                }
            }
            Animal::Pig => {
                let delta = a.level();
                trace!("    {} on sell gives 🪙 +{}", a.animal, delta);
                self.gold += delta;
            }
            _ => (),
        }
    }

    fn on_sold(&mut self, _i: usize) {
        // No Tier 1 animals have an on-sold trigger
    }

    fn step_random(&mut self) -> bool {
        match self.rng.gen_range(0..=4) {
            // Buy an animal
            0 => {
                if self.gold == 0 {
                    return true;
                } else if self.gold >= 3 {
                    if let Some(i) = self.shop.random_friend(&mut self.rng) {
                        let a = self.shop.animals[i].unwrap().animal;
                        if let Some(j) = self.team.random_compatible_slot(a, &mut self.rng) {
                            self.buy_animal(i, j);
                        }
                    }
                }
            }
            // Buy food
            1 => {
                if self.gold == 0 {
                    return true;
                } else if self.gold >= 3 {
                    let i = self.shop.random_food(&mut self.rng);
                    let j = self.team.random_friend(&mut self.rng);
                    if i.is_some() && j.is_some() {
                        self.buy_food(i.unwrap(), j.unwrap());
                    }
                }
            }
            // Sell friend
            2 => {
                if let Some(j) = self.team.random_friend(&mut self.rng) {
                    self.sell_animal(j);
                }
            }
            // Reroll
            3 => {
                if self.gold == 0 {
                    return true;
                } else {
                    trace!("Re-rolling shop");
                    self.shop = Shop::new(&mut self.rng);
                    self.gold -= 1;
                }
            }
            // Attempt to combine
            4 => {
                let mut targets = [[false; TEAM_SIZE]; TEAM_SIZE];
                let mut any_targets = false;
                for i in 0..TEAM_SIZE {
                    for j in (i + 1)..TEAM_SIZE {
                        let a = self.team[i];
                        let b = self.team[j];
                        if a.is_some() && b.is_some() && a.unwrap().animal == b.unwrap().animal {
                            targets[i][j] = true;
                            targets[j][i] = true;
                            any_targets = true;
                        }
                    }
                }
                if any_targets {
                    let i = loop {
                        let i = self.rng.gen_range(0..TEAM_SIZE);
                        if targets[i].iter().any(|i| *i) {
                            break i;
                        }
                    };
                    let j = loop {
                        let j = self.rng.gen_range(0..TEAM_SIZE);
                        if targets[i][j] {
                            break j;
                        }
                    };
                    let friend = self.team[i].take().unwrap();
                    trace!("Merging {} at {} into {}", friend.animal, i, j);
                    self.combine_animal(j, friend);
                }
            }
            i => panic!("Invalid random choice {}", i),
        }
        false
    }

    fn run_random(&mut self) {
        loop {
            if self.step_random() {
                break;
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Up to five animal friends.  The front of the team is at index 0, i.e.
/// attacking and defending first.
#[derive(Copy, Clone, Hash, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Team([Option<Friend>; TEAM_SIZE]);

impl Team {
    /// Asks the animal at position `i` to perform on-summon actions, with
    /// a new animal summoned at position `pos`
    fn on_summon(&mut self, i: usize, pos: usize) {
        assert!(i != pos);
        assert!(self[i].is_some());
        assert!(self[pos].is_some());

        match self[i].unwrap().animal {
            Animal::Horse => {
                // This is technically a temporary buf, but we're only
                // simulating a single turn here, so it doesn't matter.
                let f = self[pos].as_mut().unwrap();
                trace!(
                    "    🐴 at {} bufs {}  at {} by ❤️  +1, ⚔️  +1",
                    i,
                    f.animal,
                    pos
                );
                f.attack += 1;
                f.health += 1;
            }
            _ => (),
        }
    }

    fn count(&self) -> usize {
        self.0.iter().filter(|i| i.is_some()).count()
    }

    fn is_empty(&self) -> bool {
        self.count() == 0
    }

    fn deterministic_in_battle(&self) -> bool {
        self.0
            .iter()
            .flatten()
            .all(|f| f.animal.deterministic_in_battle())
    }

    /// Picks some number of random friends from the team, returning a mask.  If
    /// there are fewer than `n` members on the team, returns a smaller number.
    fn random_friends_mask<R: rand::Rng>(&self, mut n: usize, rng: &mut R) -> [bool; TEAM_SIZE] {
        n = std::cmp::min(self.count(), n);
        let mut out = [false; TEAM_SIZE];
        while n > 0 {
            let i = rng.gen_range(0..TEAM_SIZE);
            if !out[i] && self[i].is_some() {
                out[i] = true;
                n -= 1;
            }
        }
        out
    }

    /// Picks some number of random friends from the team, returning an iterator
    fn random_friends<R: rand::Rng>(&self, n: usize, rng: &mut R) -> impl Iterator<Item = usize> {
        self.random_friends_mask(n, rng)
            .into_iter()
            .enumerate()
            .filter(|b| b.1)
            .map(|b| b.0)
    }

    /// Returns a random friend's index, or `None` if the team is empty
    fn random_friend<R: rand::Rng>(&self, rng: &mut R) -> Option<usize> {
        if self.0.iter().all(|i| i.is_none()) {
            return None;
        }
        loop {
            let i = rng.gen_range(0..STORE_ANIMAL_COUNT);
            if self[i].is_some() {
                return Some(i);
            }
        }
    }

    /// Returns a random empty slot, or `None` if the team is full
    fn random_compatible_slot<R: rand::Rng>(&self, a: Animal, rng: &mut R) -> Option<usize> {
        if self.0.iter().all(|i| i.is_some()) {
            return None;
        }
        loop {
            let i = rng.gen_range(0..STORE_ANIMAL_COUNT);
            match self[i] {
                None => return Some(i),
                Some(f) => {
                    if f.animal == a {
                        return Some(i);
                    }
                }
            }
        }
    }

    /// Shuffles team members so they're tightly packed against 0
    fn compact(&mut self) {
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
                write!(f, "│ {} │ ", a.animal)?;
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
    /// Removes dead animals from the team, performing their on-death actions
    /// then compacting the team afterwards.
    fn remove_dead<R: Rng>(&mut self, rng: &mut R) {
        let mut changed = false;
        for i in 0..TEAM_SIZE {
            if self[i].is_some() && self[i].unwrap().health == 0 {
                let f = self[i].take().unwrap();
                trace!("{} at {} is dead, removing", f.animal, i);
                self.on_death(f, i, rng);
                changed = true;
            }
        }
        if changed {
            trace!("Compacting team");
            self.compact();
        }
    }
    fn on_death<R: Rng>(&mut self, f: Friend, i: usize, rng: &mut R) {
        assert!(self[i].is_none());
        match f.animal {
            Animal::Cricket => {
                trace!("Summoning ghost cricket at {}", i);
                self[i] = Some(Friend {
                    animal: Animal::GhostCricket,
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
            Animal::Ant => {
                if let Some(j) = self.random_friend(rng) {
                    let g = self[j].as_mut().unwrap();
                    let attack = f.level() * 2;
                    let health = f.level();
                    trace!(
                        "{} on death is buffing {} at {} by ❤️  +{}, ⚔️  + {}",
                        f.animal,
                        g.animal,
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
                    animal: Animal::Bee,
                    attack: 1,
                    health: 1,
                    modifier: None,
                    exp: 0,
                };
                if let Some(i) = self.make_space_at(i) {
                    trace!("Summoning {} at {}", bee.animal, i);
                    self[i] = Some(bee);
                    for j in 0..TEAM_SIZE {
                        if i != j && self[j].is_some() {
                            self.on_summon(j, i);
                        }
                    }
                } else {
                    trace!("No room to summon {}", bee.animal);
                }
            }
            None => (),
        }
    }
    /// Attempts to make space at the given position.  Returns the empty
    /// position, after shoving animals around, or None if the team is full.
    fn make_space_at(&mut self, i: usize) -> Option<usize> {
        if self.count() == TEAM_SIZE {
            None
        } else if self[i].is_none() {
            Some(i)
        } else {
            // Look for an empty slot behind the target slot, and shift
            // friends backwards (away from 0) to free up slot i
            for j in (i + 1)..TEAM_SIZE {
                if self[j].is_none() {
                    for k in (i..j).rev() {
                        assert!(self[k + 1].is_none());
                        self[k + 1] = self[k].take();
                    }
                    assert!(self[i].is_none());
                    return Some(i);
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
                    return Some(j);
                }
            }
            unreachable!()
        }
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

struct TeamPrinter<'a>(&'a Team, bool);

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

////////////////////////////////////////////////////////////////////////////////

fn random_team(seed: u64) -> Team {
    let mut rng = ChaChaRng::seed_from_u64(seed);
    let shop = Shop::new(&mut rng);
    let mut buy = BuyPhase {
        team: Team([None; TEAM_SIZE]),
        gold: 10,
        shop,
        rng,
    };
    buy.run_random();
    let mut team = buy.team;
    team.compact();
    team
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
enum Winner {
    TeamA,
    TeamB,
    Tied,
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
struct Battle(Team, Team);
impl Battle {
    /// Simulates a battle, returning the winner
    fn run(mut self) -> Winner {
        let seed = rand::thread_rng().gen();
        let mut rng = ChaChaRng::seed_from_u64(seed);

        trace!("Initial state:\n{}", self);
        self.before_battle(&mut rng);
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
                (false, false) => self.step(&mut rng),
            }
        }
        unreachable!();
    }

    /// Checks whether this battle is fully deterministic
    fn is_deterministic(&self) -> bool {
        self.0.deterministic_in_battle() && self.1.deterministic_in_battle()
    }

    /// Performs pre-battle actions, returning all possible states
    fn before_battle<R: Rng>(&mut self, rng: &mut R) {
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
    fn on_battle_start<R: Rng>(&mut self, i: usize, team: bool, rng: &mut R) {
        let f = match self[team][i] {
            Some(f) => f,
            None => return,
        };
        match f.animal {
            Animal::Mosquito => {
                for j in self[!team].random_friends(f.level(), rng) {
                    let g = self[!team][j].as_mut().unwrap();
                    trace!("{} at {} shot {} at {} for 1", f.animal, i, g.animal, j);
                    g.health = g.health.saturating_sub(1);
                }
            }
            _ => (),
        }
    }

    /// Executes a single step of the battle, returning true if the battle ended
    fn step<R: Rng>(&mut self, rng: &mut R) {
        let f = self.0[0].as_mut().unwrap();
        let g = self.1[0].as_mut().unwrap();
        trace!("{} clashes with {}!", f.animal, g.animal);
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

////////////////////////////////////////////////////////////////////////////////

const TEAMS_FILE: &str = "teams.ron";
const SCORES_FILE: &str = "scores.ron";

fn generate_teams() {
    let exit_flag = std::sync::Arc::new(AtomicBool::new(false));
    let exit_flag_copy = exit_flag.clone();
    ctrlc::set_handler(move || exit_flag_copy.store(true, Ordering::Release)).unwrap();
    let mut seen = HashSet::new();
    for i in 0.. {
        // Check the flag periodically for Ctrl-C
        if i % 100 == 0 && exit_flag.load(Ordering::Acquire) {
            break;
        }
        let seed = rand::thread_rng().gen();
        let team = random_team(seed);
        if seen.insert(team) {
            debug!("New team [{}]:\n{}", seed, team);
        }
        if i % 1000000 == 0 {
            debug!("{} [{}]", i, seen.len());
        }
    }
    info!("Saving {} teams to '{}'", seen.len(), TEAMS_FILE);
    let seen: Vec<Team> = seen.into_iter().collect();
    std::fs::write(
        TEAMS_FILE,
        ron::to_string(&seen).expect("Failed to serialize teams"),
    )
    .expect("Failed to save teams");
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
struct Record {
    wins: f32,
    ties: f32,
}
fn score_teams(teams: Vec<Team>) {
    let mut results: HashMap<usize, HashMap<usize, Record>> = HashMap::new();
    for (i, a) in teams.iter().enumerate() {
        for (j, b) in teams.iter().enumerate() {
            let battle = Battle(*a, *b);
            let num_battles = if battle.is_deterministic() { 1 } else { 100 };

            let mut team_a = 0.0;
            let mut team_b = 0.0;
            let mut ties = 0.0;
            for _ in 0..num_battles {
                match battle.run() {
                    Winner::TeamA => team_a += 1.0,
                    Winner::TeamB => team_b += 1.0,
                    Winner::Tied => ties += 1.0,
                }
            }
            let num_battles = num_battles as f32;
            results.entry(i).or_default().insert(
                j,
                Record {
                    wins: team_a / num_battles,
                    ties: ties / num_battles,
                },
            );
            results.entry(j).or_default().insert(
                i,
                Record {
                    wins: team_b / num_battles,
                    ties: ties / num_battles,
                },
            );
        }
        let mut num_wins = 0.0;
        let mut num_ties = 0.0;
        let mut count = 0.0;
        for r in results[&i].values() {
            num_wins += r.wins;
            num_ties += r.ties;
            count += 1.0;
        }
        debug!(
            "Team {} wins {:.1}% and draws {:.1}%:\n{}",
            i,
            num_wins / count * 100.0,
            num_ties / count * 100.0,
            teams[i]
        );
    }
    std::fs::write(
        SCORES_FILE,
        ron::to_string(&results).expect("Failed to serialize scores"),
    )
    .expect("Failed to save scores");
    // TODO
}

fn analyze_scores(teams: Vec<Team>, results: HashMap<usize, HashMap<usize, Record>>) {
    let mut most_wins = 0.0;
    let mut best_team = 0;

    for (k, v) in &results {
        let mut num_wins = 0.0;
        let mut num_ties = 0.0;
        let mut count = 0.0;
        for r in v.values() {
            num_wins += r.wins;
            num_ties += r.ties;
            count += 1.0;
        }
        if num_wins / count > most_wins {
            best_team = *k;
            most_wins = num_wins / count;
        }
    }
    println!(
        "The team with the most wins ({:.2}%):\n{}",
        most_wins * 100.0,
        teams[best_team]
    );
}

fn main() {
    use env_logger::Builder;

    let mut log = Builder::new();

    let args = std::env::args();
    match args.len() {
        1 => {
            log.filter_level(LevelFilter::Debug);
            log.parse_env("RUST_LOG");
            log.init();

            if let Ok(d) = std::fs::read_to_string(TEAMS_FILE) {
                let teams: Vec<Team> = ron::from_str(&d).unwrap();
                if let Ok(d) = std::fs::read_to_string(SCORES_FILE) {
                    let scores = ron::from_str(&d).unwrap();
                    analyze_scores(teams, scores);
                } else {
                    score_teams(teams);
                }
            } else {
                generate_teams();
            }
        }
        2 => {
            // By default, when asked to generate a team, print the verbose
            // team generation log.
            log.filter_level(LevelFilter::Trace);
            log.parse_env("RUST_LOG");
            log.init();

            let seed = args.last().unwrap().parse().unwrap();
            let team = random_team(seed);
            debug!("Got team [{}]:\n{}", seed, team);
        }
        i => {
            panic!("Invalid argument count {}", i);
        }
    }
}
