//! Allocation groups - named collections of allocations that can be freed together.

use std::alloc::{alloc, dealloc, Layout};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::sync::mutex::Mutex;

/// Unique identifier for an allocation group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupId(u64);

/// Metadata for a single allocation within a group.
struct GroupAllocation {
    ptr: *mut u8,
    layout: Layout,
}

/// A group of allocations that can be freed together.
struct Group {
    name: String,
    allocations: Vec<GroupAllocation>,
    total_bytes: usize,
}

/// Manages allocation groups.
pub struct GroupAllocator {
    groups: Mutex<HashMap<GroupId, Group>>,
    next_id: AtomicU64,
}

impl GroupAllocator {
    /// Create a new group allocator.
    pub fn new() -> Self {
        Self {
            groups: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Create a new allocation group.
    pub fn create_group(&self, name: impl Into<String>) -> GroupId {
        let id = GroupId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let group = Group {
            name: name.into(),
            allocations: Vec::new(),
            total_bytes: 0,
        };

        let mut groups = self.groups.lock();
        groups.insert(id, group);
        id
    }

    /// Allocate memory within a group.
    pub fn alloc<T>(&self, group_id: GroupId) -> Option<*mut T> {
        self.alloc_layout(group_id, Layout::new::<T>())
            .map(|ptr| ptr as *mut T)
    }

    /// Allocate memory with a specific layout within a group.
    pub fn alloc_layout(&self, group_id: GroupId, layout: Layout) -> Option<*mut u8> {
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return None;
        }

        let mut groups = self.groups.lock();
        if let Some(group) = groups.get_mut(&group_id) {
            group.allocations.push(GroupAllocation { ptr, layout });
            group.total_bytes += layout.size();
            Some(ptr)
        } else {
            // Group doesn't exist, free the memory
            unsafe { dealloc(ptr, layout) };
            None
        }
    }

    /// Allocate and initialize a value within a group.
    pub fn alloc_val<T>(&self, group_id: GroupId, value: T) -> Option<*mut T> {
        let ptr = self.alloc::<T>(group_id)?;
        unsafe {
            std::ptr::write(ptr, value);
        }
        Some(ptr)
    }

    /// Allocate a slice within a group.
    pub fn alloc_slice<T>(&self, group_id: GroupId, len: usize) -> Option<*mut T> {
        let layout = Layout::array::<T>(len).ok()?;
        self.alloc_layout(group_id, layout).map(|ptr| ptr as *mut T)
    }

    /// Free all allocations in a group.
    pub fn free_group(&self, group_id: GroupId) {
        let mut groups = self.groups.lock();
        if let Some(group) = groups.remove(&group_id) {
            for alloc in group.allocations {
                unsafe {
                    dealloc(alloc.ptr, alloc.layout);
                }
            }
        }
    }

    /// Get the total bytes allocated in a group.
    pub fn group_size(&self, group_id: GroupId) -> usize {
        let groups = self.groups.lock();
        groups.get(&group_id).map(|g| g.total_bytes).unwrap_or(0)
    }

    /// Get the number of allocations in a group.
    pub fn group_count(&self, group_id: GroupId) -> usize {
        let groups = self.groups.lock();
        groups.get(&group_id).map(|g| g.allocations.len()).unwrap_or(0)
    }

    /// Get the name of a group.
    pub fn group_name(&self, group_id: GroupId) -> Option<String> {
        let groups = self.groups.lock();
        groups.get(&group_id).map(|g| g.name.clone())
    }

    /// Get statistics about all groups.
    pub fn stats(&self) -> GroupStats {
        let groups = self.groups.lock();
        let mut stats = GroupStats::default();

        for group in groups.values() {
            stats.total_groups += 1;
            stats.total_allocations += group.allocations.len();
            stats.total_bytes += group.total_bytes;
        }

        stats
    }

    /// Check if a group exists.
    pub fn group_exists(&self, group_id: GroupId) -> bool {
        let groups = self.groups.lock();
        groups.contains_key(&group_id)
    }
}

impl Default for GroupAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: GroupAllocator uses internal synchronization
unsafe impl Send for GroupAllocator {}
unsafe impl Sync for GroupAllocator {}

/// Statistics about allocation groups.
#[derive(Debug, Clone, Default)]
pub struct GroupStats {
    /// Total number of active groups
    pub total_groups: usize,
    /// Total number of allocations across all groups
    pub total_allocations: usize,
    /// Total bytes allocated across all groups
    pub total_bytes: usize,
}

/// A handle to a specific group for convenient allocation.
pub struct GroupHandle<'a> {
    allocator: &'a GroupAllocator,
    id: GroupId,
}

impl<'a> GroupHandle<'a> {
    /// Create a new group handle.
    pub fn new(allocator: &'a GroupAllocator, id: GroupId) -> Self {
        Self { allocator, id }
    }

    /// Get the group ID.
    pub fn id(&self) -> GroupId {
        self.id
    }

    /// Allocate memory in this group.
    pub fn alloc<T>(&self) -> Option<*mut T> {
        self.allocator.alloc::<T>(self.id)
    }

    /// Allocate and initialize a value in this group.
    pub fn alloc_val<T>(&self, value: T) -> Option<*mut T> {
        self.allocator.alloc_val(self.id, value)
    }

    /// Allocate a slice in this group.
    pub fn alloc_slice<T>(&self, len: usize) -> Option<*mut T> {
        self.allocator.alloc_slice::<T>(self.id, len)
    }

    /// Get the total bytes allocated in this group.
    pub fn size(&self) -> usize {
        self.allocator.group_size(self.id)
    }

    /// Get the number of allocations in this group.
    pub fn count(&self) -> usize {
        self.allocator.group_count(self.id)
    }

    /// Free all allocations in this group.
    pub fn free_all(self) {
        self.allocator.free_group(self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_allocation() {
        let allocator = GroupAllocator::new();
        let group = allocator.create_group("test");

        let ptr1 = allocator.alloc::<u64>(group).unwrap();
        let ptr2 = allocator.alloc::<u64>(group).unwrap();

        unsafe {
            *ptr1 = 42;
            *ptr2 = 123;
        }

        assert_eq!(allocator.group_count(group), 2);
        assert!(allocator.group_size(group) >= 16);

        allocator.free_group(group);
        assert!(!allocator.group_exists(group));
    }

    #[test]
    fn test_group_handle() {
        let allocator = GroupAllocator::new();
        let id = allocator.create_group("level_1");
        let handle = GroupHandle::new(&allocator, id);

        let ptr = handle.alloc_val(42u64).unwrap();
        assert_eq!(unsafe { *ptr }, 42);
        assert_eq!(handle.count(), 1);

        handle.free_all();
    }

    #[test]
    fn test_multiple_groups() {
        let allocator = GroupAllocator::new();
        let group1 = allocator.create_group("group1");
        let group2 = allocator.create_group("group2");

        allocator.alloc::<u64>(group1);
        allocator.alloc::<u64>(group1);
        allocator.alloc::<u64>(group2);

        assert_eq!(allocator.group_count(group1), 2);
        assert_eq!(allocator.group_count(group2), 1);

        allocator.free_group(group1);
        assert_eq!(allocator.group_count(group2), 1);

        allocator.free_group(group2);
    }
}
