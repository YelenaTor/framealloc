//! Memory behavior tracking and filtering.
//!
//! This module provides runtime observation of allocation patterns to detect
//! "bad memory" — allocations that violate their declared intent.
//!
//! # What is "Bad Memory"?
//!
//! Bad memory is NOT unsafe memory. It's memory that contradicts the allocator
//! intent the developer declared:
//!
//! - Frame allocations that behave like long-lived data
//! - Pool allocations used as scratch (freed same frame)
//! - Heap allocations made in hot paths
//! - Repeated promotions every frame (promotion churn)
//!
//! # Design
//!
//! Tracking is done **per-tag**, not per-allocation, to minimize overhead.
//! This gives pattern detection with O(tags) memory instead of O(allocations).
//!
//! # Usage
//!
//! ```rust,ignore
//! // Enable the filter
//! alloc.enable_behavior_filter();
//!
//! // Run your game loop...
//!
//! // Check for issues
//! let report = alloc.behavior_report();
//! for issue in report.issues() {
//!     eprintln!("{}", issue);
//! }
//! ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::cell::RefCell;

use crate::diagnostics::{DiagnosticCode, DiagnosticLevel};

/// Allocation kind for behavior tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocKind {
    Frame,
    Pool,
    Heap,
    Scratch,
}

impl std::fmt::Display for AllocKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Frame => write!(f, "frame"),
            Self::Pool => write!(f, "pool"),
            Self::Heap => write!(f, "heap"),
            Self::Scratch => write!(f, "scratch"),
        }
    }
}

/// Per-tag behavior statistics.
///
/// Tracks allocation patterns for a specific tag to detect intent violations.
#[derive(Debug, Clone)]
pub struct TagBehaviorStats {
    /// The tag being tracked
    pub tag: &'static str,
    /// Allocation kind
    pub kind: AllocKind,
    /// Total allocations observed
    pub total_allocs: u64,
    /// Allocations that survived more than one frame
    pub survived_frame_count: u64,
    /// Total frames these allocations lived
    pub total_lifetime_frames: u64,
    /// Number of promotions
    pub promotion_count: u64,
    /// Allocations freed in the same frame they were created
    pub same_frame_frees: u64,
    /// Peak bytes allocated
    pub peak_bytes: usize,
    /// Current bytes allocated
    pub current_bytes: usize,
    /// Frame number when last allocation occurred
    pub last_alloc_frame: u64,
    /// Frame number when tracking started
    pub first_seen_frame: u64,
}

impl TagBehaviorStats {
    /// Create new stats for a tag.
    pub fn new(tag: &'static str, kind: AllocKind, frame: u64) -> Self {
        Self {
            tag,
            kind,
            total_allocs: 0,
            survived_frame_count: 0,
            total_lifetime_frames: 0,
            promotion_count: 0,
            same_frame_frees: 0,
            peak_bytes: 0,
            current_bytes: 0,
            last_alloc_frame: frame,
            first_seen_frame: frame,
        }
    }
    
    /// Average lifetime in frames for allocations that survived.
    pub fn avg_lifetime_frames(&self) -> f32 {
        if self.survived_frame_count == 0 {
            0.0
        } else {
            self.total_lifetime_frames as f32 / self.survived_frame_count as f32
        }
    }
    
    /// Rate of promotions per frame (0.0 - 1.0+).
    pub fn promotion_rate(&self) -> f32 {
        let frames_active = self.last_alloc_frame.saturating_sub(self.first_seen_frame) + 1;
        if frames_active == 0 {
            0.0
        } else {
            self.promotion_count as f32 / frames_active as f32
        }
    }
    
    /// Rate of same-frame frees (0.0 - 1.0).
    pub fn same_frame_free_rate(&self) -> f32 {
        if self.total_allocs == 0 {
            0.0
        } else {
            self.same_frame_frees as f32 / self.total_allocs as f32
        }
    }
    
    /// Survival rate (allocations that lived > 1 frame).
    pub fn survival_rate(&self) -> f32 {
        if self.total_allocs == 0 {
            0.0
        } else {
            self.survived_frame_count as f32 / self.total_allocs as f32
        }
    }
}

/// A detected behavior issue.
#[derive(Debug, Clone)]
pub struct BehaviorIssue {
    /// Diagnostic code
    pub code: DiagnosticCode,
    /// Severity level
    pub level: DiagnosticLevel,
    /// The tag involved
    pub tag: &'static str,
    /// Allocation kind
    pub kind: AllocKind,
    /// Human-readable message
    pub message: String,
    /// Suggestion for fixing
    pub suggestion: String,
    /// Observed value that triggered the issue
    pub observed_value: String,
    /// Threshold that was exceeded
    pub threshold: String,
}

impl std::fmt::Display for BehaviorIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}: {} allocation behaves unexpectedly\n  \
             tag: {}\n  \
             observed: {}\n  \
             threshold: {}\n  \
             suggestion: {}",
            self.code,
            self.level,
            self.kind,
            self.tag,
            self.observed_value,
            self.threshold,
            self.suggestion
        )
    }
}

/// Thresholds for behavior detection.
#[derive(Debug, Clone)]
pub struct BehaviorThresholds {
    /// Frame allocations surviving more than this many frames trigger FA501
    pub frame_survival_frames: u64,
    /// Frame allocation survival rate above this triggers FA502
    pub frame_survival_rate: f32,
    /// Pool allocations with same-frame-free rate above this trigger FA510
    pub pool_same_frame_free_rate: f32,
    /// Promotion rate above this triggers FA520
    pub promotion_churn_rate: f32,
    /// Heap allocations in frame phases above this count trigger FA530
    pub heap_in_hot_path_count: u64,
    /// Minimum allocations before analysis kicks in
    pub min_samples: u64,
}

impl Default for BehaviorThresholds {
    fn default() -> Self {
        Self {
            frame_survival_frames: 60,        // ~1 second at 60fps
            frame_survival_rate: 0.5,         // 50% surviving is suspicious
            pool_same_frame_free_rate: 0.8,   // 80% freed same frame = misuse
            promotion_churn_rate: 0.5,        // Promoting every other frame
            heap_in_hot_path_count: 100,      // 100 heap allocs in hot path
            min_samples: 10,                  // Need at least 10 allocs to analyze
        }
    }
}

impl BehaviorThresholds {
    /// Strict thresholds for CI/testing.
    pub fn strict() -> Self {
        Self {
            frame_survival_frames: 10,
            frame_survival_rate: 0.2,
            pool_same_frame_free_rate: 0.5,
            promotion_churn_rate: 0.2,
            heap_in_hot_path_count: 10,
            min_samples: 5,
        }
    }
    
    /// Relaxed thresholds for development.
    pub fn relaxed() -> Self {
        Self {
            frame_survival_frames: 300,       // 5 seconds
            frame_survival_rate: 0.8,
            pool_same_frame_free_rate: 0.95,
            promotion_churn_rate: 0.8,
            heap_in_hot_path_count: 1000,
            min_samples: 50,
        }
    }
}

/// Behavior analysis report.
#[derive(Debug, Clone)]
pub struct BehaviorReport {
    /// Detected issues
    pub issues: Vec<BehaviorIssue>,
    /// Per-tag statistics
    pub stats: Vec<TagBehaviorStats>,
    /// Total frames analyzed
    pub frames_analyzed: u64,
    /// Whether the filter was enabled
    pub filter_enabled: bool,
}

impl BehaviorReport {
    /// Get issues at or above a severity level.
    pub fn issues_at_level(&self, min_level: DiagnosticLevel) -> impl Iterator<Item = &BehaviorIssue> {
        self.issues.iter().filter(move |i| i.level >= min_level)
    }
    
    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.level == DiagnosticLevel::Error)
    }
    
    /// Check if there are any warnings or errors.
    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.level >= DiagnosticLevel::Warning)
    }
    
    /// Get summary string.
    pub fn summary(&self) -> String {
        let errors = self.issues.iter().filter(|i| i.level == DiagnosticLevel::Error).count();
        let warnings = self.issues.iter().filter(|i| i.level == DiagnosticLevel::Warning).count();
        let hints = self.issues.iter().filter(|i| i.level == DiagnosticLevel::Hint).count();
        
        format!(
            "Behavior analysis: {} errors, {} warnings, {} hints ({} frames, {} tags)",
            errors, warnings, hints, self.frames_analyzed, self.stats.len()
        )
    }
}

/// The behavior filter - tracks and analyzes allocation patterns.
pub struct BehaviorFilter {
    /// Whether filtering is enabled
    enabled: AtomicBool,
    /// Current frame number
    current_frame: AtomicU64,
    /// Detection thresholds
    thresholds: BehaviorThresholds,
    /// Per-tag statistics (tag+kind -> stats)
    stats: RefCell<HashMap<(&'static str, AllocKind), TagBehaviorStats>>,
    /// Pending allocations this frame (for same-frame-free detection)
    pending_this_frame: RefCell<HashMap<*const u8, (&'static str, AllocKind, usize)>>,
}

impl BehaviorFilter {
    /// Create a new behavior filter.
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            current_frame: AtomicU64::new(0),
            thresholds: BehaviorThresholds::default(),
            stats: RefCell::new(HashMap::new()),
            pending_this_frame: RefCell::new(HashMap::new()),
        }
    }
    
    /// Create with custom thresholds.
    pub fn with_thresholds(thresholds: BehaviorThresholds) -> Self {
        Self {
            enabled: AtomicBool::new(false),
            current_frame: AtomicU64::new(0),
            thresholds,
            stats: RefCell::new(HashMap::new()),
            pending_this_frame: RefCell::new(HashMap::new()),
        }
    }
    
    /// Enable the filter.
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }
    
    /// Disable the filter.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }
    
    /// Check if enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
    
    /// Set thresholds.
    pub fn set_thresholds(&mut self, thresholds: BehaviorThresholds) {
        self.thresholds = thresholds;
    }
    
    /// Get current thresholds.
    pub fn thresholds(&self) -> &BehaviorThresholds {
        &self.thresholds
    }
    
    /// Record an allocation.
    pub fn record_alloc(&self, ptr: *const u8, tag: &'static str, kind: AllocKind, size: usize) {
        if !self.is_enabled() {
            return;
        }
        
        let frame = self.current_frame.load(Ordering::SeqCst);
        let key = (tag, kind);
        
        let mut stats = self.stats.borrow_mut();
        let entry = stats.entry(key).or_insert_with(|| TagBehaviorStats::new(tag, kind, frame));
        
        entry.total_allocs += 1;
        entry.current_bytes += size;
        entry.peak_bytes = entry.peak_bytes.max(entry.current_bytes);
        entry.last_alloc_frame = frame;
        
        // Track for same-frame-free detection
        self.pending_this_frame.borrow_mut().insert(ptr, (tag, kind, size));
    }
    
    /// Record a deallocation.
    pub fn record_free(&self, ptr: *const u8, tag: &'static str, kind: AllocKind, size: usize) {
        if !self.is_enabled() {
            return;
        }
        
        let key = (tag, kind);
        
        // Check if this was allocated this frame
        let same_frame = self.pending_this_frame.borrow_mut().remove(&ptr).is_some();
        
        let mut stats = self.stats.borrow_mut();
        if let Some(entry) = stats.get_mut(&key) {
            entry.current_bytes = entry.current_bytes.saturating_sub(size);
            if same_frame {
                entry.same_frame_frees += 1;
            }
        }
    }
    
    /// Record a promotion.
    pub fn record_promotion(&self, tag: &'static str, from_kind: AllocKind) {
        if !self.is_enabled() {
            return;
        }
        
        let key = (tag, from_kind);
        let mut stats = self.stats.borrow_mut();
        if let Some(entry) = stats.get_mut(&key) {
            entry.promotion_count += 1;
        }
    }
    
    /// Record frame survival (called at frame end for surviving allocations).
    pub fn record_survival(&self, tag: &'static str, kind: AllocKind, frames_alive: u64) {
        if !self.is_enabled() {
            return;
        }
        
        let key = (tag, kind);
        let mut stats = self.stats.borrow_mut();
        if let Some(entry) = stats.get_mut(&key) {
            entry.survived_frame_count += 1;
            entry.total_lifetime_frames += frames_alive;
        }
    }
    
    /// Called at frame end.
    pub fn end_frame(&self) {
        if !self.is_enabled() {
            return;
        }
        
        self.current_frame.fetch_add(1, Ordering::SeqCst);
        self.pending_this_frame.borrow_mut().clear();
    }
    
    /// Get current frame number.
    pub fn current_frame(&self) -> u64 {
        self.current_frame.load(Ordering::SeqCst)
    }
    
    /// Analyze behavior and generate report.
    pub fn analyze(&self) -> BehaviorReport {
        let stats_map = self.stats.borrow();
        let stats: Vec<_> = stats_map.values().cloned().collect();
        let frames = self.current_frame.load(Ordering::SeqCst);
        
        let mut issues = Vec::new();
        
        for stat in &stats {
            if stat.total_allocs < self.thresholds.min_samples {
                continue;
            }
            
            // FA501: Frame allocation survives too long
            if stat.kind == AllocKind::Frame {
                let avg_lifetime = stat.avg_lifetime_frames();
                if avg_lifetime > self.thresholds.frame_survival_frames as f32 {
                    issues.push(BehaviorIssue {
                        code: FA501,
                        level: DiagnosticLevel::Warning,
                        tag: stat.tag,
                        kind: stat.kind,
                        message: "Frame allocation behaves like long-lived data".into(),
                        suggestion: "Consider using pool_alloc() or scratch_pool()".into(),
                        observed_value: format!("avg lifetime: {:.1} frames", avg_lifetime),
                        threshold: format!("expected < {} frames", self.thresholds.frame_survival_frames),
                    });
                }
            }
            
            // FA502: High frame allocation survival rate
            if stat.kind == AllocKind::Frame {
                let survival_rate = stat.survival_rate();
                if survival_rate > self.thresholds.frame_survival_rate {
                    issues.push(BehaviorIssue {
                        code: FA502,
                        level: DiagnosticLevel::Warning,
                        tag: stat.tag,
                        kind: stat.kind,
                        message: "High frame allocation survival rate".into(),
                        suggestion: "Frame allocations should be ephemeral; use pool or heap".into(),
                        observed_value: format!("{:.0}% survive beyond frame", survival_rate * 100.0),
                        threshold: format!("expected < {:.0}%", self.thresholds.frame_survival_rate * 100.0),
                    });
                }
            }
            
            // FA510: Pool allocation used as scratch
            if stat.kind == AllocKind::Pool {
                let same_frame_rate = stat.same_frame_free_rate();
                if same_frame_rate > self.thresholds.pool_same_frame_free_rate {
                    issues.push(BehaviorIssue {
                        code: FA510,
                        level: DiagnosticLevel::Hint,
                        tag: stat.tag,
                        kind: stat.kind,
                        message: "Pool allocation used as scratch memory".into(),
                        suggestion: "Consider using frame_alloc() for ephemeral data".into(),
                        observed_value: format!("{:.0}% freed same frame", same_frame_rate * 100.0),
                        threshold: format!("expected < {:.0}%", self.thresholds.pool_same_frame_free_rate * 100.0),
                    });
                }
            }
            
            // FA520: Promotion churn
            let promotion_rate = stat.promotion_rate();
            if promotion_rate > self.thresholds.promotion_churn_rate {
                issues.push(BehaviorIssue {
                    code: FA520,
                    level: DiagnosticLevel::Warning,
                    tag: stat.tag,
                    kind: stat.kind,
                    message: "Excessive promotion churn detected".into(),
                    suggestion: "Consider allocating directly in the target allocator".into(),
                    observed_value: format!("{:.2} promotions/frame", promotion_rate),
                    threshold: format!("expected < {:.2}/frame", self.thresholds.promotion_churn_rate),
                });
            }
            
            // FA530: Heap allocation in hot path (high frequency)
            if stat.kind == AllocKind::Heap {
                let allocs_per_frame = stat.total_allocs as f32 / frames.max(1) as f32;
                if allocs_per_frame > self.thresholds.heap_in_hot_path_count as f32 / 60.0 {
                    issues.push(BehaviorIssue {
                        code: FA530,
                        level: DiagnosticLevel::Warning,
                        tag: stat.tag,
                        kind: stat.kind,
                        message: "Frequent heap allocations detected".into(),
                        suggestion: "Consider using pool_alloc() or frame_alloc() for hot paths".into(),
                        observed_value: format!("{:.1} heap allocs/frame", allocs_per_frame),
                        threshold: format!("expected < {:.1}/frame", self.thresholds.heap_in_hot_path_count as f32 / 60.0),
                    });
                }
            }
        }
        
        // Sort by severity
        issues.sort_by(|a, b| b.level.cmp(&a.level));
        
        BehaviorReport {
            issues,
            stats,
            frames_analyzed: frames,
            filter_enabled: self.is_enabled(),
        }
    }
    
    /// Reset all statistics.
    pub fn reset(&self) {
        self.stats.borrow_mut().clear();
        self.pending_this_frame.borrow_mut().clear();
        self.current_frame.store(0, Ordering::SeqCst);
    }
}

impl Default for BehaviorFilter {
    fn default() -> Self {
        Self::new()
    }
}

// Diagnostic codes for behavior issues
/// Frame allocation survives too long
pub const FA501: DiagnosticCode = DiagnosticCode::new("FA501");
/// High frame allocation survival rate
pub const FA502: DiagnosticCode = DiagnosticCode::new("FA502");
/// Pool allocation used as scratch
pub const FA510: DiagnosticCode = DiagnosticCode::new("FA510");
/// Promotion churn detected
pub const FA520: DiagnosticCode = DiagnosticCode::new("FA520");
/// Heap allocation in hot path
pub const FA530: DiagnosticCode = DiagnosticCode::new("FA530");

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_behavior_filter_disabled_by_default() {
        let filter = BehaviorFilter::new();
        assert!(!filter.is_enabled());
    }
    
    #[test]
    fn test_behavior_filter_enable_disable() {
        let filter = BehaviorFilter::new();
        filter.enable();
        assert!(filter.is_enabled());
        filter.disable();
        assert!(!filter.is_enabled());
    }
    
    #[test]
    fn test_tag_behavior_stats() {
        let stats = TagBehaviorStats::new("test", AllocKind::Frame, 0);
        assert_eq!(stats.tag, "test");
        assert_eq!(stats.kind, AllocKind::Frame);
        assert_eq!(stats.total_allocs, 0);
    }
    
    #[test]
    fn test_behavior_thresholds() {
        let default = BehaviorThresholds::default();
        let strict = BehaviorThresholds::strict();
        let relaxed = BehaviorThresholds::relaxed();
        
        assert!(strict.frame_survival_frames < default.frame_survival_frames);
        assert!(relaxed.frame_survival_frames > default.frame_survival_frames);
    }
    
    #[test]
    fn test_behavior_report_summary() {
        let report = BehaviorReport {
            issues: vec![],
            stats: vec![],
            frames_analyzed: 100,
            filter_enabled: true,
        };
        
        let summary = report.summary();
        assert!(summary.contains("100 frames"));
    }
    
    #[test]
    fn test_record_alloc_when_disabled() {
        let filter = BehaviorFilter::new();
        // Should not panic or do anything when disabled
        filter.record_alloc(std::ptr::null(), "test", AllocKind::Frame, 64);
        
        let report = filter.analyze();
        assert!(report.stats.is_empty());
    }
    
    #[test]
    fn test_record_alloc_when_enabled() {
        let filter = BehaviorFilter::new();
        filter.enable();
        
        filter.record_alloc(0x1000 as *const u8, "physics", AllocKind::Frame, 64);
        filter.record_alloc(0x2000 as *const u8, "physics", AllocKind::Frame, 128);
        
        let report = filter.analyze();
        assert_eq!(report.stats.len(), 1);
        assert_eq!(report.stats[0].total_allocs, 2);
        assert_eq!(report.stats[0].current_bytes, 192);
    }
    
    #[test]
    fn test_same_frame_free_detection() {
        let filter = BehaviorFilter::new();
        filter.enable();
        
        let ptr = 0x1000 as *const u8;
        filter.record_alloc(ptr, "test", AllocKind::Pool, 64);
        filter.record_free(ptr, "test", AllocKind::Pool, 64);
        
        let report = filter.analyze();
        assert_eq!(report.stats[0].same_frame_frees, 1);
    }
    
    #[test]
    fn test_frame_advancement() {
        let filter = BehaviorFilter::new();
        filter.enable();
        
        assert_eq!(filter.current_frame(), 0);
        filter.end_frame();
        assert_eq!(filter.current_frame(), 1);
        filter.end_frame();
        assert_eq!(filter.current_frame(), 2);
    }
}
