use std::collections::HashSet;

use enum_iterator::IntoEnumIterator;
use log::trace;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;

const TEAM_SIZE: usize = 5;
const STORE_ANIMAL_COUNT: usize = 3;
const STORE_FOOD_COUNT: usize = 1;

////////////////////////////////////////////////////////////////////////////////

/// Tier 1 animals in the free-to-play pack
#[derive(Copy, Clone, Debug, Eq, Hash, IntoEnumIterator, PartialEq)]
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

    /// Returns the default health and attack for an animal, or an error
    /// if the animal can't be bought from the shop.
    fn default_power(&self) -> (u32, u32) {
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

    /// Returns the default modifier for the animal, which is `None` for all
    /// Tier 1 units.
    fn default_modifier(&self) -> Option<Modifier> {
        None
    }
}

impl std::fmt::Display for Animal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.emoji())
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

#[derive(Copy, Clone, Debug, Eq, Hash, IntoEnumIterator, PartialEq)]
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

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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
#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
struct Friend {
    animal: Animal,
    attack: u32,
    health: u32,
    modifier: Option<Modifier>,
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
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
struct Shop {
    animals: [Option<Friend>; STORE_ANIMAL_COUNT],
    foods: [Option<Food>; STORE_FOOD_COUNT],
}

impl Shop {
    fn new() -> Self {
        let mut animals = [None; STORE_ANIMAL_COUNT];
        for a in animals.iter_mut() {
            *a = Some(Friend::new(rand::random()));
        }
        let mut foods = [None; STORE_FOOD_COUNT];
        for f in foods.iter_mut() {
            *f = Some(rand::random());
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

#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
struct BuyPhase<R: rand::Rng> {
    team: Team,
    gold: usize,
    shop: Shop,
    rng: R,
}

impl<R: rand::Rng> BuyPhase<R> {
    /// Buys the animal at `shop_pos` and adds it to `team_pos`
    fn buy_animal(&mut self, shop_pos: usize, team_pos: usize) {
        // TODO: combining animals!
        assert!(self.team.0[team_pos].is_none());
        assert!(self.gold >= 3);

        self.gold -= 3;
        let friend = self.shop.animals[shop_pos].take().unwrap();
        trace!("Buying {} at position {}", friend.animal, team_pos);
        self.on_buy(friend);
        self.team.0[team_pos] = Some(friend);

        for i in 0..TEAM_SIZE {
            if i != team_pos && self.team.0[i].is_some() {
                self.team.on_summon(i, team_pos);
            }
        }

        // XXX: There are also buy triggers, but nothing in Tier 1 uses them
    }

    fn sell_animal(&mut self, team_pos: usize) {
        assert!(self.team.0[team_pos].is_some());

        // XXX: This changes depending on level
        self.gold += 1;

        let a = self.team.0[team_pos].take().unwrap();
        trace!("Selling {} at position {}", a.animal, team_pos);
        self.on_sell(a);
        for i in 0..TEAM_SIZE {
            if i != team_pos && self.team.0[i].is_some() {
                self.on_sold(i);
            }
        }
    }

    /// Purchases food at the given shop position, applying it to the chosen
    /// member of the team.
    fn buy_food(&mut self, shop_pos: usize, team_pos: usize) {
        assert!(self.shop.foods[shop_pos].is_some());
        assert!(self.team.0[team_pos].is_some());

        let friend = self.team.0[team_pos].as_mut().unwrap();
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
                trace!("    Buffing by (+1, +1)");
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
                    trace!("    🦦 on buy bufs {} at {} by (+1, +1)", f.animal, i);
                    let f = self.team.0[i].as_mut().unwrap();
                    f.health += 1;
                    f.attack += 1;
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
                for i in self.team.random_friends(2, &mut self.rng) {
                    let f = self.team.0[i].as_mut().unwrap();
                    trace!("    🦫 on sell bufs {} at {} by (+1, +1)", f.animal, i);
                    f.health += 1;
                }
            }
            Animal::Duck => {
                // Give shop pets +1 Health
                for f in self.shop.animals.iter_mut().flatten() {
                    trace!("    🦆 on sell bufs {} in shop by (+1, +0)", f.animal);
                    f.health += 1;
                }
            }
            Animal::Pig => {
                // Give player +1 gold
                self.gold += 1;
            }
            _ => (),
        }
    }

    fn on_sold(&mut self, _i: usize) {
        // No Tier 1 animals have an on-sold trigger
    }

    fn step_random(&mut self) -> bool {
        match self.rng.gen_range(0..4) {
            // Buy an animal
            0 => {
                if self.gold == 0 {
                    return true;
                } else if self.gold >= 3 {
                    let i = self.shop.random_friend(&mut self.rng);
                    let j = self.team.random_empty_slot(&mut self.rng);
                    if i.is_some() && j.is_some() {
                        self.buy_animal(i.unwrap(), j.unwrap());
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
                    self.shop = Shop::new();
                    self.gold -= 1;
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
#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
struct Team([Option<Friend>; TEAM_SIZE]);

impl Team {
    /// Asks the animal at position `i` to perform on-summon actions, with
    /// a new animal summoned at position `pos`
    fn on_summon(&mut self, i: usize, pos: usize) {
        assert!(i != pos);
        assert!(self.0[i].is_some());
        assert!(self.0[pos].is_some());

        match self.0[i].unwrap().animal {
            Animal::Horse => {
                // This is technically a temporary buf, but we're only
                // simulating a single turn here, so it doesn't matter.
                let f = self.0[pos].as_mut().unwrap();
                trace!("    🐴 at {} bufs {} at {} by (+1, +1)", i, f.animal, pos);
                f.attack += 1;
                f.health += 1;
            }
            _ => (),
        }
    }

    fn count(&self) -> usize {
        self.0.iter().filter(|i| i.is_some()).count()
    }

    /// Picks some number of random friends from the team, returning a mask.  If
    /// there are fewer than `n` members on the team, returns a smaller number.
    fn random_friends_mask<R: rand::Rng>(&self, mut n: usize, rng: &mut R) -> [bool; TEAM_SIZE] {
        n = std::cmp::min(self.count(), n);
        let mut out = [false; TEAM_SIZE];
        while n > 0 {
            let i = rng.gen_range(0..TEAM_SIZE);
            if !out[i] && self.0[i].is_some() {
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
            if self.0[i].is_some() {
                return Some(i);
            }
        }
    }

    /// Returns a random empty slot, or `None` if the team is full
    fn random_empty_slot<R: rand::Rng>(&self, rng: &mut R) -> Option<usize> {
        if self.0.iter().all(|i| i.is_some()) {
            return None;
        }
        loop {
            let i = rng.gen_range(0..STORE_ANIMAL_COUNT);
            if self.0[i].is_none() {
                return Some(i);
            }
        }
    }

    /// Shuffles team members so they're tightly packed against 0
    fn compact(&mut self) {
        let mut i = 0;
        loop {
            while i < TEAM_SIZE && self.0[i].is_some() {
                i += 1;
            }
            let mut j = i;
            while j < TEAM_SIZE && self.0[j].is_none() {
                j += 1;
            }
            if i >= TEAM_SIZE || j >= TEAM_SIZE {
                break;
            }
            self.0[i] = self.0[j].take();
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

fn random_team(seed: u64) -> Team {
    let mut buy = BuyPhase {
        team: Team([None; TEAM_SIZE]),
        gold: 10,
        shop: Shop::new(),
        rng: ChaChaRng::seed_from_u64(seed),
    };
    buy.run_random();
    let mut team = buy.team;
    team.compact();
    team
}
fn main() {
    let mut seen = HashSet::new();
    env_logger::init();
    for i in 0.. {
        let seed = rand::thread_rng().gen();
        let team = random_team(seed);
        if seen.insert(team) {
            println!("New team [{}]: {:?}", seed, team);
        }
        if i % 10000 == 0 {
            println!("{} [{}]", i, seen.len());
        }
    }
}
