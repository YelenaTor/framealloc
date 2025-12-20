//! Mutex wrapper - uses parking_lot if available, std otherwise.

#[cfg(feature = "parking_lot")]
pub use parking_lot::{Mutex, MutexGuard};

#[cfg(not(feature = "parking_lot"))]
mod std_mutex {
    use std::sync::{Mutex as StdMutex, MutexGuard as StdMutexGuard};

    /// Thin wrapper around std::sync::Mutex.
    pub struct Mutex<T>(StdMutex<T>);

    impl<T> Mutex<T> {
        /// Create a new mutex.
        pub const fn new(value: T) -> Self {
            Self(StdMutex::new(value))
        }

        /// Lock the mutex.
        pub fn lock(&self) -> MutexGuard<'_, T> {
            MutexGuard(self.0.lock().expect("Mutex poisoned"))
        }
    }

    /// Guard for std mutex.
    pub struct MutexGuard<'a, T>(StdMutexGuard<'a, T>);

    impl<'a, T> std::ops::Deref for MutexGuard<'a, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'a, T> std::ops::DerefMut for MutexGuard<'a, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}

#[cfg(not(feature = "parking_lot"))]
pub use std_mutex::Mutex;
