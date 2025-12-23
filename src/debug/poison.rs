//! Memory poisoning for debugging.
//!
//! Fills freed memory with known patterns to detect use-after-free.

/// Pattern used to poison freed memory.
pub const FREED_PATTERN: u8 = 0xCD;

/// Pattern used to poison uninitialized memory.
pub const UNINIT_PATTERN: u8 = 0xAB;

/// Pattern used to poison guard bytes.
pub const GUARD_PATTERN: u8 = 0xFD;

/// Poison a region of memory with the freed pattern.
///
/// # Safety
///
/// The memory region must be valid and writable.
pub unsafe fn poison_freed(ptr: *mut u8, size: usize) {
    std::ptr::write_bytes(ptr, FREED_PATTERN, size);
}

/// Poison a region of memory with the uninitialized pattern.
///
/// # Safety
///
/// The memory region must be valid and writable.
pub unsafe fn poison_uninit(ptr: *mut u8, size: usize) {
    std::ptr::write_bytes(ptr, UNINIT_PATTERN, size);
}

/// Check if a region appears to be poisoned with freed pattern.
///
/// Returns true if all bytes match the freed pattern.
pub fn is_freed_poison(ptr: *const u8, size: usize) -> bool {
    for i in 0..size {
        // SAFETY: Caller guarantees valid memory region
        let byte = unsafe { *ptr.add(i) };
        if byte != FREED_PATTERN {
            return false;
        }
    }
    true
}

/// Check for potential use-after-free.
///
/// Returns true if the memory appears to have been freed.
pub fn check_use_after_free(ptr: *const u8, size: usize) -> bool {
    // Check first few bytes for freed pattern
    let check_size = size.min(16);
    is_freed_poison(ptr, check_size)
}
