#[derive(Debug)]
pub struct DeterministicDice {
    initialized: bool,
    index: usize,
    data: Vec<(usize, std::ops::Range<usize>)>,
}

impl DeterministicDice {
    pub fn new() -> Self {
        Self {
            initialized: false,
            index: 0,
            data: vec![],
        }
    }

    /// Converts the given DeterministicDice state into a string key.
    /// Panics if any of the choices can't be represented as a single
    /// base-36 number.
    pub fn key(&self) -> String {
        self.data
            .iter()
            .map(|v| char::from_digit(v.0.try_into().unwrap(), 36).unwrap())
            .collect::<String>()
    }

    pub fn from_key(s: &str) -> Self {
        Self {
            initialized: true,
            index: 0,
            data: s
                .chars()
                .map(|c| (char::to_digit(c, 36).unwrap() as usize, 0..0))
                .collect(),
        }
    }

    pub fn next(&mut self) -> bool {
        if !self.initialized {
            self.initialized = true;
            true
        } else {
            while let Some((mut v, r)) = self.data.pop() {
                v += 1;
                if v >= r.end {
                    continue;
                } else {
                    self.data.push((v, r));
                    break;
                }
            }
            self.index = 0;
            !self.data.is_empty()
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub trait Dice {
    fn roll(&mut self, range: std::ops::Range<usize>) -> usize;
}

impl<R: rand::Rng> Dice for R {
    fn roll(&mut self, range: std::ops::Range<usize>) -> usize {
        rand::Rng::gen_range(self, range)
    }
}

impl Dice for DeterministicDice {
    fn roll(&mut self, range: std::ops::Range<usize>) -> usize {
        let out = if let Some((v, r)) = self.data.get_mut(self.index) {
            // Special-case if a DeterministicDice has been loaded from a
            // key, which doesn't preserve ranges (to keep small).
            if (*r).is_empty() {
                *r = range.clone();
            }
            assert!(*r == range);
            assert!(range.contains(v));
            *v
        } else {
            self.data.push((range.start, range.clone()));
            range.start
        };
        self.index += 1;
        out
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Chooses up to `n` items from `vs`, returning indices `i` where `f(vs[i])`
pub fn pick_some<'a, 'b, D: Dice, T: 'a>(
    dice: &'a mut D,
    mut n: usize,
    vs: &'b [Option<T>],
) -> impl Iterator<Item = usize> + 'a {
    let mut mask: Vec<bool> = vs.iter().map(Option::is_some).collect();
    let count = mask.iter().filter(|i| **i).count();
    n = std::cmp::min(n, count);

    (0..n).map(move |i| {
        let j = dice.roll(0..(count - i));
        let k = mask.iter().enumerate().filter(|i| *i.1).nth(j).unwrap().0;
        mask[k] = false;
        k
    })
}

pub fn pick_one<D: Dice, T>(dice: &mut D, vs: &[Option<T>]) -> Option<usize> {
    pick_some(dice, 1, vs).next()
}
