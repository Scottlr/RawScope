//! A tiny deterministic RNG for synthetic fixtures.

/// Deterministic SplitMix64-based RNG for reproducible fixture generation.
#[derive(Debug, Clone)]
pub struct SyntheticRng {
    state: u64,
}

impl SyntheticRng {
    /// Creates a deterministic RNG from the provided seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns the next pseudo-random `u64`.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns a value in `[0, 1)`.
    pub fn next_unit_f32(&mut self) -> f32 {
        let value = self.next_u64() >> 40;
        (value as f32) / ((1u32 << 24) as f32)
    }

    /// Returns a value in the provided range.
    pub fn f32_in_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_unit_f32() * (max - min)
    }

    /// Returns a value in the provided integer range.
    pub fn u32_in_range(&mut self, min: u32, max_exclusive: u32) -> u32 {
        assert!(max_exclusive > min, "u32 range must be non-empty");
        min + (self.next_u64() % (max_exclusive - min) as u64) as u32
    }

    /// Returns a value in the provided integer range.
    pub fn u64_in_range(&mut self, min: u64, max_exclusive: u64) -> u64 {
        assert!(max_exclusive > min, "u64 range must be non-empty");
        min + (self.next_u64() % (max_exclusive - min))
    }
}
