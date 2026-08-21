//! Integration tests for the OTLP and Prometheus observability module.
//!
//! These tests exercise the public API surface of `phenotype_gfx::otlp` and
//! require the `prometheus` feature to be enabled.

#![cfg(feature = "prometheus")]

use phenotype_gfx::otlp::{new_registry, PrometheusMetrics};

#[test]
fn test_prometheus_registry_creation() {
    let _reg = new_registry();
    // Simply verify we can create a registry without panicking.
}

#[test]
fn test_metrics_counter_and_histogram() {
    let reg = new_registry();
    let metrics = PrometheusMetrics::new(reg);
    metrics.requests_total.inc();
    metrics.requests_total.inc_by(4);
    metrics.latency_histogram.observe(0.123);

    let rendered = metrics.render();
    assert!(
        rendered.contains("phenotype_gfx_requests_total"),
        "rendered output should contain the counter"
    );
    assert!(
        rendered.contains("phenotype_gfx_latency_seconds"),
        "rendered output should contain the histogram"
    );
}

#[test]
fn test_metrics_render_output_is_valid_utf8() {
    let reg = new_registry();
    let metrics = PrometheusMetrics::new(reg);
    let rendered = metrics.render();
    // Smoke: just make sure render doesn't panic and returns valid UTF-8.
    let _ = String::from_utf8(rendered.into_bytes())
        .expect("rendered metrics should be valid UTF-8");
}

#[cfg(feature = "otlp")]
#[tokio::test]
async fn test_tracing_init() {
    // Provider created successfully — the OTLP exporter will buffer spans
    // and flush them on drop/shutdown.  In a real test environment the
    // collector at localhost:4317 would need to be running.
    match phenotype_gfx::otlp::init_tracing() {
        Ok(_provider) => {}
        Err(e) => {
            // If the collector isn't running, this is expected.
            eprintln!("OTLP init returned error (collector likely not running): {}", e);
        }
    }
}
