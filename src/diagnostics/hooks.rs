//! Diagnostic hooks and callbacks for UI integration.

use std::sync::Arc;

use crate::api::stats::AllocStats;
use crate::sync::mutex::Mutex;

use super::snapshot::{AllocatorSnapshot, SnapshotHistory};

/// Events that can be emitted by the allocator for diagnostics.
#[derive(Debug, Clone)]
pub enum DiagnosticsEvent {
    /// Frame started
    FrameBegin { frame_number: u64 },
    
    /// Frame ended
    FrameEnd { frame_number: u64 },
    
    /// Large allocation occurred
    LargeAllocation { size: usize, tag: Option<&'static str> },
    
    /// Memory pressure detected
    MemoryPressure { current: usize, limit: usize },
    
    /// Slab refill occurred
    SlabRefill { size_class: usize, count: usize },
    
    /// Cross-thread free processed
    DeferredFree { count: usize },
    
    /// Budget warning
    BudgetWarning { tag: &'static str, current: usize, limit: usize },
    
    /// Budget exceeded
    BudgetExceeded { tag: &'static str, current: usize, limit: usize },
}

/// Hooks for integrating with debug UIs.
pub struct DiagnosticsHooks {
    /// Whether diagnostics are enabled
    enabled: bool,
    
    /// Current frame number
    frame_number: u64,
    
    /// Snapshot history for graphing
    history: SnapshotHistory,
    
    /// Event listeners
    listeners: Vec<Box<dyn Fn(&DiagnosticsEvent) + Send + Sync>>,
    
    /// Custom data provider for UI
    data_provider: Option<Arc<dyn DiagnosticsProvider + Send + Sync>>,
}

impl DiagnosticsHooks {
    /// Create new diagnostics hooks.
    pub fn new() -> Self {
        Self {
            enabled: true,
            frame_number: 0,
            history: SnapshotHistory::default(),
            listeners: Vec::new(),
            data_provider: None,
        }
    }
    
    /// Enable or disable diagnostics.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Check if diagnostics are enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Register an event listener.
    pub fn add_listener<F>(&mut self, listener: F)
    where
        F: Fn(&DiagnosticsEvent) + Send + Sync + 'static,
    {
        self.listeners.push(Box::new(listener));
    }
    
    /// Set a custom data provider.
    pub fn set_provider<P>(&mut self, provider: P)
    where
        P: DiagnosticsProvider + Send + Sync + 'static,
    {
        self.data_provider = Some(Arc::new(provider));
    }
    
    /// Emit an event to all listeners.
    pub fn emit(&self, event: DiagnosticsEvent) {
        if !self.enabled {
            return;
        }
        
        for listener in &self.listeners {
            listener(&event);
        }
    }
    
    /// Called at frame start.
    pub fn on_frame_begin(&mut self) {
        self.frame_number += 1;
        self.emit(DiagnosticsEvent::FrameBegin {
            frame_number: self.frame_number,
        });
    }
    
    /// Called at frame end with current stats.
    pub fn on_frame_end(&mut self, stats: &AllocStats) {
        self.emit(DiagnosticsEvent::FrameEnd {
            frame_number: self.frame_number,
        });
        
        // Take a snapshot if we have a provider
        if let Some(ref provider) = self.data_provider {
            let snapshot = provider.take_snapshot(self.frame_number);
            self.history.push(snapshot);
        }
        
        let _ = stats; // Used by provider
    }
    
    /// Get the snapshot history.
    pub fn history(&self) -> &SnapshotHistory {
        &self.history
    }
    
    /// Get mutable access to history.
    pub fn history_mut(&mut self) -> &mut SnapshotHistory {
        &mut self.history
    }
    
    /// Get the current frame number.
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }
    
    /// Get data formatted for a memory graph.
    pub fn get_memory_graph_data(&self, max_points: usize) -> MemoryGraphData {
        let timeline = self.history.memory_timeline();
        let peak_timeline = self.history.peak_timeline();
        
        let step = if timeline.len() > max_points {
            timeline.len() / max_points
        } else {
            1
        };
        
        MemoryGraphData {
            current: timeline.iter().step_by(step).map(|&(_, v)| v).collect(),
            peak: peak_timeline.iter().step_by(step).map(|&(_, v)| v).collect(),
            frames: timeline.iter().step_by(step).map(|&(f, _)| f).collect(),
        }
    }
}

impl Default for DiagnosticsHooks {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for providing diagnostic data.
pub trait DiagnosticsProvider {
    /// Take a snapshot of current state.
    fn take_snapshot(&self, frame_number: u64) -> AllocatorSnapshot;
    
    /// Get current stats.
    fn get_stats(&self) -> AllocStats;
}

/// Data formatted for a memory usage graph.
#[derive(Debug, Clone, Default)]
pub struct MemoryGraphData {
    /// Current memory values over time
    pub current: Vec<usize>,
    
    /// Peak memory values over time
    pub peak: Vec<usize>,
    
    /// Frame numbers corresponding to values
    pub frames: Vec<u64>,
}

impl MemoryGraphData {
    /// Get the maximum value for scaling.
    pub fn max_value(&self) -> usize {
        self.peak.iter().copied().max().unwrap_or(0)
    }
    
    /// Normalize values to 0.0-1.0 range.
    pub fn normalized_current(&self) -> Vec<f32> {
        let max = self.max_value() as f32;
        if max == 0.0 {
            return vec![0.0; self.current.len()];
        }
        self.current.iter().map(|&v| v as f32 / max).collect()
    }
}

/// Thread-safe wrapper for diagnostics hooks.
pub struct SharedDiagnostics {
    inner: Mutex<DiagnosticsHooks>,
}

impl SharedDiagnostics {
    /// Create new shared diagnostics.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(DiagnosticsHooks::new()),
        }
    }
    
    /// Access diagnostics with a closure.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut DiagnosticsHooks) -> R,
    {
        let mut hooks = self.inner.lock();
        f(&mut hooks)
    }
    
    /// Emit an event.
    pub fn emit(&self, event: DiagnosticsEvent) {
        let hooks = self.inner.lock();
        hooks.emit(event);
    }
}

impl Default for SharedDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_event_emission() {
        let mut hooks = DiagnosticsHooks::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        let counter_clone = counter.clone();
        hooks.add_listener(move |_event| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });
        
        hooks.emit(DiagnosticsEvent::FrameBegin { frame_number: 1 });
        hooks.emit(DiagnosticsEvent::FrameEnd { frame_number: 1 });
        
        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_disabled_hooks() {
        let mut hooks = DiagnosticsHooks::new();
        hooks.set_enabled(false);
        
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        hooks.add_listener(move |_event| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });
        
        hooks.emit(DiagnosticsEvent::FrameBegin { frame_number: 1 });
        
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }
}
