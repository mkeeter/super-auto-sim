#[derive(Debug)]
pub struct Rng {
    initialized: bool,
    index: usize,
    data: Vec<(usize, usize, usize)>,
}

impl Rng {
    pub fn new() -> Self {
        Self {
            initialized: false,
            index: 0,
            data: vec![],
        }
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

pub trait RangeRng {
    fn gen_range(&mut self, range: std::ops::Range<usize>) -> usize;
}

impl<R: rand::Rng> RangeRng for R {
    fn gen_range(&mut self, range: std::ops::Range<usize>) -> usize {
        rand::Rng::gen_range(self, range)
    }
}

impl RangeRng for Rng {
    fn gen_range(&mut self, range: std::ops::Range<usize>) -> usize {
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
        self.index += 1;
        out
    }
}

////////////////////////////////////////////////////////////////////////////////

/// Chooses up to `n` items from `vs`, returning indices `i` where `f(vs[i])`
pub fn pick_where<'a, 'b, R: RangeRng, T, F: Fn(&T) -> bool>(
    rng: &'a mut R,
    mut n: usize,
    vs: &'b [T],
    f: F,
) -> impl Iterator<Item = usize> + 'a {
    let mut mask: Vec<bool> = vs.iter().map(|v| f(v)).collect();
    let count = mask.iter().filter(|i| **i).count();
    n = std::cmp::min(n, count);

    (0..n).map(move |i| {
        let j = rng.gen_range(0..(count - i));
        let k = mask.iter().enumerate().filter(|i| *i.1).nth(j).unwrap().0;
        mask[k] = false;
        k
    })
}

pub fn pick_one_where<R: RangeRng, T, F: Fn(&T) -> bool>(
    rng: &mut R,
    vs: &[T],
    f: F,
) -> Option<usize> {
    pick_where(rng, 1, vs, f).next()
}

pub fn pick_some<'a, 'b, R: RangeRng, T: 'a>(
    rng: &'a mut R,
    n: usize,
    vs: &'b [Option<T>],
) -> impl Iterator<Item = usize> + 'a {
    pick_where(rng, n, vs, Option::is_some)
}

pub fn pick_one_some<R: RangeRng, T>(
    rng: &mut R,
    vs: &[Option<T>],
) -> Option<usize> {
    pick_some(rng, 1, vs).next()
}
