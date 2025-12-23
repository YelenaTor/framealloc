//! Per-thread frame budgets for v0.6.0.
//!
//! Provides explicit per-thread memory limits with deterministic
//! behavior when budgets are exceeded.

use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::thread::ThreadId;
use std::collections::HashMap;
use std::sync::Mutex;

/// Policy for handling budget exceeded situations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetExceededPolicy {
    /// Fail the allocation (return null/error).
    Fail,
    /// Log a warning but allow the allocation.
    Warn,
    /// Silently allow the allocation.
    Allow,
    /// Attempt to promote to a larger allocator (pool → heap).
    Promote,
    /// Call a custom handler.
    Custom,
}

impl Default for BudgetExceededPolicy {
    fn default() -> Self {
        Self::Warn
    }
}

/// Per-thread budget configuration.
#[derive(Debug, Clone)]
pub struct ThreadBudgetConfig {
    /// Maximum bytes for frame allocations.
    pub frame_budget: usize,
    /// Maximum bytes for pool allocations.
    pub pool_budget: usize,
    /// Policy when frame budget is exceeded.
    pub frame_exceeded_policy: BudgetExceededPolicy,
    /// Policy when pool budget is exceeded.
    pub pool_exceeded_policy: BudgetExceededPolicy,
    /// Warning threshold (percentage of budget, 0-100).
    pub warning_threshold: u8,
}

impl Default for ThreadBudgetConfig {
    fn default() -> Self {
        Self {
            frame_budget: 16 * 1024 * 1024,  // 16 MB default
            pool_budget: 8 * 1024 * 1024,    // 8 MB default
            frame_exceeded_policy: BudgetExceededPolicy::Warn,
            pool_exceeded_policy: BudgetExceededPolicy::Warn,
            warning_threshold: 80,
        }
    }
}

impl ThreadBudgetConfig {
    /// Create a strict configuration that fails on budget exceeded.
    pub fn strict(frame_mb: usize, pool_mb: usize) -> Self {
        Self {
            frame_budget: frame_mb * 1024 * 1024,
            pool_budget: pool_mb * 1024 * 1024,
            frame_exceeded_policy: BudgetExceededPolicy::Fail,
            pool_exceeded_policy: BudgetExceededPolicy::Fail,
            warning_threshold: 90,
        }
    }

    /// Create a relaxed configuration that allows exceeding budgets.
    pub fn relaxed(frame_mb: usize, pool_mb: usize) -> Self {
        Self {
            frame_budget: frame_mb * 1024 * 1024,
            pool_budget: pool_mb * 1024 * 1024,
            frame_exceeded_policy: BudgetExceededPolicy::Allow,
            pool_exceeded_policy: BudgetExceededPolicy::Allow,
            warning_threshold: 95,
        }
    }
}

/// Current state of a thread's budget.
#[derive(Debug, Default)]
pub struct ThreadBudgetState {
    /// Current frame allocation usage.
    pub frame_used: AtomicUsize,
    /// Peak frame allocation usage.
    pub frame_peak: AtomicUsize,
    /// Current pool allocation usage.
    pub pool_used: AtomicUsize,
    /// Peak pool allocation usage.
    pub pool_peak: AtomicUsize,
    /// Whether warning threshold has been hit this frame.
    pub warning_issued: AtomicBool,
    /// Number of times budget was exceeded.
    pub exceeded_count: AtomicUsize,
}

impl ThreadBudgetState {
    /// Create new budget state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a frame allocation.
    pub fn record_frame_alloc(&self, size: usize) -> usize {
        let new_used = self.frame_used.fetch_add(size, Ordering::Relaxed) + size;
        self.frame_peak.fetch_max(new_used, Ordering::Relaxed);
        new_used
    }

    /// Record a frame deallocation.
    pub fn record_frame_free(&self, size: usize) {
        self.frame_used.fetch_sub(size, Ordering::Relaxed);
    }

    /// Record a pool allocation.
    pub fn record_pool_alloc(&self, size: usize) -> usize {
        let new_used = self.pool_used.fetch_add(size, Ordering::Relaxed) + size;
        self.pool_peak.fetch_max(new_used, Ordering::Relaxed);
        new_used
    }

    /// Record a pool deallocation.
    pub fn record_pool_free(&self, size: usize) {
        self.pool_used.fetch_sub(size, Ordering::Relaxed);
    }

    /// Reset for new frame.
    pub fn reset_frame(&self) {
        self.frame_used.store(0, Ordering::Relaxed);
        self.warning_issued.store(false, Ordering::Relaxed);
    }

    /// Get current frame usage.
    pub fn frame_usage(&self) -> usize {
        self.frame_used.load(Ordering::Relaxed)
    }

    /// Get current pool usage.
    pub fn pool_usage(&self) -> usize {
        self.pool_used.load(Ordering::Relaxed)
    }
}

/// Result of a budget check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetCheckResult {
    /// Within budget.
    Ok,
    /// Warning threshold exceeded.
    Warning,
    /// Budget exceeded, action taken per policy.
    Exceeded(BudgetExceededPolicy),
}

/// Manager for per-thread budgets.
pub struct ThreadBudgetManager {
    /// Default configuration for new threads.
    default_config: Mutex<ThreadBudgetConfig>,
    /// Per-thread configurations.
    thread_configs: Mutex<HashMap<ThreadId, ThreadBudgetConfig>>,
    /// Per-thread states.
    thread_states: Mutex<HashMap<ThreadId, ThreadBudgetState>>,
    /// Global enabled flag.
    enabled: AtomicBool,
    /// Custom exceeded handler.
    exceeded_handler: Mutex<Option<Box<dyn Fn(ThreadId, usize, usize) + Send + Sync>>>,
}

impl ThreadBudgetManager {
    /// Create a new budget manager.
    pub fn new() -> Self {
        Self {
            default_config: Mutex::new(ThreadBudgetConfig::default()),
            thread_configs: Mutex::new(HashMap::new()),
            thread_states: Mutex::new(HashMap::new()),
            enabled: AtomicBool::new(false),
            exceeded_handler: Mutex::new(None),
        }
    }

    /// Enable budget tracking.
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable budget tracking.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Check if budget tracking is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Set the default configuration for new threads.
    pub fn set_default_config(&self, config: ThreadBudgetConfig) {
        let mut default = self.default_config.lock().unwrap();
        *default = config;
    }

    /// Set configuration for a specific thread.
    pub fn set_thread_config(&self, thread_id: ThreadId, config: ThreadBudgetConfig) {
        let mut configs = self.thread_configs.lock().unwrap();
        configs.insert(thread_id, config);
    }

    /// Get configuration for a thread (or default).
    pub fn get_config(&self, thread_id: ThreadId) -> ThreadBudgetConfig {
        let configs = self.thread_configs.lock().unwrap();
        configs.get(&thread_id).cloned().unwrap_or_else(|| {
            self.default_config.lock().unwrap().clone()
        })
    }

    /// Get or create state for a thread.
    fn get_or_create_state(&self, thread_id: ThreadId) -> &ThreadBudgetState {
        let mut states = self.thread_states.lock().unwrap();
        if !states.contains_key(&thread_id) {
            states.insert(thread_id, ThreadBudgetState::new());
        }
        // SAFETY: We just ensured the entry exists and we hold the lock
        unsafe {
            let ptr = states.get(&thread_id).unwrap() as *const ThreadBudgetState;
            &*ptr
        }
    }

    /// Check frame budget before allocation.
    pub fn check_frame_budget(&self, thread_id: ThreadId, size: usize) -> BudgetCheckResult {
        if !self.is_enabled() {
            return BudgetCheckResult::Ok;
        }

        let config = self.get_config(thread_id);
        let state = self.get_or_create_state(thread_id);
        let current = state.frame_usage();
        let new_total = current + size;

        // Check exceeded
        if new_total > config.frame_budget {
            state.exceeded_count.fetch_add(1, Ordering::Relaxed);
            
            // Call custom handler if set
            if config.frame_exceeded_policy == BudgetExceededPolicy::Custom {
                if let Some(handler) = self.exceeded_handler.lock().unwrap().as_ref() {
                    handler(thread_id, new_total, config.frame_budget);
                }
            }
            
            return BudgetCheckResult::Exceeded(config.frame_exceeded_policy);
        }

        // Check warning threshold
        let warning_threshold = config.frame_budget * config.warning_threshold as usize / 100;
        if new_total > warning_threshold && !state.warning_issued.swap(true, Ordering::Relaxed) {
            return BudgetCheckResult::Warning;
        }

        BudgetCheckResult::Ok
    }

    /// Record a frame allocation (after budget check passed).
    pub fn record_frame_alloc(&self, thread_id: ThreadId, size: usize) {
        if !self.is_enabled() {
            return;
        }
        let state = self.get_or_create_state(thread_id);
        state.record_frame_alloc(size);
    }

    /// Record a frame deallocation.
    pub fn record_frame_free(&self, thread_id: ThreadId, size: usize) {
        if !self.is_enabled() {
            return;
        }
        let state = self.get_or_create_state(thread_id);
        state.record_frame_free(size);
    }

    /// Reset frame budget for a thread (called at frame end).
    pub fn reset_frame(&self, thread_id: ThreadId) {
        if !self.is_enabled() {
            return;
        }
        let state = self.get_or_create_state(thread_id);
        state.reset_frame();
    }

    /// Set a custom exceeded handler.
    pub fn set_exceeded_handler<F>(&self, handler: F)
    where
        F: Fn(ThreadId, usize, usize) + Send + Sync + 'static,
    {
        let mut h = self.exceeded_handler.lock().unwrap();
        *h = Some(Box::new(handler));
    }

    /// Get budget statistics for a thread.
    pub fn get_stats(&self, thread_id: ThreadId) -> Option<ThreadBudgetStats> {
        let states = self.thread_states.lock().unwrap();
        states.get(&thread_id).map(|state| {
            let config = self.get_config(thread_id);
            ThreadBudgetStats {
                frame_used: state.frame_usage(),
                frame_budget: config.frame_budget,
                frame_peak: state.frame_peak.load(Ordering::Relaxed),
                pool_used: state.pool_usage(),
                pool_budget: config.pool_budget,
                pool_peak: state.pool_peak.load(Ordering::Relaxed),
                exceeded_count: state.exceeded_count.load(Ordering::Relaxed),
            }
        })
    }

    /// Get remaining frame budget for current thread.
    pub fn frame_remaining(&self) -> usize {
        let thread_id = std::thread::current().id();
        let config = self.get_config(thread_id);
        let state = self.get_or_create_state(thread_id);
        config.frame_budget.saturating_sub(state.frame_usage())
    }
}

impl Default for ThreadBudgetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a thread's budget usage.
#[derive(Debug, Clone)]
pub struct ThreadBudgetStats {
    /// Current frame usage.
    pub frame_used: usize,
    /// Frame budget.
    pub frame_budget: usize,
    /// Peak frame usage.
    pub frame_peak: usize,
    /// Current pool usage.
    pub pool_used: usize,
    /// Pool budget.
    pub pool_budget: usize,
    /// Peak pool usage.
    pub pool_peak: usize,
    /// Number of times exceeded.
    pub exceeded_count: usize,
}

impl ThreadBudgetStats {
    /// Get frame usage as percentage.
    pub fn frame_usage_percent(&self) -> f32 {
        if self.frame_budget == 0 {
            return 0.0;
        }
        (self.frame_used as f32 / self.frame_budget as f32) * 100.0
    }

    /// Get pool usage as percentage.
    pub fn pool_usage_percent(&self) -> f32 {
        if self.pool_budget == 0 {
            return 0.0;
        }
        (self.pool_used as f32 / self.pool_budget as f32) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_config_defaults() {
        let config = ThreadBudgetConfig::default();
        assert_eq!(config.frame_budget, 16 * 1024 * 1024);
        assert_eq!(config.pool_budget, 8 * 1024 * 1024);
    }

    #[test]
    fn test_budget_check_disabled() {
        let manager = ThreadBudgetManager::new();
        let result = manager.check_frame_budget(std::thread::current().id(), 1000);
        assert_eq!(result, BudgetCheckResult::Ok);
    }

    #[test]
    fn test_budget_check_enabled() {
        let manager = ThreadBudgetManager::new();
        manager.enable();
        manager.set_default_config(ThreadBudgetConfig {
            frame_budget: 1000,
            ..Default::default()
        });

        let tid = std::thread::current().id();
        
        // Under budget
        let result = manager.check_frame_budget(tid, 500);
        assert_eq!(result, BudgetCheckResult::Ok);
        
        // Over budget
        manager.record_frame_alloc(tid, 500);
        let result = manager.check_frame_budget(tid, 600);
        assert!(matches!(result, BudgetCheckResult::Exceeded(_)));
    }
}
