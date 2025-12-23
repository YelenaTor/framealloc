<p align="center">
  <h1 align="center">cargo-fa</h1>
  <p align="center">
    <strong>Static analysis for framealloc — catch memory intent violations before runtime</strong>
  </p>
</p>

<p align="center">
  <a href="https://crates.io/crates/cargo-fa"><img src="https://img.shields.io/crates/v/cargo-fa.svg?style=flat-square" alt="Crates.io"></a>
  <a href="https://docs.rs/cargo-fa"><img src="https://img.shields.io/docsrs/cargo-fa?style=flat-square" alt="Documentation"></a>
  <a href="#license"><img src="https://img.shields.io/crates/l/cargo-fa?style=flat-square" alt="License"></a>
</p>

<p align="center">
  <a href="#installation">Installation</a> •
  <a href="#usage">Usage</a> •
  <a href="#diagnostics">Diagnostics</a> •
  <a href="#ci-integration">CI Integration</a>
</p>

---

## Overview

**cargo-fa** is a static analysis tool for [framealloc](https://crates.io/crates/framealloc) that detects memory intent violations at build time. It catches patterns that compile but violate frame allocation principles — issues that would otherwise only surface as performance problems or subtle bugs at runtime.

### What It Catches

| Category | Examples |
|----------|----------|
| **Lifetime Issues** | Frame allocations escaping scope, hot loop allocations |
| **Async Safety** | Frame data crossing await points, closure captures |
| **Threading** | Cross-thread frame access, missing thread-local init |
| **Architecture** | Tag mismatches, unknown tags, module boundary violations |
| **Budgets** | Unbounded allocation loops |

---

## Installation

```bash
cargo install cargo-fa
```

Or from source:

```bash
git clone https://github.com/YelenaTor/framealloc
cd framealloc/cargo-fa
cargo install --path .
```

---

## Usage

### Basic Checks

```bash
# Check specific categories
cargo fa --dirtymem       # Lifetime/escape issues (FA6xx)
cargo fa --async-safety   # Async/await issues (FA7xx)
cargo fa --threading      # Thread safety issues (FA2xx)
cargo fa --budgets        # Budget violations (FA3xx)
cargo fa --architecture   # Tag/module issues (FA8xx)

# Run all checks (optimized order)
cargo fa --all
```

### Filtering

```bash
# Treat specific diagnostic as error
cargo fa --all --deny FA701

# Suppress specific diagnostic
cargo fa --all --allow FA602

# Exclude paths (glob pattern)
cargo fa --all --exclude "**/tests/**"

# Stop on first error
cargo fa --all --fail-fast

# Minimum severity threshold
cargo fa --all --min-severity warning
```

### Output Formats

```bash
# Human-readable (default)
cargo fa --all

# JSON for programmatic consumption
cargo fa --all --format json

# SARIF for GitHub Actions
cargo fa --all --format sarif

# JUnit XML for test reporters
cargo fa --all --format junit

# Checkstyle XML for Jenkins
cargo fa --all --format checkstyle

# Compact one-line-per-issue
cargo fa --all --format compact
```

### Subcommands

```bash
# Explain a diagnostic code in detail
cargo fa explain FA601

# Analyze a single file
cargo fa show src/physics.rs

# List all diagnostic codes
cargo fa list

# Filter by category
cargo fa list --category async

# Generate configuration file
cargo fa init
```

---

## Diagnostics

### Diagnostic Codes

| Range | Category | Description |
|-------|----------|-------------|
| FA2xx | Threading | Cross-thread frame access, thread-local issues |
| FA3xx | Budgets | Unbounded allocations, missing budget guards |
| FA6xx | Lifetime | Frame escape, hot loops, missing boundaries |
| FA7xx | Async | Await crossing, closure capture, async functions |
| FA8xx | Architecture | Tag mismatch, unknown tags, module violations |

### Example Output

```
error[FA701]: frame allocation in async function
  --> src/network/client.rs:45:12
   |
45 |     let buffer = alloc.frame_box(vec![0u8; 1024]);
   |            ^^^^^^ frame allocation here
   |
   = note: async functions can suspend across frame boundaries
   = help: use `alloc.heap_box()` or `alloc.pool_box()` instead

warning[FA602]: allocation in hot loop
  --> src/physics/collision.rs:128:16
   |
128|         let contact = alloc.pool_alloc::<Contact>();
   |                ^^^^^^ allocation inside loop
   |
   = note: loop may execute many times per frame
   = help: consider pre-allocating with `alloc.frame_vec()`
```

### Getting Detailed Explanations

```bash
$ cargo fa explain FA701

━━━ FA701 ━━━

Name: async-frame
Category: Async Safety
Severity: error

Summary
Frame allocation in async function

Description
Async functions can suspend at await points. When they resume, they might
be on a different thread or at a different point in the frame lifecycle...

Example (incorrect)
async fn load_asset(alloc: &SmartAlloc) {
    let buffer = alloc.frame_box(vec![0u8; 1024]);  // FA701
    ...

Example (correct)
async fn load_asset(alloc: &SmartAlloc) {
    let buffer = alloc.heap_box(vec![0u8; 1024]);  // Safe
    ...
```

---

## CI Integration

### GitHub Actions (SARIF)

```yaml
- name: Run cargo-fa
  run: cargo fa --all --format sarif > results.sarif

- name: Upload SARIF
  uses: github/codeql-action/upload-sarif@v2
  with:
    sarif_file: results.sarif
```

### Jenkins (Checkstyle)

```groovy
stage('Static Analysis') {
    sh 'cargo fa --all --format checkstyle > checkstyle.xml'
    recordIssues tools: [checkStyle(pattern: 'checkstyle.xml')]
}
```

### Generic CI

```bash
# Exit with error on any issues
cargo fa --all --deny-warnings

# Exit with error only on specific codes
cargo fa --all --deny FA701 --deny FA702
```

---

## Configuration

Create a `.fa.toml` in your project root:

```bash
cargo fa init
```

Example configuration:

```toml
[global]
enabled = true
exclude = ["target/**", "tests/**"]

[lints.FA601]
level = "warn"

[lints.FA701]
level = "deny"

[tags]
known = ["physics", "rendering", "audio", "network"]

[tags.modules]
"src/physics/**" = "physics"
"src/render/**" = "rendering"
```

---

## Related

- [framealloc](https://crates.io/crates/framealloc) — The memory allocation library
- [TECHNICAL.md](https://github.com/YelenaTor/framealloc/blob/main/TECHNICAL.md) — Architecture documentation

---

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
