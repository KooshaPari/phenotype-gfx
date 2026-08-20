//! Observability façade for the phenotype-gfx kernel.
//!
//! Wires structured tracing spans and metrics counters into the kernel's
//! hot paths.  Everything here is **zero-cost when no subscriber / recorder
//! is installed**: `tracing` macros compile away, `metrics` macros resolve
//! to no-ops at link time.
//!
//! ## Consumers
//!
//! Install a subscriber before calling kernel code:
//!
//! ```rust,ignore
//! // Example: log to stderr with tracing-subscriber
//! // Add `tracing-subscriber` as a dev-dependency to run this example.
//! use tracing_subscriber::EnvFilter;
//! tracing_subscriber::fmt()
//!     .with_env_filter(EnvFilter::from_default_env())
//!     .init();
//! ```
//! Install a metrics recorder to export counters/gauges:
//!
//! ```rust,no_run
//! // Example: Prometheus recorder (metrics-exporter-prometheus crate)
//! // metrics_exporter_prometheus::PrometheusBuilder::new().install().unwrap();
//! ```
//!
//! ## Metric names
//!
//! All metrics are prefixed `phenotype_gfx.`:
//!
//! | Name | Kind | Description |
//! |---|---|---|
//! | `phenotype_gfx.lod_plan_calls` | counter | invocations of `plan_render_lod` |
//! | `phenotype_gfx.lod_chunks_visible` | histogram-like counter | visible chunks per call |
//! | `phenotype_gfx.streaming_load` | counter | chunks promoted to Loaded |
//! | `phenotype_gfx.streaming_unload` | counter | chunks evicted from the ring |
//! | `phenotype_gfx.voxel_mesh_builds` | counter | `build_mesh` invocations |
//! | `phenotype_gfx.terrain_mesh_builds` | counter | terrain mesh build invocations |
//! | `phenotype_gfx.postfx_stack_runs` | counter | post-process stack executions |
//! | `phenotype_gfx.water_mesh_builds` | counter | water mesh build invocations |
//! | `phenotype_gfx.voxelizer_runs` | counter | sprite-voxelizer invocations |

// Re-export the macros so callers within this crate can import from `crate::obs`.
pub use metrics::{counter, gauge};
pub use tracing::{debug, error, info, instrument, span, trace, warn, Level};

/// Emit a structured event at TRACE level.  No-op when no subscriber is wired.
#[macro_export]
macro_rules! gfx_trace {
    ($($arg:tt)*) => { ::tracing::trace!($($arg)*) };
}

/// Emit a structured event at DEBUG level.  No-op when no subscriber is wired.
#[macro_export]
macro_rules! gfx_debug {
    ($($arg:tt)*) => { ::tracing::debug!($($arg)*) };
}

/// Emit a structured event at INFO level.  No-op when no subscriber is wired.
#[macro_export]
macro_rules! gfx_info {
    ($($arg:tt)*) => { ::tracing::info!($($arg)*) };
}

/// Emit a structured event at WARN level.  No-op when no subscriber is wired.
#[macro_export]
macro_rules! gfx_warn {
    ($($arg:tt)*) => { ::tracing::warn!($($arg)*) };
}

/// Emit a structured event at ERROR level.  No-op when no subscriber is wired.
#[macro_export]
macro_rules! gfx_error {
    ($($arg:tt)*) => { ::tracing::error!($($arg)*) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx_count;

    // ======================================================================
    // tracing subscriber init
    // ======================================================================

    #[test]
    fn init_tracing_subscriber_no_panic() {
        let _ = tracing_subscriber::fmt().try_init();
    }

    #[test]
    fn multiple_subscriber_init_graceful() {
        let _ = tracing_subscriber::fmt().try_init();
        let _ = tracing_subscriber::fmt().try_init();
    }

    // ======================================================================
    // counter lifecycle
    // ======================================================================

    #[test]
    fn create_and_increment_counter() {
        let c = counter!("phenotype_gfx.test_counter");
        c.increment(1);
        // Increment should not panic even with no recorder installed
    }

    #[test]
    fn counter_increment_by_arbitrary_amount() {
        let c = counter!("phenotype_gfx.test_counter_batch");
        c.increment(42);
        c.increment(7);
    }

    #[test]
    fn counter_is_send_and_sync() {
        let c = counter!("phenotype_gfx.test_counter_threadsafe");
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<metrics::Counter>();
        c.increment(1);
    }

    // ======================================================================
    // gauge lifecycle
    // ======================================================================

    #[test]
    fn create_and_set_gauge() {
        let g = gauge!("phenotype_gfx.test_gauge");
        g.set(42.0);
        // Set should not panic even with no recorder installed
    }

    #[test]
    fn gauge_set_to_zero() {
        let g = gauge!("phenotype_gfx.test_gauge_zero");
        g.set(0.0);
    }

    #[test]
    fn gauge_overwrite_previous_value() {
        let g = gauge!("phenotype_gfx.test_gauge_overwrite");
        g.set(10.0);
        g.set(20.0);
    }

    // ======================================================================
    // span creation and entry
    // ======================================================================

    #[test]
    fn span_creation_and_entry() {
        let _guard = span!(Level::INFO, "test_span").entered();
    }

    #[test]
    fn span_with_fields() {
        let _guard = span!(Level::DEBUG, "fielded_span", chunk = 42, ring = 3).entered();
    }

    #[test]
    fn nested_spans() {
        let _outer = span!(Level::INFO, "outer").entered();
        let _inner = span!(Level::DEBUG, "inner").entered();
    }

    // ======================================================================
    // gfx_* macro smoke tests — verify macros compile and don't panic
    // ======================================================================

    #[test]
    fn gfx_trace_macro_smoke() {
        gfx_trace!("trace message chunk={}", 7);
    }

    #[test]
    fn gfx_debug_macro_smoke() {
        gfx_debug!("debug message ring={}", 3);
    }

    #[test]
    fn gfx_info_macro_smoke() {
        gfx_info!("info message anchor={}", "origin");
    }

    #[test]
    fn gfx_warn_macro_smoke() {
        gfx_warn!("warn message budget={}", 1024);
    }

    #[test]
    fn gfx_error_macro_smoke() {
        gfx_error!("error message code={}", 500);
    }

    #[test]
    fn gfx_count_macro_smoke() {
        gfx_count!("phenotype_gfx.test_gfx_count");
        gfx_count!("phenotype_gfx.test_gfx_count_batch", 5);
    }
}

/// Increment a named counter metric.  No-op when no recorder is installed.
///
/// Usage: `gfx_count!("phenotype_gfx.lod_plan_calls")`
#[macro_export]
macro_rules! gfx_count {
    ($name:expr) => {
        ::metrics::counter!($name).increment(1)
    };
    ($name:expr, $n:expr) => {
        ::metrics::counter!($name).increment($n)
    };
}
