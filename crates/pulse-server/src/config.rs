//! Server configuration.
//!
//! Configuration can be loaded from:
//! - Environment variables (PULSE_*)
//! - TOML configuration file
//! - Command line arguments (future)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Host to bind to.
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Transport configuration.
    #[serde(default)]
    pub transport: TransportConfig,

    /// Resource limits.
    #[serde(default)]
    pub limits: LimitsConfig,

    /// Heartbeat configuration.
    #[serde(default)]
    pub heartbeat: HeartbeatConfig,

    /// Metrics configuration.
    #[serde(default)]
    pub metrics: MetricsConfig,
}

/// Transport configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Enable WebSocket transport.
    #[serde(default = "default_true")]
    pub websocket: bool,

    /// Enable WebTransport (experimental).
    #[serde(default)]
    pub webtransport: bool,

    /// Path for WebSocket endpoint.
    #[serde(default = "default_ws_path")]
    pub websocket_path: String,
}

/// Resource limits configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// Maximum number of connections.
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Maximum number of channels.
    #[serde(default = "default_max_channels")]
    pub max_channels: usize,

    /// Maximum subscriptions per connection.
    #[serde(default = "default_max_subscriptions")]
    pub max_subscriptions_per_connection: usize,

    /// Maximum message size in bytes.
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,
}

/// Heartbeat configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// Heartbeat interval in milliseconds.
    #[serde(default = "default_heartbeat_interval")]
    pub interval_ms: u64,

    /// Connection timeout in milliseconds.
    #[serde(default = "default_heartbeat_timeout")]
    pub timeout_ms: u64,
}

/// Metrics configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics export.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Metrics port.
    #[serde(default = "default_metrics_port")]
    pub port: u16,
}

// Default value functions
fn default_host() -> String {
    std::env::var("PULSE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn default_port() -> u16 {
    std::env::var("PULSE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080)
}

fn default_true() -> bool {
    true
}

fn default_ws_path() -> String {
    "/ws".to_string()
}

fn default_max_connections() -> usize {
    100_000
}

fn default_max_channels() -> usize {
    10_000
}

fn default_max_subscriptions() -> usize {
    100
}

fn default_max_message_size() -> usize {
    64 * 1024 // 64 KB
}

fn default_heartbeat_interval() -> u64 {
    30_000 // 30 seconds
}

fn default_heartbeat_timeout() -> u64 {
    60_000 // 60 seconds
}

fn default_metrics_port() -> u16 {
    9090
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            transport: TransportConfig::default(),
            limits: LimitsConfig::default(),
            heartbeat: HeartbeatConfig::default(),
            metrics: MetricsConfig::default(),
        }
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            websocket: true,
            webtransport: false,
            websocket_path: default_ws_path(),
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_connections: default_max_connections(),
            max_channels: default_max_channels(),
            max_subscriptions_per_connection: default_max_subscriptions(),
            max_message_size: default_max_message_size(),
        }
    }
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_ms: default_heartbeat_interval(),
            timeout_ms: default_heartbeat_timeout(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: default_metrics_port(),
        }
    }
}

impl Config {
    /// Load configuration from file or defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if the config file exists but cannot be parsed.
    pub fn load() -> Result<Self> {
        // Try to load from default paths
        let config_paths = [
            "pulse.toml",
            "/etc/pulse/pulse.toml",
            "~/.config/pulse/pulse.toml",
        ];

        for path in &config_paths {
            let expanded = shellexpand::tilde(path);
            if Path::new(expanded.as_ref()).exists() {
                return Self::from_file(expanded.as_ref());
            }
        }

        // Fall back to defaults with environment overrides
        Ok(Self::default())
    }

    /// Load configuration from a specific file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Get the socket address to bind to.
    #[must_use]
    pub fn bind_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid host:port")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.port, 8080);
        assert!(config.transport.websocket);
        assert!(!config.transport.webtransport);
    }

    #[test]
    fn test_config_bind_addr() {
        let config = Config::default();
        let addr = config.bind_addr();
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn test_config_from_toml() {
        let toml_str = r#"
            host = "0.0.0.0"
            port = 9000
            
            [limits]
            max_connections = 50000
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 9000);
        assert_eq!(config.limits.max_connections, 50000);
    }
}
