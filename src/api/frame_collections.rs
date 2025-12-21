//! Frame-local collections - bounded collections that live for a frame.
//!
//! These are explicitly frame-bound and cannot escape the frame.
//! They provide familiar collection APIs with frame allocation semantics.
//!
//! # Thread Safety
//!
//! Frame collections are explicitly `!Send` and `!Sync` because they reference
//! thread-local frame memory. Moving them across threads would be undefined behavior.

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice;

/// Marker type that is !Send and !Sync.
/// Used to prevent frame collections from crossing thread boundaries.
struct NotSendSync(*const ());

/// A frame-allocated vector with fixed capacity.
///
/// Unlike `Vec`, this:
/// - Has a fixed capacity set at creation
/// - Cannot be reallocated
/// - Is automatically freed at frame end
/// - Cannot escape the frame (lifetime-bound)
/// - **Cannot be sent across threads** (uses thread-local memory)
///
/// # Example
///
/// ```rust,ignore
/// let mut list = alloc.frame_vec::<Entity>(128);
/// list.push(entity1);
/// list.push(entity2);
/// for entity in list.iter() {
///     process(entity);
/// }
/// // Freed automatically at end_frame()
/// ```
pub struct FrameVec<'a, T> {
    ptr: *mut T,
    len: usize,
    capacity: usize,
    _marker: PhantomData<&'a mut T>,
    /// Prevents Send/Sync - frame memory is thread-local
    _not_send_sync: PhantomData<NotSendSync>,
}

impl<'a, T> FrameVec<'a, T> {
    /// Create a new FrameVec from raw parts.
    ///
    /// # Safety
    ///
    /// The pointer must be valid for the lifetime 'a and have
    /// space for `capacity` elements of type T.
    pub(crate) unsafe fn from_raw_parts(ptr: *mut T, capacity: usize) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }
        Some(Self {
            ptr,
            len: 0,
            capacity,
            _marker: PhantomData,
            _not_send_sync: PhantomData,
        })
    }

    /// Returns the number of elements in the vector.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the maximum capacity of the vector.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the remaining capacity.
    pub fn remaining(&self) -> usize {
        self.capacity - self.len
    }

    /// Returns true if the vector is full.
    pub fn is_full(&self) -> bool {
        self.len >= self.capacity
    }

    /// Push an element onto the vector.
    ///
    /// Returns `Err(value)` if the vector is full.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.is_full() {
            return Err(value);
        }
        unsafe {
            self.ptr.add(self.len).write(value);
        }
        self.len += 1;
        Ok(())
    }

    /// Pop an element from the vector.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        self.len -= 1;
        unsafe { Some(self.ptr.add(self.len).read()) }
    }

    /// Get a reference to an element.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        unsafe { Some(&*self.ptr.add(index)) }
    }

    /// Get a mutable reference to an element.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        unsafe { Some(&mut *self.ptr.add(index)) }
    }

    /// Clear the vector.
    pub fn clear(&mut self) {
        // Drop all elements
        for i in 0..self.len {
            unsafe {
                std::ptr::drop_in_place(self.ptr.add(i));
            }
        }
        self.len = 0;
    }

    /// Get a slice of the elements.
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Get a mutable slice of the elements.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    /// Iterate over the elements.
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Iterate mutably over the elements.
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    /// Try to extend from an iterator.
    ///
    /// Returns the number of elements added.
    pub fn extend_from_iter<I: IntoIterator<Item = T>>(&mut self, iter: I) -> usize {
        let mut count = 0;
        for item in iter {
            if self.push(item).is_err() {
                break;
            }
            count += 1;
        }
        count
    }

    /// Retain only elements that satisfy the predicate.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut write = 0;
        for read in 0..self.len {
            unsafe {
                let elem = &*self.ptr.add(read);
                if f(elem) {
                    if write != read {
                        std::ptr::copy_nonoverlapping(self.ptr.add(read), self.ptr.add(write), 1);
                    }
                    write += 1;
                } else {
                    std::ptr::drop_in_place(self.ptr.add(read));
                }
            }
        }
        self.len = write;
    }
}

impl<'a, T> Deref for FrameVec<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<'a, T> DerefMut for FrameVec<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<'a, T> Index<usize> for FrameVec<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("index out of bounds")
    }
}

impl<'a, T> IndexMut<usize> for FrameVec<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("index out of bounds")
    }
}

impl<'a, T> Drop for FrameVec<'a, T> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<'a, T> IntoIterator for FrameVec<'a, T> {
    type Item = T;
    type IntoIter = FrameVecIntoIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        FrameVecIntoIter {
            vec: self,
            index: 0,
        }
    }
}

/// Consuming iterator for FrameVec.
pub struct FrameVecIntoIter<'a, T> {
    vec: FrameVec<'a, T>,
    index: usize,
}

impl<'a, T> Iterator for FrameVecIntoIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.vec.len {
            return None;
        }
        let item = unsafe { self.vec.ptr.add(self.index).read() };
        self.index += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vec.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a, T> ExactSizeIterator for FrameVecIntoIter<'a, T> {}

impl<'a, T> Drop for FrameVecIntoIter<'a, T> {
    fn drop(&mut self) {
        // Drop remaining elements
        for i in self.index..self.vec.len {
            unsafe {
                std::ptr::drop_in_place(self.vec.ptr.add(i));
            }
        }
        // Prevent FrameVec from double-dropping
        self.vec.len = 0;
    }
}

/// A frame-allocated hash map with fixed capacity.
///
/// Simple open-addressing hash map for frame-temporary lookups.
/// **Cannot be sent across threads** (uses thread-local memory).
pub struct FrameMap<'a, K, V> {
    keys: *mut Option<K>,
    values: *mut V,
    len: usize,
    capacity: usize,
    _marker: PhantomData<&'a mut (K, V)>,
    /// Prevents Send/Sync - frame memory is thread-local
    _not_send_sync: PhantomData<NotSendSync>,
}

impl<'a, K: Eq + std::hash::Hash, V> FrameMap<'a, K, V> {
    /// Create a new FrameMap from raw parts.
    ///
    /// # Safety
    ///
    /// Pointers must be valid for the lifetime 'a.
    pub(crate) unsafe fn from_raw_parts(
        keys: *mut Option<K>,
        values: *mut V,
        capacity: usize,
    ) -> Option<Self> {
        if keys.is_null() || values.is_null() {
            return None;
        }
        // Initialize keys to None
        for i in 0..capacity {
            keys.add(i).write(None);
        }
        Some(Self {
            keys,
            values,
            len: 0,
            capacity,
            _marker: PhantomData,
            _not_send_sync: PhantomData,
        })
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    fn hash_index(&self, key: &K) -> usize {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish() as usize % self.capacity
    }

    /// Insert a key-value pair.
    ///
    /// Returns `Err((key, value))` if full.
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, (K, V)> {
        if self.len >= self.capacity * 3 / 4 {
            return Err((key, value));
        }

        let mut index = self.hash_index(&key);
        for _ in 0..self.capacity {
            unsafe {
                let slot = &mut *self.keys.add(index);
                match slot {
                    None => {
                        *slot = Some(key);
                        self.values.add(index).write(value);
                        self.len += 1;
                        return Ok(None);
                    }
                    Some(k) if k == &key => {
                        let old = self.values.add(index).read();
                        self.values.add(index).write(value);
                        return Ok(Some(old));
                    }
                    _ => {
                        index = (index + 1) % self.capacity;
                    }
                }
            }
        }
        Err((key, value))
    }

    /// Get a value by key.
    pub fn get(&self, key: &K) -> Option<&V> {
        let mut index = self.hash_index(key);
        for _ in 0..self.capacity {
            unsafe {
                let slot = &*self.keys.add(index);
                match slot {
                    None => return None,
                    Some(k) if k == key => return Some(&*self.values.add(index)),
                    _ => {
                        index = (index + 1) % self.capacity;
                    }
                }
            }
        }
        None
    }

    /// Get a mutable value by key.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut index = self.hash_index(key);
        for _ in 0..self.capacity {
            unsafe {
                let slot = &*self.keys.add(index);
                match slot {
                    None => return None,
                    Some(k) if k == key => return Some(&mut *self.values.add(index)),
                    _ => {
                        index = (index + 1) % self.capacity;
                    }
                }
            }
        }
        None
    }

    /// Check if a key exists.
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }
}

impl<'a, K, V> Drop for FrameMap<'a, K, V> {
    fn drop(&mut self) {
        for i in 0..self.capacity {
            unsafe {
                let slot = &mut *self.keys.add(i);
                if slot.is_some() {
                    std::ptr::drop_in_place(slot);
                    std::ptr::drop_in_place(self.values.add(i));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_vec_basic() {
        let mut buffer = [0u32; 16];
        let mut vec = unsafe { FrameVec::from_raw_parts(buffer.as_mut_ptr(), 16).unwrap() };

        assert!(vec.is_empty());
        assert_eq!(vec.capacity(), 16);

        vec.push(1).unwrap();
        vec.push(2).unwrap();
        vec.push(3).unwrap();

        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 3);

        assert_eq!(vec.pop(), Some(3));
        assert_eq!(vec.len(), 2);
    }

    #[test]
    fn test_frame_vec_full() {
        let mut buffer = [0u32; 2];
        let mut vec = unsafe { FrameVec::from_raw_parts(buffer.as_mut_ptr(), 2).unwrap() };

        vec.push(1).unwrap();
        vec.push(2).unwrap();
        assert!(vec.push(3).is_err());
    }

    #[test]
    fn test_frame_vec_iter() {
        let mut buffer = [0u32; 16];
        let mut vec = unsafe { FrameVec::from_raw_parts(buffer.as_mut_ptr(), 16).unwrap() };

        vec.push(1).unwrap();
        vec.push(2).unwrap();
        vec.push(3).unwrap();

        let sum: u32 = vec.iter().sum();
        assert_eq!(sum, 6);
    }
}
