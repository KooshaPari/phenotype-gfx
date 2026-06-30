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
//! ```rust,no_run
//! // Example: log to stderr with tracing-subscriber
//! use tracing_subscriber::EnvFilter;
//! tracing_subscriber::fmt()
//!     .with_env_filter(EnvFilter::from_default_env())
//!     .init();
//! ```
//!
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

/// Increment a named counter metric.  No-op when no recorder is installed.
///
/// Usage: `gfx_count!("phenotype_gfx.lod_plan_calls")`
#[macro_export]
macro_rules! gfx_count {
    ($name:expr) => { ::metrics::counter!($name).increment(1) };
    ($name:expr, $n:expr) => { ::metrics::counter!($name).increment($n) };
}
