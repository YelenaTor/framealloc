//! Unified budget management for CPU and GPU memory

/// Manages unified memory budgets across CPU and GPU
pub struct UnifiedBudgetManager {
    /// Current CPU memory usage
    cpu_usage: usize,
    /// Current GPU memory usage
    gpu_usage: usize,
    /// CPU memory budget
    cpu_budget: usize,
    /// GPU memory budget
    gpu_budget: usize,
    /// Combined memory budget
    combined_budget: usize,
    /// Frame-local usage tracking
    frame_cpu_usage: usize,
    frame_gpu_usage: usize,
}

impl UnifiedBudgetManager {
    /// Create a new budget manager with default limits
    pub fn new() -> Self {
        Self {
            cpu_usage: 0,
            gpu_usage: 0,
            cpu_budget: 512 * 1024 * 1024, // 512 MB
            gpu_budget: 1024 * 1024 * 1024, // 1 GB
            combined_budget: 1536 * 1024 * 1024, // 1.5 GB
            frame_cpu_usage: 0,
            frame_gpu_usage: 0,
        }
    }
    
    /// Create a new budget manager with custom limits
    pub fn with_limits(cpu_budget: usize, gpu_budget: usize) -> Self {
        let combined_budget = cpu_budget + gpu_budget;
        Self {
            cpu_usage: 0,
            gpu_usage: 0,
            cpu_budget,
            gpu_budget,
            combined_budget,
            frame_cpu_usage: 0,
            frame_gpu_usage: 0,
        }
    }
    
    /// Check if we can allocate the given amount on CPU
    pub fn check_cpu_budget(&self, size: usize) -> bool {
        self.cpu_usage + size <= self.cpu_budget &&
        self.cpu_usage + self.gpu_usage + size <= self.combined_budget
    }
    
    /// Check if we can allocate the given amount on GPU
    pub fn check_gpu_budget(&self, size: usize) -> bool {
        self.gpu_usage + size <= self.gpu_budget &&
        self.cpu_usage + self.gpu_usage + size <= self.combined_budget
    }
    
    /// Check if we can allocate the given combined amount
    pub fn check_combined_budget(&self, size: usize) -> bool {
        self.cpu_usage + self.gpu_usage + size <= self.combined_budget
    }
    
    /// Add CPU memory usage
    pub fn add_cpu_usage(&mut self, size: usize) {
        self.cpu_usage += size;
        self.frame_cpu_usage += size;
    }
    
    /// Add GPU memory usage
    pub fn add_gpu_usage(&mut self, size: usize) {
        self.gpu_usage += size;
        self.frame_gpu_usage += size;
    }
    
    /// Remove CPU memory usage
    pub fn remove_cpu_usage(&mut self, size: usize) {
        self.cpu_usage = self.cpu_usage.saturating_sub(size);
    }
    
    /// Remove GPU memory usage
    pub fn remove_gpu_usage(&mut self, size: usize) {
        self.gpu_usage = self.gpu_usage.saturating_sub(size);
    }
    
    /// Get current CPU usage
    pub fn cpu_usage(&self) -> usize {
        self.cpu_usage
    }
    
    /// Get current GPU usage
    pub fn gpu_usage(&self) -> usize {
        self.gpu_usage
    }
    
    /// Get CPU budget
    pub fn cpu_budget(&self) -> usize {
        self.cpu_budget
    }
    
    /// Get GPU budget
    pub fn gpu_budget(&self) -> usize {
        self.gpu_budget
    }
    
    /// Get combined budget
    pub fn combined_budget(&self) -> usize {
        self.combined_budget
    }
    
    /// Get CPU utilization (0.0 to 1.0)
    pub fn cpu_utilization(&self) -> f32 {
        self.cpu_usage as f32 / self.cpu_budget as f32
    }
    
    /// Get GPU utilization (0.0 to 1.0)
    pub fn gpu_utilization(&self) -> f32 {
        self.gpu_usage as f32 / self.gpu_budget as f32
    }
    
    /// Get combined utilization (0.0 to 1.0)
    pub fn combined_utilization(&self) -> f32 {
        (self.cpu_usage + self.gpu_usage) as f32 / self.combined_budget as f32
    }
    
    /// Reset frame-local usage tracking
    pub fn reset_frame_usage(&mut self) {
        self.frame_cpu_usage = 0;
        self.frame_gpu_usage = 0;
    }
    
    /// Get frame-local CPU usage
    pub fn frame_cpu_usage(&self) -> usize {
        self.frame_cpu_usage
    }
    
    /// Get frame-local GPU usage
    pub fn frame_gpu_usage(&self) -> usize {
        self.frame_gpu_usage
    }
}

impl Default for UnifiedBudgetManager {
    fn default() -> Self {
        Self::new()
    }
}
