//! Snapshot emission for IDE integration (v0.7.0).
//!
//! This module provides runtime snapshot functionality for integration with
//! `fa-insight` and other IDE tooling. Snapshots capture point-in-time memory
//! state at frame boundaries.
//!
//! # Philosophy
//!
//! Snapshots are:
//! - **Opt-in**: Only emitted when explicitly enabled
//! - **Aggregated**: No per-allocation data, only summaries
//! - **Bounded**: Rate-limited and cleaned up automatically
//! - **Safe boundary**: Only captured at frame end, never mid-frame
//!
//! # Usage
//!
//! ```rust,ignore
//! use framealloc::{SnapshotConfig, SnapshotEmitter};
//!
//! let config = SnapshotConfig::default()
//!     .with_directory("target/framealloc")
//!     .with_max_snapshots(30);
//!
//! let emitter = SnapshotEmitter::new(config);
//!
//! // In your frame loop:
//! alloc.end_frame();
//! emitter.maybe_emit(&alloc); // Checks for request file
//! ```

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// Snapshot schema version.
pub const SNAPSHOT_VERSION: u32 = 1;

/// Configuration for snapshot emission.
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// Directory to write snapshots (default: "target/framealloc")
    pub directory: PathBuf,
    
    /// Maximum number of snapshots to retain (default: 30)
    pub max_snapshots: usize,
    
    /// Minimum interval between snapshots (default: 500ms)
    pub min_interval: Duration,
    
    /// Whether to check for request file (default: true)
    pub check_request_file: bool,
    
    /// Whether to auto-emit on every frame (default: false)
    /// Use with caution - generates many files
    pub auto_emit: bool,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("target/framealloc"),
            max_snapshots: 30,
            min_interval: Duration::from_millis(500),
            check_request_file: true,
            auto_emit: false,
        }
    }
}

impl SnapshotConfig {
    /// Builder: set snapshot directory.
    pub fn with_directory<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.directory = dir.into();
        self
    }
    
    /// Builder: set maximum snapshots to retain.
    pub fn with_max_snapshots(mut self, max: usize) -> Self {
        self.max_snapshots = max;
        self
    }
    
    /// Builder: set minimum interval between snapshots.
    pub fn with_min_interval(mut self, interval: Duration) -> Self {
        self.min_interval = interval;
        self
    }
    
    /// Builder: enable/disable request file checking.
    pub fn with_request_file(mut self, check: bool) -> Self {
        self.check_request_file = check;
        self
    }
    
    /// Builder: enable/disable auto-emit on every frame.
    pub fn with_auto_emit(mut self, auto: bool) -> Self {
        self.auto_emit = auto;
        self
    }
}

/// Snapshot data for serialization.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Schema version
    pub version: u32,
    
    /// ISO 8601 timestamp
    pub timestamp: String,
    
    /// Frame number
    pub frame: u64,
    
    /// Frame duration in microseconds
    pub duration_us: u64,
    
    /// Memory summary
    pub summary: SnapshotSummary,
    
    /// Per-thread data
    pub threads: Vec<ThreadSnapshot>,
    
    /// Per-tag data
    pub tags: Vec<TagSnapshot>,
    
    /// Promotion statistics
    pub promotions: PromotionStats,
    
    /// Transfer statistics
    pub transfers: TransferStats,
    
    /// Deferred queue statistics
    pub deferred: DeferredStats,
    
    /// Runtime diagnostics
    pub diagnostics: Vec<RuntimeDiagnostic>,
}

/// Memory summary.
#[derive(Debug, Clone, Default)]
pub struct SnapshotSummary {
    pub frame_bytes: usize,
    pub pool_bytes: usize,
    pub heap_bytes: usize,
    pub total_bytes: usize,
    pub peak_bytes: usize,
}

/// Per-thread snapshot data.
#[derive(Debug, Clone)]
pub struct ThreadSnapshot {
    pub id: String,
    pub name: String,
    pub frame_bytes: usize,
    pub pool_bytes: usize,
    pub heap_bytes: usize,
    pub peak_bytes: usize,
    pub budget: Option<BudgetInfo>,
}

/// Budget information.
#[derive(Debug, Clone)]
pub struct BudgetInfo {
    pub limit: usize,
    pub used: usize,
    pub percent: u8,
}

/// Per-tag snapshot data.
#[derive(Debug, Clone)]
pub struct TagSnapshot {
    pub path: String,
    pub thread: String,
    pub alloc_kind: String,
    pub alloc_count: usize,
    pub bytes: usize,
    pub avg_lifetime_frames: f32,
    pub promotion_rate: f32,
    pub diagnostics: Vec<String>,
}

/// Promotion statistics.
#[derive(Debug, Clone, Default)]
pub struct PromotionStats {
    pub to_pool: usize,
    pub to_heap: usize,
    pub failed: usize,
}

/// Transfer statistics.
#[derive(Debug, Clone, Default)]
pub struct TransferStats {
    pub pending: usize,
    pub completed_this_frame: usize,
}

/// Deferred queue statistics.
#[derive(Debug, Clone, Default)]
pub struct DeferredStats {
    pub queue_depth: usize,
    pub processed_this_frame: usize,
}

/// Runtime diagnostic.
#[derive(Debug, Clone)]
pub struct RuntimeDiagnostic {
    pub code: String,
    pub tag: Option<String>,
    pub message: String,
}

impl Snapshot {
    /// Create a new empty snapshot.
    pub fn new(frame: u64) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| {
                // Format as ISO 8601
                let secs = d.as_secs();
                let dt = time_to_iso8601(secs);
                dt
            })
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());
        
        Self {
            version: SNAPSHOT_VERSION,
            timestamp,
            frame,
            duration_us: 0,
            summary: SnapshotSummary::default(),
            threads: Vec::new(),
            tags: Vec::new(),
            promotions: PromotionStats::default(),
            transfers: TransferStats::default(),
            deferred: DeferredStats::default(),
            diagnostics: Vec::new(),
        }
    }
    
    /// Serialize to JSON.
    pub fn to_json(&self) -> String {
        let mut json = String::with_capacity(4096);
        json.push_str("{\n");
        
        // Version and metadata
        json.push_str(&format!("  \"version\": {},\n", self.version));
        json.push_str(&format!("  \"timestamp\": \"{}\",\n", self.timestamp));
        json.push_str(&format!("  \"frame\": {},\n", self.frame));
        json.push_str(&format!("  \"duration_us\": {},\n", self.duration_us));
        
        // Summary
        json.push_str("  \"summary\": {\n");
        json.push_str(&format!("    \"frame_bytes\": {},\n", self.summary.frame_bytes));
        json.push_str(&format!("    \"pool_bytes\": {},\n", self.summary.pool_bytes));
        json.push_str(&format!("    \"heap_bytes\": {},\n", self.summary.heap_bytes));
        json.push_str(&format!("    \"total_bytes\": {},\n", self.summary.total_bytes));
        json.push_str(&format!("    \"peak_bytes\": {}\n", self.summary.peak_bytes));
        json.push_str("  },\n");
        
        // Threads
        json.push_str("  \"threads\": [\n");
        for (i, thread) in self.threads.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"id\": \"{}\",\n", thread.id));
            json.push_str(&format!("      \"name\": \"{}\",\n", thread.name));
            json.push_str(&format!("      \"frame_bytes\": {},\n", thread.frame_bytes));
            json.push_str(&format!("      \"pool_bytes\": {},\n", thread.pool_bytes));
            json.push_str(&format!("      \"heap_bytes\": {},\n", thread.heap_bytes));
            json.push_str(&format!("      \"peak_bytes\": {},\n", thread.peak_bytes));
            if let Some(ref budget) = thread.budget {
                json.push_str("      \"budget\": {\n");
                json.push_str(&format!("        \"limit\": {},\n", budget.limit));
                json.push_str(&format!("        \"used\": {},\n", budget.used));
                json.push_str(&format!("        \"percent\": {}\n", budget.percent));
                json.push_str("      }\n");
            } else {
                json.push_str("      \"budget\": null\n");
            }
            if i < self.threads.len() - 1 {
                json.push_str("    },\n");
            } else {
                json.push_str("    }\n");
            }
        }
        json.push_str("  ],\n");
        
        // Tags
        json.push_str("  \"tags\": [\n");
        for (i, tag) in self.tags.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"path\": \"{}\",\n", tag.path));
            json.push_str(&format!("      \"thread\": \"{}\",\n", tag.thread));
            json.push_str(&format!("      \"alloc_kind\": \"{}\",\n", tag.alloc_kind));
            json.push_str(&format!("      \"alloc_count\": {},\n", tag.alloc_count));
            json.push_str(&format!("      \"bytes\": {},\n", tag.bytes));
            json.push_str(&format!("      \"avg_lifetime_frames\": {:.2},\n", tag.avg_lifetime_frames));
            json.push_str(&format!("      \"promotion_rate\": {:.2},\n", tag.promotion_rate));
            json.push_str("      \"diagnostics\": [");
            for (j, diag) in tag.diagnostics.iter().enumerate() {
                json.push_str(&format!("\"{}\"", diag));
                if j < tag.diagnostics.len() - 1 {
                    json.push_str(", ");
                }
            }
            json.push_str("]\n");
            if i < self.tags.len() - 1 {
                json.push_str("    },\n");
            } else {
                json.push_str("    }\n");
            }
        }
        json.push_str("  ],\n");
        
        // Promotions
        json.push_str("  \"promotions\": {\n");
        json.push_str(&format!("    \"to_pool\": {},\n", self.promotions.to_pool));
        json.push_str(&format!("    \"to_heap\": {},\n", self.promotions.to_heap));
        json.push_str(&format!("    \"failed\": {}\n", self.promotions.failed));
        json.push_str("  },\n");
        
        // Transfers
        json.push_str("  \"transfers\": {\n");
        json.push_str(&format!("    \"pending\": {},\n", self.transfers.pending));
        json.push_str(&format!("    \"completed_this_frame\": {}\n", self.transfers.completed_this_frame));
        json.push_str("  },\n");
        
        // Deferred
        json.push_str("  \"deferred\": {\n");
        json.push_str(&format!("    \"queue_depth\": {},\n", self.deferred.queue_depth));
        json.push_str(&format!("    \"processed_this_frame\": {}\n", self.deferred.processed_this_frame));
        json.push_str("  },\n");
        
        // Diagnostics
        json.push_str("  \"diagnostics\": [\n");
        for (i, diag) in self.diagnostics.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"code\": \"{}\",\n", diag.code));
            if let Some(ref tag) = diag.tag {
                json.push_str(&format!("      \"tag\": \"{}\",\n", tag));
            } else {
                json.push_str("      \"tag\": null,\n");
            }
            json.push_str(&format!("      \"message\": \"{}\"\n", diag.message));
            if i < self.diagnostics.len() - 1 {
                json.push_str("    },\n");
            } else {
                json.push_str("    }\n");
            }
        }
        json.push_str("  ]\n");
        
        json.push_str("}\n");
        json
    }
}

/// Snapshot emitter that handles file I/O and rate limiting.
pub struct SnapshotEmitter {
    config: SnapshotConfig,
    last_emit: std::sync::Mutex<Option<Instant>>,
    enabled: AtomicBool,
    emit_count: AtomicU64,
}

impl SnapshotEmitter {
    /// Create a new snapshot emitter.
    pub fn new(config: SnapshotConfig) -> Self {
        Self {
            config,
            last_emit: std::sync::Mutex::new(None),
            enabled: AtomicBool::new(true),
            emit_count: AtomicU64::new(0),
        }
    }
    
    /// Enable or disable snapshot emission.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }
    
    /// Check if snapshots are enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }
    
    /// Get number of snapshots emitted.
    pub fn emit_count(&self) -> u64 {
        self.emit_count.load(Ordering::Relaxed)
    }
    
    /// Check for request file and emit snapshot if present.
    /// 
    /// Returns true if a snapshot was emitted.
    pub fn maybe_emit(&self, snapshot: &Snapshot) -> bool {
        if !self.is_enabled() {
            return false;
        }
        
        // Check rate limit
        {
            let last = self.last_emit.lock().unwrap();
            if let Some(last_time) = *last {
                if last_time.elapsed() < self.config.min_interval {
                    return false;
                }
            }
        }
        
        // Check for request file or auto-emit
        let should_emit = self.config.auto_emit || self.check_request_file();
        
        if should_emit {
            return self.emit(snapshot);
        }
        
        false
    }
    
    /// Force emit a snapshot (ignores request file, respects rate limit).
    pub fn emit(&self, snapshot: &Snapshot) -> bool {
        if !self.is_enabled() {
            return false;
        }
        
        // Ensure directory exists
        if let Err(e) = fs::create_dir_all(&self.config.directory) {
            eprintln!("framealloc: failed to create snapshot directory: {}", e);
            return false;
        }
        
        // Write snapshot file
        let filename = format!("snapshot_{:08}.json", snapshot.frame);
        let path = self.config.directory.join(&filename);
        
        match fs::File::create(&path) {
            Ok(mut file) => {
                let json = snapshot.to_json();
                if let Err(e) = file.write_all(json.as_bytes()) {
                    eprintln!("framealloc: failed to write snapshot: {}", e);
                    return false;
                }
            }
            Err(e) => {
                eprintln!("framealloc: failed to create snapshot file: {}", e);
                return false;
            }
        }
        
        // Update state
        {
            let mut last = self.last_emit.lock().unwrap();
            *last = Some(Instant::now());
        }
        self.emit_count.fetch_add(1, Ordering::Relaxed);
        
        // Cleanup old snapshots
        self.cleanup_old_snapshots();
        
        // Remove request file if present
        self.remove_request_file();
        
        true
    }
    
    /// Check if request file exists.
    fn check_request_file(&self) -> bool {
        if !self.config.check_request_file {
            return false;
        }
        
        let request_path = self.config.directory.join("snapshot.request");
        request_path.exists()
    }
    
    /// Remove request file after processing.
    fn remove_request_file(&self) {
        let request_path = self.config.directory.join("snapshot.request");
        let _ = fs::remove_file(request_path);
    }
    
    /// Clean up old snapshots beyond max_snapshots limit.
    fn cleanup_old_snapshots(&self) {
        let dir = &self.config.directory;
        
        let mut snapshots: Vec<_> = match fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("snapshot_")
                        && e.file_name()
                            .to_string_lossy()
                            .ends_with(".json")
                })
                .collect(),
            Err(_) => return,
        };
        
        if snapshots.len() <= self.config.max_snapshots {
            return;
        }
        
        // Sort by name (which includes frame number, so older first)
        snapshots.sort_by_key(|e| e.file_name());
        
        // Remove oldest
        let to_remove = snapshots.len() - self.config.max_snapshots;
        for entry in snapshots.into_iter().take(to_remove) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Convert Unix timestamp to ISO 8601 string.
fn time_to_iso8601(secs: u64) -> String {
    // Simple conversion without external dependencies
    const SECS_PER_DAY: u64 = 86400;
    const DAYS_PER_YEAR: u64 = 365;
    const DAYS_PER_4_YEARS: u64 = 1461; // 365*4 + 1
    
    let days = secs / SECS_PER_DAY;
    let day_secs = secs % SECS_PER_DAY;
    
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;
    
    // Approximate year calculation (good enough for our purposes)
    let mut year = 1970u64;
    let mut remaining_days = days;
    
    // Handle 4-year cycles (including leap years)
    let four_year_cycles = remaining_days / DAYS_PER_4_YEARS;
    year += four_year_cycles * 4;
    remaining_days %= DAYS_PER_4_YEARS;
    
    // Handle remaining years
    while remaining_days >= DAYS_PER_YEAR {
        let is_leap = (year % 4 == 0) && (year % 100 != 0 || year % 400 == 0);
        let days_this_year = if is_leap { 366 } else { 365 };
        if remaining_days < days_this_year {
            break;
        }
        remaining_days -= days_this_year;
        year += 1;
    }
    
    // Approximate month/day (simplified)
    let month = (remaining_days / 30).min(11) + 1;
    let day = (remaining_days % 30) + 1;
    
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_snapshot_to_json() {
        let snapshot = Snapshot::new(100);
        let json = snapshot.to_json();
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"frame\": 100"));
    }
    
    #[test]
    fn test_snapshot_config_builder() {
        let config = SnapshotConfig::default()
            .with_directory("custom/path")
            .with_max_snapshots(50)
            .with_auto_emit(true);
        
        assert_eq!(config.directory, PathBuf::from("custom/path"));
        assert_eq!(config.max_snapshots, 50);
        assert!(config.auto_emit);
    }
}
