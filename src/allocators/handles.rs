//! Handle-based allocation with relocation support.
//!
//! Provides stable handles that remain valid even when the underlying
//! memory is relocated for defragmentation.

use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::sync::mutex::Mutex;

/// Generation counter for handle validation.
type Generation = u32;

/// A stable handle to allocated memory.
///
/// Handles remain valid across relocations. The actual pointer
/// is resolved at access time.
#[derive(Debug)]
pub struct Handle<T> {
    index: u32,
    generation: Generation,
    _marker: PhantomData<T>,
}

// Manual implementations to avoid T: Copy/Clone bounds
impl<T> Copy for Handle<T> {}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> Handle<T> {
    /// Create a dangling handle (for default initialization).
    pub const fn dangling() -> Self {
        Self {
            index: u32::MAX,
            generation: 0,
            _marker: PhantomData,
        }
    }

    /// Check if this is a dangling/invalid handle.
    pub fn is_dangling(&self) -> bool {
        self.index == u32::MAX
    }

    /// Get the raw index (for debugging).
    pub fn raw_index(&self) -> u32 {
        self.index
    }

    /// Get the generation (for debugging).
    pub fn raw_generation(&self) -> u32 {
        self.generation
    }
}

impl<T> Default for Handle<T> {
    fn default() -> Self {
        Self::dangling()
    }
}


/// Internal slot for tracking allocations.
struct Slot {
    /// Pointer to allocated memory
    ptr: *mut u8,
    /// Size of allocation
    size: usize,
    /// Current generation
    generation: Generation,
    /// Whether this slot is in use
    in_use: bool,
    /// Whether the memory can be relocated
    relocatable: bool,
    /// Relocation callback (called after move)
    on_relocate: Option<Box<dyn Fn(*mut u8, *mut u8) + Send + Sync>>,
}

impl Slot {
    #[allow(dead_code)]
    fn empty() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            size: 0,
            generation: 0,
            in_use: false,
            relocatable: true,
            on_relocate: None,
        }
    }
}

/// Handle-based allocator with relocation support.
pub struct HandleAllocator {
    /// Allocation slots
    slots: Mutex<Vec<Slot>>,
    
    /// Free slot indices
    free_list: Mutex<Vec<u32>>,
    
    /// Total allocated bytes
    total_allocated: AtomicU64,
    
    /// Number of active handles
    active_count: AtomicU32,
    
    /// Number of relocations performed
    relocation_count: AtomicU64,
}

impl HandleAllocator {
    /// Create a new handle allocator.
    pub fn new() -> Self {
        Self {
            slots: Mutex::new(Vec::with_capacity(1024)),
            free_list: Mutex::new(Vec::new()),
            total_allocated: AtomicU64::new(0),
            active_count: AtomicU32::new(0),
            relocation_count: AtomicU64::new(0),
        }
    }

    /// Allocate memory and return a handle.
    pub fn alloc<T>(&self) -> Option<Handle<T>> {
        self.alloc_with_options::<T>(true, None)
    }

    /// Allocate with options.
    pub fn alloc_with_options<T>(
        &self,
        relocatable: bool,
        on_relocate: Option<Box<dyn Fn(*mut u8, *mut u8) + Send + Sync>>,
    ) -> Option<Handle<T>> {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();
        
        let layout = Layout::from_size_align(size, align).ok()?;
        let ptr = unsafe { alloc(layout) };
        
        if ptr.is_null() {
            return None;
        }

        let (index, generation) = self.allocate_slot(ptr, size, relocatable, on_relocate);
        
        self.total_allocated.fetch_add(size as u64, Ordering::Relaxed);
        self.active_count.fetch_add(1, Ordering::Relaxed);

        Some(Handle {
            index,
            generation,
            _marker: PhantomData,
        })
    }

    /// Allocate a slot for the given pointer.
    fn allocate_slot(
        &self,
        ptr: *mut u8,
        size: usize,
        relocatable: bool,
        on_relocate: Option<Box<dyn Fn(*mut u8, *mut u8) + Send + Sync>>,
    ) -> (u32, Generation) {
        let mut free_list = self.free_list.lock();
        let mut slots = self.slots.lock();

        if let Some(index) = free_list.pop() {
            let slot = &mut slots[index as usize];
            slot.ptr = ptr;
            slot.size = size;
            slot.generation = slot.generation.wrapping_add(1);
            slot.in_use = true;
            slot.relocatable = relocatable;
            slot.on_relocate = on_relocate;
            (index, slot.generation)
        } else {
            let index = slots.len() as u32;
            slots.push(Slot {
                ptr,
                size,
                generation: 1,
                in_use: true,
                relocatable,
                on_relocate,
            });
            (index, 1)
        }
    }

    /// Free a handle.
    pub fn free<T>(&self, handle: Handle<T>) {
        if handle.is_dangling() {
            return;
        }

        let mut slots = self.slots.lock();
        let mut free_list = self.free_list.lock();

        if let Some(slot) = slots.get_mut(handle.index as usize) {
            if slot.in_use && slot.generation == handle.generation {
                let layout = Layout::from_size_align(slot.size, 1).expect("Invalid layout");
                unsafe {
                    dealloc(slot.ptr, layout);
                }
                
                self.total_allocated.fetch_sub(slot.size as u64, Ordering::Relaxed);
                self.active_count.fetch_sub(1, Ordering::Relaxed);
                
                slot.ptr = std::ptr::null_mut();
                slot.in_use = false;
                slot.on_relocate = None;
                free_list.push(handle.index);
            }
        }
    }

    /// Resolve a handle to a pointer.
    ///
    /// Returns None if the handle is invalid or has been freed.
    pub fn resolve<T>(&self, handle: Handle<T>) -> Option<*const T> {
        if handle.is_dangling() {
            return None;
        }

        let slots = self.slots.lock();
        slots.get(handle.index as usize).and_then(|slot| {
            if slot.in_use && slot.generation == handle.generation {
                Some(slot.ptr as *const T)
            } else {
                None
            }
        })
    }

    /// Resolve a handle to a mutable pointer.
    pub fn resolve_mut<T>(&self, handle: Handle<T>) -> Option<*mut T> {
        if handle.is_dangling() {
            return None;
        }

        let slots = self.slots.lock();
        slots.get(handle.index as usize).and_then(|slot| {
            if slot.in_use && slot.generation == handle.generation {
                Some(slot.ptr as *mut T)
            } else {
                None
            }
        })
    }

    /// Check if a handle is valid.
    pub fn is_valid<T>(&self, handle: Handle<T>) -> bool {
        if handle.is_dangling() {
            return false;
        }

        let slots = self.slots.lock();
        slots.get(handle.index as usize).map_or(false, |slot| {
            slot.in_use && slot.generation == handle.generation
        })
    }

    /// Pin a handle to prevent relocation.
    pub fn pin<T>(&self, handle: Handle<T>) {
        if handle.is_dangling() {
            return;
        }

        let mut slots = self.slots.lock();
        if let Some(slot) = slots.get_mut(handle.index as usize) {
            if slot.in_use && slot.generation == handle.generation {
                slot.relocatable = false;
            }
        }
    }

    /// Unpin a handle to allow relocation.
    pub fn unpin<T>(&self, handle: Handle<T>) {
        if handle.is_dangling() {
            return;
        }

        let mut slots = self.slots.lock();
        if let Some(slot) = slots.get_mut(handle.index as usize) {
            if slot.in_use && slot.generation == handle.generation {
                slot.relocatable = true;
            }
        }
    }

    /// Perform defragmentation by relocating memory.
    ///
    /// This is a simple compaction that moves relocatable allocations
    /// to reduce fragmentation. Returns the number of relocations performed.
    pub fn defragment(&self) -> usize {
        let mut slots = self.slots.lock();
        let mut relocations = 0;

        // Simple strategy: try to compact smaller allocations together
        // This is a basic implementation - production would be more sophisticated
        
        let relocatable: Vec<usize> = slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.in_use && s.relocatable)
            .map(|(i, _)| i)
            .collect();

        for idx in relocatable {
            let slot = &mut slots[idx];
            
            // Try to allocate new memory
            let layout = match Layout::from_size_align(slot.size, 16) {
                Ok(l) => l,
                Err(_) => continue,
            };
            
            let new_ptr = unsafe { alloc(layout) };
            if new_ptr.is_null() {
                continue;
            }

            // Copy data
            unsafe {
                std::ptr::copy_nonoverlapping(slot.ptr, new_ptr, slot.size);
            }

            let old_ptr = slot.ptr;
            
            // Call relocation callback if set
            if let Some(ref callback) = slot.on_relocate {
                callback(old_ptr, new_ptr);
            }

            // Free old memory
            unsafe {
                dealloc(old_ptr, layout);
            }

            slot.ptr = new_ptr;
            relocations += 1;
        }

        self.relocation_count.fetch_add(relocations as u64, Ordering::Relaxed);
        relocations
    }

    /// Get total allocated bytes.
    pub fn total_allocated(&self) -> u64 {
        self.total_allocated.load(Ordering::Relaxed)
    }

    /// Get number of active handles.
    pub fn active_count(&self) -> u32 {
        self.active_count.load(Ordering::Relaxed)
    }

    /// Get total relocations performed.
    pub fn relocation_count(&self) -> u64 {
        self.relocation_count.load(Ordering::Relaxed)
    }

    /// Get statistics about the handle allocator.
    pub fn stats(&self) -> HandleAllocatorStats {
        let slots = self.slots.lock();
        let free_list = self.free_list.lock();
        
        let relocatable_count = slots.iter().filter(|s| s.in_use && s.relocatable).count();
        let pinned_count = slots.iter().filter(|s| s.in_use && !s.relocatable).count();

        HandleAllocatorStats {
            total_allocated: self.total_allocated.load(Ordering::Relaxed),
            active_handles: self.active_count.load(Ordering::Relaxed),
            total_slots: slots.len(),
            free_slots: free_list.len(),
            relocatable_count,
            pinned_count,
            relocation_count: self.relocation_count.load(Ordering::Relaxed),
        }
    }
}

impl Default for HandleAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: HandleAllocator uses internal synchronization
unsafe impl Send for HandleAllocator {}
unsafe impl Sync for HandleAllocator {}

/// Statistics for the handle allocator.
#[derive(Debug, Clone, Default)]
pub struct HandleAllocatorStats {
    /// Total bytes allocated
    pub total_allocated: u64,
    /// Number of active handles
    pub active_handles: u32,
    /// Total slot capacity
    pub total_slots: usize,
    /// Number of free slots
    pub free_slots: usize,
    /// Number of relocatable allocations
    pub relocatable_count: usize,
    /// Number of pinned allocations
    pub pinned_count: usize,
    /// Total relocations performed
    pub relocation_count: u64,
}

/// RAII guard for pinning a handle.
pub struct PinGuard<'a, T> {
    allocator: &'a HandleAllocator,
    handle: Handle<T>,
}

impl<'a, T> PinGuard<'a, T> {
    /// Create a new pin guard.
    pub fn new(allocator: &'a HandleAllocator, handle: Handle<T>) -> Self {
        allocator.pin(handle);
        Self { allocator, handle }
    }

    /// Get the handle.
    pub fn handle(&self) -> Handle<T> {
        self.handle
    }

    /// Resolve to a pointer (valid for lifetime of guard).
    pub fn get(&self) -> Option<*const T> {
        self.allocator.resolve(self.handle)
    }

    /// Resolve to a mutable pointer.
    pub fn get_mut(&self) -> Option<*mut T> {
        self.allocator.resolve_mut(self.handle)
    }
}

impl<'a, T> Drop for PinGuard<'a, T> {
    fn drop(&mut self) {
        self.allocator.unpin(self.handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_alloc_free() {
        let allocator = HandleAllocator::new();
        
        let handle: Handle<u64> = allocator.alloc().unwrap();
        assert!(allocator.is_valid(handle));
        
        let ptr = allocator.resolve_mut(handle).unwrap();
        unsafe { *ptr = 42; }
        
        let value = unsafe { *allocator.resolve(handle).unwrap() };
        assert_eq!(value, 42);
        
        allocator.free(handle);
        assert!(!allocator.is_valid(handle));
    }

    #[test]
    fn test_generation_invalidation() {
        let allocator = HandleAllocator::new();
        
        let handle1: Handle<u64> = allocator.alloc().unwrap();
        allocator.free(handle1);
        
        let handle2: Handle<u64> = allocator.alloc().unwrap();
        
        // Same index, different generation
        assert_eq!(handle1.index, handle2.index);
        assert_ne!(handle1.generation, handle2.generation);
        
        // Old handle should be invalid
        assert!(!allocator.is_valid(handle1));
        assert!(allocator.is_valid(handle2));
    }

    #[test]
    fn test_pin_unpin() {
        let allocator = HandleAllocator::new();
        
        let handle: Handle<u64> = allocator.alloc().unwrap();
        
        allocator.pin(handle);
        
        let stats = allocator.stats();
        assert_eq!(stats.pinned_count, 1);
        assert_eq!(stats.relocatable_count, 0);
        
        allocator.unpin(handle);
        
        let stats = allocator.stats();
        assert_eq!(stats.pinned_count, 0);
        assert_eq!(stats.relocatable_count, 1);
    }

    #[test]
    fn test_dangling_handle() {
        let allocator = HandleAllocator::new();
        
        let handle: Handle<u64> = Handle::dangling();
        
        assert!(handle.is_dangling());
        assert!(!allocator.is_valid(handle));
        assert!(allocator.resolve(handle).is_none());
    }
}
