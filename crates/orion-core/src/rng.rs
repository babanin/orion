pub trait Rng {
    fn next_u32(&mut self) -> u32;

    fn index(&mut self, limit: usize) -> usize {
        debug_assert!(limit > 0);
        (self.next_u32() as usize) % limit
    }
}

#[derive(Debug, Clone)]
pub struct LcgRng {
    state: u32,
}

impl LcgRng {
    pub const fn new(seed: u32) -> Self {
        Self { state: seed }
    }
}

impl Rng for LcgRng {
    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state
    }
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct ScriptedRng {
    values: Vec<u32>,
    cursor: usize,
}

#[cfg(test)]
impl ScriptedRng {
    pub fn new(values: impl Into<Vec<u32>>) -> Self {
        Self {
            values: values.into(),
            cursor: 0,
        }
    }
}

#[cfg(test)]
impl Rng for ScriptedRng {
    fn next_u32(&mut self) -> u32 {
        let value = self.values[self.cursor % self.values.len()];
        self.cursor += 1;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcg_rng_produces_deterministic_sequence() {
        let mut rng = LcgRng::new(1);
        let a = rng.next_u32();
        let b = rng.next_u32();
        assert_ne!(a, b);
        let mut rng2 = LcgRng::new(1);
        assert_eq!(rng2.next_u32(), a);
        assert_eq!(rng2.next_u32(), b);
    }

    #[test]
    fn lcg_rng_index_returns_values_within_limit() {
        let mut rng = LcgRng::new(42);
        for _ in 0..100 {
            let idx = rng.index(10);
            assert!(idx < 10);
        }
    }

    #[test]
    fn scripted_rng_cycles_values() {
        let mut rng = ScriptedRng::new([7u32, 3, 9]);
        assert_eq!(rng.next_u32(), 7);
        assert_eq!(rng.next_u32(), 3);
        assert_eq!(rng.next_u32(), 9);
        assert_eq!(rng.next_u32(), 7);
        assert_eq!(rng.next_u32(), 3);
    }

    #[test]
    fn scripted_rng_index_modulates_by_limit() {
        let mut rng = ScriptedRng::new([17u32]);
        assert_eq!(rng.index(5), 17 % 5);
    }
}
