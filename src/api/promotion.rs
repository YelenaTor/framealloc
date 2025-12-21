//! Promotion logic - handles moving allocations from frame to other allocators.
//!
//! This module processes retained allocations at frame end and moves them
//! to their designated destination allocators.

use std::alloc::Layout;

use crate::api::retention::{
    PromotedAllocation, PromotionFailure, RetainedAllocation, RetainedMeta, RetentionPolicy,
};

/// Summary of frame end operations.
#[derive(Debug, Clone, Default)]
pub struct FrameSummary {
    /// Total bytes discarded (normal frame behavior)
    pub discarded_bytes: usize,
    /// Number of allocations discarded
    pub discarded_count: usize,
    
    /// Bytes promoted to pool
    pub promoted_pool_bytes: usize,
    /// Count promoted to pool
    pub promoted_pool_count: usize,
    
    /// Bytes promoted to heap
    pub promoted_heap_bytes: usize,
    /// Count promoted to heap
    pub promoted_heap_count: usize,
    
    /// Bytes promoted to scratch pools
    pub promoted_scratch_bytes: usize,
    /// Count promoted to scratch pools
    pub promoted_scratch_count: usize,
    
    /// Bytes that failed to promote
    pub failed_bytes: usize,
    /// Count of failed promotions
    pub failed_count: usize,
    
    /// Breakdown by failure reason
    pub failures_by_reason: FailureBreakdown,
    
    /// Per-tag breakdown (if tags were used)
    pub by_tag: Vec<TagSummary>,
    
    /// Per-phase breakdown (if phases were used)
    pub by_phase: Vec<PhaseSummary>,
}

impl FrameSummary {
    /// Total bytes that were retained (not discarded)
    pub fn total_retained_bytes(&self) -> usize {
        self.promoted_pool_bytes + self.promoted_heap_bytes + self.promoted_scratch_bytes
    }
    
    /// Total count of retained allocations
    pub fn total_retained_count(&self) -> usize {
        self.promoted_pool_count + self.promoted_heap_count + self.promoted_scratch_count
    }
    
    /// Success rate of promotions (0.0 - 1.0)
    pub fn promotion_success_rate(&self) -> f32 {
        let total = self.total_retained_count() + self.failed_count;
        if total == 0 {
            1.0
        } else {
            self.total_retained_count() as f32 / total as f32
        }
    }
}

/// Breakdown of promotion failures by reason.
#[derive(Debug, Clone, Default)]
pub struct FailureBreakdown {
    pub budget_exceeded: usize,
    pub scratch_pool_not_found: usize,
    pub scratch_pool_full: usize,
    pub too_large: usize,
    pub internal_error: usize,
}

impl FailureBreakdown {
    pub fn record(&mut self, reason: PromotionFailure) {
        match reason {
            PromotionFailure::BudgetExceeded => self.budget_exceeded += 1,
            PromotionFailure::ScratchPoolNotFound => self.scratch_pool_not_found += 1,
            PromotionFailure::ScratchPoolFull => self.scratch_pool_full += 1,
            PromotionFailure::TooLarge => self.too_large += 1,
            PromotionFailure::InternalError => self.internal_error += 1,
        }
    }
}

/// Per-tag summary.
#[derive(Debug, Clone)]
pub struct TagSummary {
    pub tag: &'static str,
    pub discarded_bytes: usize,
    pub promoted_bytes: usize,
    pub failed_bytes: usize,
}

/// Per-phase summary.
#[derive(Debug, Clone)]
pub struct PhaseSummary {
    pub phase: &'static str,
    pub discarded_bytes: usize,
    pub promoted_bytes: usize,
    pub failed_bytes: usize,
}

/// Result of processing retained allocations.
pub struct PromotionResult {
    /// Successfully promoted allocations
    pub promoted: Vec<PromotedAllocation>,
    /// Summary statistics
    pub summary: FrameSummary,
}

/// Processor for retained allocations.
pub struct PromotionProcessor<'a> {
    /// Pool allocator callback
    pool_alloc: Option<Box<dyn FnMut(Layout) -> *mut u8 + 'a>>,
    /// Heap allocator callback  
    heap_alloc: Option<Box<dyn FnMut(Layout) -> *mut u8 + 'a>>,
    /// Scratch pool allocator callback
    scratch_alloc: Option<Box<dyn FnMut(&'static str, Layout) -> Option<*mut u8> + 'a>>,
}

impl<'a> PromotionProcessor<'a> {
    pub fn new() -> Self {
        Self {
            pool_alloc: None,
            heap_alloc: None,
            scratch_alloc: None,
        }
    }
    
    pub fn with_pool_alloc<F>(mut self, f: F) -> Self
    where
        F: FnMut(Layout) -> *mut u8 + 'a,
    {
        self.pool_alloc = Some(Box::new(f));
        self
    }
    
    pub fn with_heap_alloc<F>(mut self, f: F) -> Self
    where
        F: FnMut(Layout) -> *mut u8 + 'a,
    {
        self.heap_alloc = Some(Box::new(f));
        self
    }
    
    pub fn with_scratch_alloc<F>(mut self, f: F) -> Self
    where
        F: FnMut(&'static str, Layout) -> Option<*mut u8> + 'a,
    {
        self.scratch_alloc = Some(Box::new(f));
        self
    }
    
    /// Process all retained allocations.
    pub fn process(mut self, retained: Vec<RetainedAllocation>) -> PromotionResult {
        let mut promoted = Vec::with_capacity(retained.len());
        let mut summary = FrameSummary::default();
        
        for alloc in retained {
            let result = self.promote_one(&alloc.meta);
            
            match &result {
                PromotedAllocation::Pool { size, .. } => {
                    summary.promoted_pool_bytes += size;
                    summary.promoted_pool_count += 1;
                    
                    // Copy data to new location
                    if let PromotedAllocation::Pool { ptr, size, .. } = &result {
                        if !ptr.is_null() && !alloc.ptr.is_null() {
                            unsafe {
                                std::ptr::copy_nonoverlapping(alloc.ptr, *ptr, *size);
                            }
                        }
                    }
                }
                PromotedAllocation::Heap { size, .. } => {
                    summary.promoted_heap_bytes += size;
                    summary.promoted_heap_count += 1;
                    
                    // Copy data to new location
                    if let PromotedAllocation::Heap { ptr, size, .. } = &result {
                        if !ptr.is_null() && !alloc.ptr.is_null() {
                            unsafe {
                                std::ptr::copy_nonoverlapping(alloc.ptr, *ptr, *size);
                            }
                        }
                    }
                }
                PromotedAllocation::Scratch { size, .. } => {
                    summary.promoted_scratch_bytes += size;
                    summary.promoted_scratch_count += 1;
                    
                    // Copy data to new location
                    if let PromotedAllocation::Scratch { ptr, size, .. } = &result {
                        if !ptr.is_null() && !alloc.ptr.is_null() {
                            unsafe {
                                std::ptr::copy_nonoverlapping(alloc.ptr, *ptr, *size);
                            }
                        }
                    }
                }
                PromotedAllocation::Failed { reason, meta } => {
                    summary.failed_bytes += meta.size;
                    summary.failed_count += 1;
                    summary.failures_by_reason.record(*reason);
                }
            }
            
            promoted.push(result);
        }
        
        PromotionResult { promoted, summary }
    }
    
    fn promote_one(&mut self, meta: &RetainedMeta) -> PromotedAllocation {
        let layout = Layout::from_size_align(meta.size, 8).unwrap_or(Layout::new::<u8>());
        
        match meta.policy {
            RetentionPolicy::Discard => {
                // This shouldn't happen - discarded allocations aren't registered
                PromotedAllocation::Failed {
                    reason: PromotionFailure::InternalError,
                    meta: meta.clone(),
                }
            }
            
            RetentionPolicy::PromoteToPool => {
                if let Some(ref mut alloc_fn) = self.pool_alloc {
                    let ptr = alloc_fn(layout);
                    if ptr.is_null() {
                        PromotedAllocation::Failed {
                            reason: PromotionFailure::BudgetExceeded,
                            meta: meta.clone(),
                        }
                    } else {
                        PromotedAllocation::Pool {
                            ptr,
                            size: meta.size,
                            tag: meta.tag,
                            type_name: meta.type_name,
                        }
                    }
                } else {
                    PromotedAllocation::Failed {
                        reason: PromotionFailure::InternalError,
                        meta: meta.clone(),
                    }
                }
            }
            
            RetentionPolicy::PromoteToHeap => {
                if let Some(ref mut alloc_fn) = self.heap_alloc {
                    let ptr = alloc_fn(layout);
                    if ptr.is_null() {
                        PromotedAllocation::Failed {
                            reason: PromotionFailure::BudgetExceeded,
                            meta: meta.clone(),
                        }
                    } else {
                        PromotedAllocation::Heap {
                            ptr,
                            size: meta.size,
                            tag: meta.tag,
                            type_name: meta.type_name,
                        }
                    }
                } else {
                    PromotedAllocation::Failed {
                        reason: PromotionFailure::InternalError,
                        meta: meta.clone(),
                    }
                }
            }
            
            RetentionPolicy::PromoteToScratch(pool_name) => {
                if let Some(ref mut alloc_fn) = self.scratch_alloc {
                    match alloc_fn(pool_name, layout) {
                        Some(ptr) if !ptr.is_null() => {
                            PromotedAllocation::Scratch {
                                pool_name,
                                ptr,
                                size: meta.size,
                                tag: meta.tag,
                                type_name: meta.type_name,
                            }
                        }
                        Some(_) => {
                            PromotedAllocation::Failed {
                                reason: PromotionFailure::ScratchPoolFull,
                                meta: meta.clone(),
                            }
                        }
                        None => {
                            PromotedAllocation::Failed {
                                reason: PromotionFailure::ScratchPoolNotFound,
                                meta: meta.clone(),
                            }
                        }
                    }
                } else {
                    PromotedAllocation::Failed {
                        reason: PromotionFailure::InternalError,
                        meta: meta.clone(),
                    }
                }
            }
        }
    }
}

impl<'a> Default for PromotionProcessor<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_frame_summary_default() {
        let summary = FrameSummary::default();
        assert_eq!(summary.total_retained_bytes(), 0);
        assert_eq!(summary.total_retained_count(), 0);
        assert_eq!(summary.promotion_success_rate(), 1.0);
    }
    
    #[test]
    fn test_failure_breakdown() {
        let mut breakdown = FailureBreakdown::default();
        breakdown.record(PromotionFailure::BudgetExceeded);
        breakdown.record(PromotionFailure::BudgetExceeded);
        breakdown.record(PromotionFailure::ScratchPoolFull);
        
        assert_eq!(breakdown.budget_exceeded, 2);
        assert_eq!(breakdown.scratch_pool_full, 1);
    }
}
