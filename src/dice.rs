#[derive(Debug)]
pub struct DeterministicDice {
    initialized: bool,
    index: usize,
    data: Vec<(usize, usize, usize)>,
}

impl DeterministicDice {
    pub fn new() -> Self {
        Self {
            initialized: false,
            index: 0,
            data: vec![],
        }
    }

    pub fn key(&self) -> Vec<usize> {
        self.data.iter().map(|v| v.2).collect()
    }

    pub fn next(&mut self) -> bool {
        if !self.initialized {
            self.initialized = true;
            true
        } else {
            while let Some((lo, hi, mut v)) = self.data.pop() {
                v += 1;
                if v >= hi {
                    continue;
                } else {
                    self.data.push((lo, hi, v));
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
        //println!("    {:?} {:?}", range, self);
        let out = if let Some((lo, hi, v)) = self.data.get(self.index) {
            assert!(*lo == range.start);
            assert!(*hi == range.end);
            assert!(*v >= range.start);
            assert!(*v < range.end);
            *v
        } else {
            self.data.push((range.start, range.end, range.start));
            range.start
        };
        //println!("    => {}", out);
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
