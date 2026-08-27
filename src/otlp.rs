//! OTLP trace export and Prometheus metrics HTTP endpoint.
//!
//! All functionality here is feature-gated behind `otlp` / `prometheus` /
//! `full-obs` features.  When the features are disabled this module is empty.

// ---------------------------------------------------------------------------
// OTLP tracing (feature: otlp)
// ---------------------------------------------------------------------------

#[cfg(feature = "otlp")]
pub use opentelemetry_sdk::trace::SdkTracerProvider;

/// Initialise an OTLP trace exporter targeting `localhost:4317` and return
/// the [`TracerProvider`].
///
/// The caller is responsible for keeping the provider alive for the duration
/// of the process.  Calling [`TracerProvider::shutdown`] on drop is
/// recommended.
///
/// # Panics
///
/// Panics when the OTLP collector at `localhost:4317` is unreachable or the
/// gRPC transport cannot be established.
#[cfg(feature = "otlp")]
pub fn init_tracing() -> Result<SdkTracerProvider, Box<dyn std::error::Error + Send + Sync>> {
    use opentelemetry_otlp::WithExportConfig;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();

    Ok(provider)
}

// ---------------------------------------------------------------------------
// Prometheus (feature: prometheus)
// ---------------------------------------------------------------------------

/// A thin wrapper around a [`prometheus::Registry`] that is pre-loaded with
/// the common phenotype-gfx request metrics.
#[cfg(feature = "prometheus")]
pub struct PrometheusMetrics {
    pub requests_total: prometheus::IntCounter,
    pub latency_histogram: prometheus::Histogram,
    registry: prometheus::Registry,
}

#[cfg(feature = "prometheus")]
impl PrometheusMetrics {
    /// Create a new metrics set and register it with the given
    /// [`prometheus::Registry`].
    pub fn new(registry: prometheus::Registry) -> Self {
        let requests_total = prometheus::IntCounter::with_opts(prometheus::Opts::new(
            "phenotype_gfx_requests_total",
            "Total number of requests served by the gfx pipeline",
        ))
        .expect("failed to create requests_total counter");

        let latency_histogram = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "phenotype_gfx_latency_seconds",
                "Request latency in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
        )
        .expect("failed to create latency_histogram");

        registry
            .register(Box::new(requests_total.clone()))
            .expect("failed to register requests_total");
        registry
            .register(Box::new(latency_histogram.clone()))
            .expect("failed to register latency_histogram");

        Self {
            requests_total,
            latency_histogram,
            registry,
        }
    }

    /// Render the current metrics snapshot in the Prometheus text exposition
    /// format.
    pub fn render(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .expect("prometheus encode should not fail");
        String::from_utf8(buffer).expect("utf8 should be valid after prometheus encode")
    }
}

/// Serve the Prometheus metrics endpoint on `0.0.0.0:<port>`.
///
/// This is a **blocking** helper intended to be spawned on its own thread or
/// via `std::thread::spawn`.  It returns only when the listener encounters a
/// fatal error.
#[cfg(feature = "prometheus")]
pub fn serve_metrics(port: u16, metrics: &PrometheusMetrics) {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("failed to bind metrics listener");

    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            // Read the full HTTP request (we don't really need it).
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);

            let body = metrics.render();
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/plain; version=0.0.4; charset=utf-8\r\n\
                 Content-Length: {}\r\n\
                 \r\n\
                 {}",
                body.len(),
                body,
            );
            let _ = stream.write_all(response.as_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Re-exports for convenience (always available)
// ---------------------------------------------------------------------------

/// Create a new [`prometheus::Registry`].
#[cfg(feature = "prometheus")]
pub fn new_registry() -> prometheus::Registry {
    prometheus::Registry::new()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "prometheus"))]
mod tests {
    use super::*;

    #[test]
    fn test_prometheus_registry_creation() {
        let _reg = new_registry();
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
}
