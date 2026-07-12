//! Owner-minted generation tokens for stale-result protection.

use std::{marker::PhantomData, num::NonZeroU64};

/// A comparable generation token whose owner marker prevents accidental mixing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Generation<Owner> {
    value: NonZeroU64,
    marker: PhantomData<fn() -> Owner>,
}

impl<Owner> Generation<Owner> {
    pub const fn get(self) -> u64 {
        self.value.get()
    }
}

/// The only constructor for a marked generation token.
#[derive(Debug, Clone, Copy)]
pub struct GenerationCounter<Owner> {
    next: u64,
    marker: PhantomData<fn() -> Owner>,
}

impl<Owner> Default for GenerationCounter<Owner> {
    fn default() -> Self {
        Self {
            next: 0,
            marker: PhantomData,
        }
    }
}

impl<Owner> GenerationCounter<Owner> {
    pub fn next(&mut self) -> Generation<Owner> {
        self.next = self.next.saturating_add(1).max(1);
        Generation {
            value: NonZeroU64::new(self.next).expect("generation counter starts at one"),
            marker: PhantomData,
        }
    }
}
