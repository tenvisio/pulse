//! Metrics collection and export for Pulse.
//!
//! Uses the `metrics` crate for instrumentation and exports
//! to Prometheus format.

use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use tracing::info;

/// Metric names.
pub mod names {
    pub const CONNECTIONS_TOTAL: &str = "pulse_connections_total";
    pub const CONNECTIONS_ACTIVE: &str = "pulse_connections_active";
    pub const MESSAGES_TOTAL: &str = "pulse_messages_total";
    pub const MESSAGES_BYTES: &str = "pulse_messages_bytes";
    pub const CHANNELS_ACTIVE: &str = "pulse_channels_active";
    pub const SUBSCRIPTIONS_TOTAL: &str = "pulse_subscriptions_total";
    pub const LATENCY_SECONDS: &str = "pulse_latency_seconds";
    pub const ERRORS_TOTAL: &str = "pulse_errors_total";
}

/// Initialize the metrics system.
pub fn init_metrics() {
    // Describe metrics
    metrics::describe_counter!(
        names::CONNECTIONS_TOTAL,
        "Total number of connections since server start"
    );
    metrics::describe_gauge!(
        names::CONNECTIONS_ACTIVE,
        "Current number of active connections"
    );
    metrics::describe_counter!(names::MESSAGES_TOTAL, "Total number of messages processed");
    metrics::describe_counter!(names::MESSAGES_BYTES, "Total bytes of messages processed");
    metrics::describe_gauge!(names::CHANNELS_ACTIVE, "Current number of active channels");
    metrics::describe_counter!(
        names::SUBSCRIPTIONS_TOTAL,
        "Total number of channel subscriptions"
    );
    metrics::describe_histogram!(
        names::LATENCY_SECONDS,
        "Message processing latency in seconds"
    );
    metrics::describe_counter!(names::ERRORS_TOTAL, "Total number of errors");

    info!("Metrics initialized");
}

/// Start the Prometheus metrics server.
///
/// # Errors
///
/// Returns an error if the server cannot be started.
pub fn start_metrics_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;

    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()?;

    info!("Metrics server listening on {}", addr);
    Ok(())
}

/// Record a new connection.
pub fn record_connection() {
    counter!(names::CONNECTIONS_TOTAL).increment(1);
    gauge!(names::CONNECTIONS_ACTIVE).increment(1.0);
}

/// Record a disconnection.
pub fn record_disconnection() {
    gauge!(names::CONNECTIONS_ACTIVE).decrement(1.0);
}

/// Record a message.
pub fn record_message(bytes: usize, direction: &str) {
    counter!(names::MESSAGES_TOTAL, "direction" => direction.to_string()).increment(1);
    counter!(names::MESSAGES_BYTES, "direction" => direction.to_string()).increment(bytes as u64);
}

/// Record message latency.
pub fn record_latency(seconds: f64) {
    histogram!(names::LATENCY_SECONDS).record(seconds);
}

/// Record a subscription.
pub fn record_subscription() {
    counter!(names::SUBSCRIPTIONS_TOTAL).increment(1);
}

/// Update active channel count.
pub fn set_active_channels(count: usize) {
    gauge!(names::CHANNELS_ACTIVE).set(count as f64);
}

/// Record an error.
pub fn record_error(error_type: &str) {
    counter!(names::ERRORS_TOTAL, "type" => error_type.to_string()).increment(1);
}

/// Metrics guard that records disconnection on drop.
pub struct ConnectionMetricsGuard;

impl ConnectionMetricsGuard {
    /// Create a new metrics guard, recording a connection.
    #[must_use]
    pub fn new() -> Self {
        record_connection();
        Self
    }
}

impl Default for ConnectionMetricsGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ConnectionMetricsGuard {
    fn drop(&mut self) {
        record_disconnection();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_guard() {
        // Just test that it doesn't panic
        let _guard = ConnectionMetricsGuard::new();
    }
}
