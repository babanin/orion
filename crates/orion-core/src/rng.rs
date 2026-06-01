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
