//! Memory budget management with per-tag tracking.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::api::tag::AllocationTag;
use crate::sync::mutex::Mutex;

/// Manages memory budgets and limits with per-tag tracking.
pub struct BudgetManager {
    /// Global memory limit (0 = unlimited)
    global_limit: usize,

    /// Current total usage (atomic for fast reads)
    current_usage: AtomicUsize,

    /// Per-tag budgets and usage
    tag_data: Mutex<HashMap<&'static str, TagBudget>>,

    /// Callback for budget events
    event_callback: Mutex<Option<Box<dyn Fn(BudgetEvent) + Send + Sync>>>,
}

/// Budget configuration and current usage for a specific tag.
#[derive(Debug, Clone)]
pub struct TagBudget {
    /// Tag name
    pub name: &'static str,

    /// Soft limit (warning threshold)
    pub soft_limit: usize,

    /// Hard limit (allocation may fail)
    pub hard_limit: usize,

    /// Current usage in bytes
    pub current_usage: usize,

    /// Peak usage (high water mark)
    pub peak_usage: usize,

    /// Number of allocations
    pub allocation_count: u64,

    /// Number of deallocations
    pub deallocation_count: u64,
}

impl TagBudget {
    /// Create a new tag budget.
    pub fn new(name: &'static str, soft_limit: usize, hard_limit: usize) -> Self {
        Self {
            name,
            soft_limit,
            hard_limit,
            current_usage: 0,
            peak_usage: 0,
            allocation_count: 0,
            deallocation_count: 0,
        }
    }

    /// Check the budget status for a potential allocation.
    pub fn check_status(&self, additional_size: usize) -> BudgetStatus {
        let projected = self.current_usage + additional_size;
        
        if self.hard_limit > 0 && projected > self.hard_limit {
            BudgetStatus::Exceeded
        } else if self.soft_limit > 0 && projected > self.soft_limit {
            BudgetStatus::Warning
        } else {
            BudgetStatus::Ok
        }
    }

    /// Calculate usage as a percentage of hard limit.
    pub fn usage_percent(&self) -> f64 {
        if self.hard_limit == 0 {
            0.0
        } else {
            (self.current_usage as f64 / self.hard_limit as f64) * 100.0
        }
    }
}

/// Result of a budget check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetStatus {
    /// Under budget, allocation allowed
    Ok,

    /// Over soft limit, warning issued
    Warning,

    /// Over hard limit, allocation denied
    Exceeded,
}

/// Events emitted by the budget manager.
#[derive(Debug, Clone)]
pub enum BudgetEvent {
    /// Soft limit exceeded
    SoftLimitExceeded {
        tag: &'static str,
        current: usize,
        limit: usize,
    },
    /// Hard limit exceeded
    HardLimitExceeded {
        tag: &'static str,
        current: usize,
        limit: usize,
    },
    /// Global limit exceeded
    GlobalLimitExceeded {
        current: usize,
        limit: usize,
    },
    /// New peak usage recorded
    NewPeak {
        tag: &'static str,
        peak: usize,
    },
}

impl BudgetManager {
    /// Create a new budget manager.
    pub fn new(global_limit: usize) -> Self {
        Self {
            global_limit,
            current_usage: AtomicUsize::new(0),
            tag_data: Mutex::new(HashMap::new()),
            event_callback: Mutex::new(None),
        }
    }

    /// Set a callback for budget events.
    pub fn set_event_callback<F>(&self, callback: F)
    where
        F: Fn(BudgetEvent) + Send + Sync + 'static,
    {
        let mut cb = self.event_callback.lock();
        *cb = Some(Box::new(callback));
    }

    /// Register a budget for a specific tag.
    pub fn register_tag(&self, tag: &AllocationTag, soft_limit: usize, hard_limit: usize) {
        let mut data = self.tag_data.lock();
        data.insert(tag.name(), TagBudget::new(tag.name(), soft_limit, hard_limit));
    }

    /// Register a budget by tag name.
    pub fn register_tag_budget(&self, name: &'static str, soft_limit: usize, hard_limit: usize) {
        let mut data = self.tag_data.lock();
        data.insert(name, TagBudget::new(name, soft_limit, hard_limit));
    }

    /// Check if an allocation is within budget (global check).
    pub fn check_allocation(&self, size: usize, new_total: usize) -> BudgetStatus {
        if self.global_limit > 0 && new_total > self.global_limit {
            self.emit_event(BudgetEvent::GlobalLimitExceeded {
                current: new_total,
                limit: self.global_limit,
            });
            return BudgetStatus::Exceeded;
        }

        self.current_usage.store(new_total, Ordering::Relaxed);

        // Check soft limit (90% of hard limit)
        if self.global_limit > 0 {
            let soft_limit = self.global_limit * 9 / 10;
            if new_total > soft_limit {
                return BudgetStatus::Warning;
            }
        }

        let _ = size; // Used in tagged allocations
        BudgetStatus::Ok
    }

    /// Check and record a tagged allocation.
    pub fn check_tagged_allocation(&self, tag: &AllocationTag, size: usize) -> BudgetStatus {
        let mut data = self.tag_data.lock();
        
        // Get or create tag budget
        let budget = data.entry(tag.name()).or_insert_with(|| {
            TagBudget::new(tag.name(), 0, 0) // No limits by default
        });

        let status = budget.check_status(size);

        // Record the allocation
        budget.current_usage += size;
        budget.allocation_count += 1;

        // Update peak
        if budget.current_usage > budget.peak_usage {
            budget.peak_usage = budget.current_usage;
            self.emit_event(BudgetEvent::NewPeak {
                tag: tag.name(),
                peak: budget.peak_usage,
            });
        }

        // Emit events based on status
        match status {
            BudgetStatus::Warning => {
                self.emit_event(BudgetEvent::SoftLimitExceeded {
                    tag: tag.name(),
                    current: budget.current_usage,
                    limit: budget.soft_limit,
                });
            }
            BudgetStatus::Exceeded => {
                self.emit_event(BudgetEvent::HardLimitExceeded {
                    tag: tag.name(),
                    current: budget.current_usage,
                    limit: budget.hard_limit,
                });
            }
            BudgetStatus::Ok => {}
        }

        status
    }

    /// Record a tagged deallocation.
    pub fn record_tagged_deallocation(&self, tag: &AllocationTag, size: usize) {
        let mut data = self.tag_data.lock();
        
        if let Some(budget) = data.get_mut(tag.name()) {
            budget.current_usage = budget.current_usage.saturating_sub(size);
            budget.deallocation_count += 1;
        }
    }

    /// Get current global usage.
    pub fn current_usage(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed)
    }

    /// Get global limit.
    pub fn global_limit(&self) -> usize {
        self.global_limit
    }

    /// Get all tag budgets for reporting.
    pub fn get_all_tag_budgets(&self) -> Vec<TagBudget> {
        let data = self.tag_data.lock();
        data.values().cloned().collect()
    }

    /// Get a specific tag's budget.
    pub fn get_tag_budget(&self, tag: &AllocationTag) -> Option<TagBudget> {
        let data = self.tag_data.lock();
        data.get(tag.name()).cloned()
    }

    /// Reset all tag statistics (but keep limits).
    pub fn reset_stats(&self) {
        let mut data = self.tag_data.lock();
        for budget in data.values_mut() {
            budget.current_usage = 0;
            budget.peak_usage = 0;
            budget.allocation_count = 0;
            budget.deallocation_count = 0;
        }
        self.current_usage.store(0, Ordering::Relaxed);
    }

    /// Emit a budget event to the callback.
    fn emit_event(&self, event: BudgetEvent) {
        if let Some(ref callback) = *self.event_callback.lock() {
            callback(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_budget_tracking() {
        let manager = BudgetManager::new(0);
        let tag = AllocationTag::new("test");
        
        manager.register_tag(&tag, 1000, 2000);
        
        // Allocate under soft limit
        let status = manager.check_tagged_allocation(&tag, 500);
        assert_eq!(status, BudgetStatus::Ok);
        
        // Allocate over soft limit
        let status = manager.check_tagged_allocation(&tag, 600);
        assert_eq!(status, BudgetStatus::Warning);
        
        // Check usage
        let budget = manager.get_tag_budget(&tag).unwrap();
        assert_eq!(budget.current_usage, 1100);
        assert_eq!(budget.allocation_count, 2);
    }

    #[test]
    fn test_hard_limit() {
        let manager = BudgetManager::new(0);
        let tag = AllocationTag::new("limited");
        
        manager.register_tag(&tag, 500, 1000);
        
        manager.check_tagged_allocation(&tag, 800);
        let status = manager.check_tagged_allocation(&tag, 300);
        
        assert_eq!(status, BudgetStatus::Exceeded);
    }

    #[test]
    fn test_deallocation() {
        let manager = BudgetManager::new(0);
        let tag = AllocationTag::new("dealloc_test");
        
        manager.check_tagged_allocation(&tag, 1000);
        manager.record_tagged_deallocation(&tag, 400);
        
        let budget = manager.get_tag_budget(&tag).unwrap();
        assert_eq!(budget.current_usage, 600);
        assert_eq!(budget.deallocation_count, 1);
    }
}
