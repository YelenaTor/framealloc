//! Atomic helpers for statistics and counters.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// An atomic counter for statistics.
pub struct AtomicCounter(AtomicU64);

impl AtomicCounter {
    /// Create a new counter.
    pub const fn new(initial: u64) -> Self {
        Self(AtomicU64::new(initial))
    }

    /// Increment the counter.
    pub fn increment(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// Add a value to the counter.
    pub fn add(&self, value: u64) {
        self.0.fetch_add(value, Ordering::Relaxed);
    }

    /// Get the current value.
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    /// Reset to zero.
    pub fn reset(&self) {
        self.0.store(0, Ordering::Relaxed);
    }
}

impl Default for AtomicCounter {
    fn default() -> Self {
        Self::new(0)
    }
}

/// An atomic gauge for tracking current values (can go up or down).
pub struct AtomicGauge(AtomicUsize);

impl AtomicGauge {
    /// Create a new gauge.
    pub const fn new(initial: usize) -> Self {
        Self(AtomicUsize::new(initial))
    }

    /// Add to the gauge.
    pub fn add(&self, value: usize) -> usize {
        self.0.fetch_add(value, Ordering::Relaxed) + value
    }

    /// Subtract from the gauge.
    pub fn sub(&self, value: usize) -> usize {
        self.0.fetch_sub(value, Ordering::Relaxed) - value
    }

    /// Get the current value.
    pub fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    /// Set the value.
    pub fn set(&self, value: usize) {
        self.0.store(value, Ordering::Relaxed);
    }

    /// Update the maximum (for high-water marks).
    pub fn update_max(&self, value: usize) {
        let mut current = self.0.load(Ordering::Relaxed);
        while value > current {
            match self.0.compare_exchange_weak(
                current,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
    }
}

impl Default for AtomicGauge {
    fn default() -> Self {
        Self::new(0)
    }
}
