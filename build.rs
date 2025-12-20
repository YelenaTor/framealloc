//! Build script for framealloc.
//!
//! Provides build-time diagnostics, feature detection, and helpful messages
//! for users integrating framealloc into their projects.

use std::env;

fn main() {
    // Re-run if features change
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_BEVY");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_DEBUG");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_PARKING_LOT");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_TRACY");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_NIGHTLY");

    // Collect enabled features
    let bevy_enabled = env::var("CARGO_FEATURE_BEVY").is_ok();
    let debug_enabled = env::var("CARGO_FEATURE_DEBUG").is_ok();
    let parking_lot_enabled = env::var("CARGO_FEATURE_PARKING_LOT").is_ok();
    let tracy_enabled = env::var("CARGO_FEATURE_TRACY").is_ok();
    let nightly_enabled = env::var("CARGO_FEATURE_NIGHTLY").is_ok();

    // Get build profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let is_release = profile == "release";

    // =========================================================================
    // Feature-specific diagnostics
    // =========================================================================

    // --- Bevy Integration ---
    if bevy_enabled {
        emit_info("Bevy integration enabled");
        emit_note("Remember to add SmartAllocPlugin to your Bevy App:");
        emit_note("  app.add_plugins(framealloc::bevy::SmartAllocPlugin::default())");
        emit_note("");
        emit_note("The plugin will:");
        emit_note("  • Insert AllocResource as a Bevy resource");
        emit_note("  • Reset frame arenas at frame boundaries");
        emit_note("  • Work correctly with Bevy's parallel systems");
        
        // Check Bevy version compatibility (if we can detect it)
        if let Ok(bevy_version) = env::var("DEP_BEVY_VERSION") {
            emit_info(&format!("Detected Bevy version: {}", bevy_version));
        }
    }

    // --- Debug Features ---
    if debug_enabled {
        emit_info("Debug features enabled");
        emit_note("Debug mode provides:");
        emit_note("  • Memory poisoning (freed memory filled with 0xCD)");
        emit_note("  • Allocation backtraces (for leak detection)");
        emit_note("  • Extended validation checks");
        
        if is_release {
            emit_warning("Debug features enabled in release build!");
            emit_note("This may impact performance. Consider disabling for production.");
        }
    } else if !is_release {
        emit_note("Tip: Enable 'debug' feature for memory poisoning and allocation tracking:");
        emit_note("  framealloc = { version = \"0.1\", features = [\"debug\"] }");
    }

    // --- Parking Lot ---
    if parking_lot_enabled {
        emit_info("Using parking_lot for mutexes (faster lock implementation)");
    }

    // --- Tracy Integration ---
    if tracy_enabled {
        emit_info("Tracy profiler integration enabled");
        emit_note("Use ProfilerHooks to connect to Tracy:");
        emit_note("  let mut hooks = ProfilerHooks::new();");
        emit_note("  hooks.set_callback(|event| { /* send to tracy */ });");
    }

    // --- Nightly Features ---
    if nightly_enabled {
        emit_info("Nightly features enabled (std::alloc::Allocator trait)");
        emit_note("You can now use framealloc with standard collections:");
        emit_note("  let frame_alloc = alloc.frame_allocator();");
        emit_note("  let vec: Vec<u32, _> = Vec::new_in(frame_alloc);");
        
        // Check if actually on nightly
        check_nightly_compiler();
    }

    // =========================================================================
    // Release build recommendations
    // =========================================================================

    if is_release {
        emit_info("Building in release mode");
        
        if !parking_lot_enabled {
            emit_note("Tip: Consider enabling 'parking_lot' for better mutex performance:");
            emit_note("  framealloc = { version = \"0.1\", features = [\"parking_lot\"] }");
        }
    }

    // =========================================================================
    // Common usage reminders
    // =========================================================================

    emit_separator();
    emit_info("framealloc Quick Reference");
    emit_separator();
    emit_note("Frame allocation (fastest, reset per frame):");
    emit_note("  alloc.begin_frame();");
    emit_note("  let data = alloc.frame_box(value);");
    emit_note("  alloc.end_frame();");
    emit_note("");
    emit_note("Pool allocation (small objects, auto-freed):");
    emit_note("  let boxed = alloc.pool_box(value);");
    emit_note("");
    emit_note("Heap allocation (large objects, auto-freed):");
    emit_note("  let large = alloc.heap_box(large_value);");
    emit_separator();

    // =========================================================================
    // Environment checks
    // =========================================================================

    check_target_features();
}

// =============================================================================
// Diagnostic emission helpers
// =============================================================================

fn emit_info(msg: &str) {
    println!("cargo:warning=[framealloc] ℹ️  {}", msg);
}

fn emit_note(msg: &str) {
    if msg.is_empty() {
        println!("cargo:warning=[framealloc]");
    } else {
        println!("cargo:warning=[framealloc]    {}", msg);
    }
}

fn emit_warning(msg: &str) {
    println!("cargo:warning=[framealloc] ⚠️  {}", msg);
}

#[allow(dead_code)]
fn emit_error(msg: &str) {
    println!("cargo:warning=[framealloc] ❌ {}", msg);
}

fn emit_separator() {
    println!("cargo:warning=[framealloc] ────────────────────────────────────────");
}

// =============================================================================
// Environment and toolchain checks
// =============================================================================

fn check_nightly_compiler() {
    // Try to detect if we're on nightly by checking rustc version
    if let Ok(rustc) = env::var("RUSTC") {
        if let Ok(output) = std::process::Command::new(&rustc)
            .arg("--version")
            .output()
        {
            let version = String::from_utf8_lossy(&output.stdout);
            if !version.contains("nightly") {
                emit_warning("'nightly' feature enabled but compiler doesn't appear to be nightly!");
                emit_note("The std::alloc::Allocator trait requires nightly Rust.");
                emit_note("Install nightly: rustup install nightly");
                emit_note("Use nightly: rustup override set nightly");
            }
        }
    }
}

fn check_target_features() {
    // Check for target-specific optimizations
    let target = env::var("TARGET").unwrap_or_default();
    
    if target.contains("x86_64") {
        // x86_64 specific hints
        if env::var("CARGO_CFG_TARGET_FEATURE").map(|f| f.contains("avx2")).unwrap_or(false) {
            emit_info("AVX2 available - memory operations may be vectorized");
        }
    }
    
    if target.contains("wasm") {
        emit_warning("WebAssembly target detected");
        emit_note("framealloc works on WASM but with some limitations:");
        emit_note("  • No true threading (use single-threaded mode)");
        emit_note("  • Memory budget may be constrained");
    }
    
    if target.contains("windows") {
        // Windows-specific notes
        emit_info("Building for Windows");
    } else if target.contains("linux") {
        emit_info("Building for Linux");
    } else if target.contains("darwin") || target.contains("macos") {
        emit_info("Building for macOS");
    }
}
