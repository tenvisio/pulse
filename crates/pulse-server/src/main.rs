//! # Pulse Server
//!
//! High-performance realtime communication server.
//!
//! ## Usage
//!
//! ```bash
//! # Run with default settings
//! pulse
//!
//! # Run with custom config
//! pulse --config /path/to/pulse.toml
//!
//! # Run with environment variables
//! PULSE_PORT=8080 PULSE_HOST=0.0.0.0 pulse
//! ```

mod config;
mod handlers;
mod metrics;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "pulse=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::load()?;

    tracing::info!("Starting Pulse server on {}:{}", config.host, config.port);

    // Initialize metrics
    metrics::init_metrics();

    // Start the server
    handlers::run_server(config).await?;

    Ok(())
}
