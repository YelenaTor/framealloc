//! Configuration for cargo-fa.
//!
//! Loads settings from `.fa.toml` in the project root.

use crate::cli::{Args, Severity};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Configuration loaded from `.fa.toml`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Global settings
    pub global: GlobalConfig,
    
    /// Per-lint configuration
    pub lints: LintConfig,
    
    /// Tag definitions and rules
    pub tags: TagConfig,
    
    /// Threshold overrides
    pub thresholds: ThresholdConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    /// Minimum severity to report
    pub min_severity: String,
    
    /// Paths to exclude from analysis
    pub exclude: Vec<String>,
    
    /// Whether to fail on warnings in CI
    pub deny_warnings: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LintConfig {
    /// Lint levels: "allow", "warn", "deny"
    pub levels: HashMap<String, String>,
    
    /// Lints to completely disable
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TagConfig {
    /// Known allocation tags in this project
    pub known_tags: Vec<String>,
    
    /// Module-to-tag mappings for architecture enforcement
    pub module_tags: HashMap<String, Vec<String>>,
    
    /// Whether to warn on unknown tags
    pub warn_unknown_tags: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThresholdConfig {
    /// Max allocations in a loop before warning
    pub loop_allocation_limit: usize,
    
    /// Max frame survival frames
    pub frame_survival_frames: u64,
    
    /// Max promotion rate
    pub promotion_churn_rate: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            lints: LintConfig::default(),
            tags: TagConfig::default(),
            thresholds: ThresholdConfig::default(),
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            min_severity: "hint".to_string(),
            exclude: vec![
                "target/**".to_string(),
                "**/tests/**".to_string(),
            ],
            deny_warnings: false,
        }
    }
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            levels: HashMap::new(),
            disabled: Vec::new(),
        }
    }
}

impl Default for TagConfig {
    fn default() -> Self {
        Self {
            known_tags: vec![
                "physics".to_string(),
                "rendering".to_string(),
                "ai".to_string(),
                "audio".to_string(),
                "ui".to_string(),
                "network".to_string(),
            ],
            module_tags: HashMap::new(),
            warn_unknown_tags: false,
        }
    }
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self {
            loop_allocation_limit: 100,
            frame_survival_frames: 60,
            promotion_churn_rate: 0.5,
        }
    }
}

impl Config {
    /// Load configuration from `.fa.toml` or use defaults
    pub fn load(args: &Args) -> Result<Self> {
        let config_path = args.path.join(&args.config);
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: Config = toml::from_str(&content)?;
            
            // Apply CLI overrides
            config.apply_cli_overrides(args);
            
            Ok(config)
        } else {
            let mut config = Config::default();
            config.apply_cli_overrides(args);
            Ok(config)
        }
    }
    
    /// Apply command-line argument overrides
    fn apply_cli_overrides(&mut self, args: &Args) {
        if args.deny_warnings {
            self.global.deny_warnings = true;
        }
        
        // Apply --skip overrides
        if let Some(ref skip) = args.skip {
            self.lints.disabled.extend(skip.clone());
        }
    }
    
    /// Check if a lint is enabled
    pub fn is_lint_enabled(&self, code: &str) -> bool {
        !self.lints.disabled.contains(&code.to_string())
    }
    
    /// Get the level for a lint
    pub fn lint_level(&self, code: &str) -> LintLevel {
        self.lints
            .levels
            .get(code)
            .map(|s| match s.as_str() {
                "allow" => LintLevel::Allow,
                "warn" => LintLevel::Warn,
                "deny" => LintLevel::Deny,
                _ => LintLevel::Warn,
            })
            .unwrap_or(LintLevel::Warn)
    }
    
    /// Check if a path should be excluded
    pub fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in &self.global.exclude {
            if glob::Pattern::new(pattern)
                .map(|p| p.matches(&path_str))
                .unwrap_or(false)
            {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintLevel {
    Allow,
    Warn,
    Deny,
}

/// Generate a default `.fa.toml` configuration file
pub fn generate_default_config() -> String {
    r#"# framealloc static analysis configuration
# See https://docs.rs/framealloc/config for full documentation

[global]
# Minimum severity to report: "hint", "warning", "error"
min_severity = "hint"

# Paths to exclude from analysis (glob patterns)
exclude = [
    "target/**",
    "**/tests/**",
    "**/benches/**",
]

# Fail on warnings (useful for CI)
deny_warnings = false

[lints]
# Override lint levels: "allow", "warn", "deny"
# Example: FA601 = "deny"

[lints.levels]
# FA601 = "allow"  # Uncomment to allow frame escape warnings

[lints.disabled]
# Completely disable specific lints
# Example: ["FA602"]

[tags]
# Known allocation tags in your project
known_tags = [
    "physics",
    "rendering", 
    "ai",
    "audio",
    "ui",
    "network",
]

# Warn when using tags not in known_tags
warn_unknown_tags = false

# Module-to-tag mappings for architecture enforcement
# [tags.module_tags]
# "src/physics" = ["physics"]
# "src/rendering" = ["rendering"]

[thresholds]
# Max allocations in a loop before FA602 warning
loop_allocation_limit = 100

# Frames before FA601 warns about frame survival
frame_survival_frames = 60

# Promotions per frame before FA620 warning
promotion_churn_rate = 0.5
"#.to_string()
}
