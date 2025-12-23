//! Layout utilities.

use std::alloc::Layout;

/// Align a size up to the given alignment.
#[inline]
pub const fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

/// Align a pointer up to the given alignment.
#[inline]
pub fn align_ptr(ptr: *mut u8, align: usize) -> *mut u8 {
    let addr = ptr as usize;
    let aligned = align_up(addr, align);
    aligned as *mut u8
}

/// Calculate padding needed to align a size.
#[inline]
pub const fn padding_for(size: usize, align: usize) -> usize {
    let aligned = align_up(size, align);
    aligned - size
}

/// Create a layout for an array of T with the given count.
pub fn array_layout<T>(count: usize) -> Option<Layout> {
    Layout::array::<T>(count).ok()
}

/// Create a layout for a type T.
pub fn type_layout<T>() -> Layout {
    Layout::new::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 8), 0);
        assert_eq!(align_up(1, 8), 8);
        assert_eq!(align_up(8, 8), 8);
        assert_eq!(align_up(9, 8), 16);
    }

    #[test]
    fn test_padding_for() {
        assert_eq!(padding_for(0, 8), 0);
        assert_eq!(padding_for(1, 8), 7);
        assert_eq!(padding_for(8, 8), 0);
        assert_eq!(padding_for(9, 8), 7);
    }
}
