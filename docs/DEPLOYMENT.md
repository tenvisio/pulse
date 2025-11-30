# Deployment Guide

This guide covers deploying Pulse in production environments.

## Quick Start

### Binary

```bash
# Download latest release
curl -L https://github.com/tenvisio/pulse/releases/latest/download/pulse-linux-amd64 -o pulse
chmod +x pulse

# Run with defaults
./pulse

# Run with config file
./pulse --config /etc/pulse/pulse.toml
```

### Docker (Coming Soon)

> **Note**: Docker images are not yet available. Track progress in the [roadmap](../README.md#roadmap).
>
> For now, use the binary installation or build from source.

## Configuration

### Configuration File

Create `/etc/pulse/pulse.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080

[transport]
websocket = true
webtransport = false

[limits]
max_connections = 100000
max_channels = 10000
max_subscriptions_per_connection = 100
max_message_size = 65536  # 64 KB

[heartbeat]
interval_ms = 30000
timeout_ms = 60000

[logging]
level = "info"
format = "json"

[metrics]
enabled = true
port = 9090
```

### Environment Variables

All config options can be set via environment:

```bash
export PULSE_SERVER_HOST=0.0.0.0
export PULSE_SERVER_PORT=8080
export PULSE_LIMITS_MAX_CONNECTIONS=100000
export PULSE_LOGGING_LEVEL=info
```

## Reverse Proxy

### Nginx

```nginx
upstream pulse {
    server 127.0.0.1:8080;
    keepalive 64;
}

server {
    listen 443 ssl http2;
    server_name pulse.example.com;

    ssl_certificate /etc/ssl/pulse.crt;
    ssl_certificate_key /etc/ssl/pulse.key;

    location / {
        proxy_pass http://pulse;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 3600s;  # Long timeout for WebSocket
    }
}
```

### Caddy

```caddyfile
pulse.example.com {
    reverse_proxy localhost:8080
}
```

## Systemd Service

Create `/etc/systemd/system/pulse.service`:

```ini
[Unit]
Description=Pulse Realtime Server
After=network.target

[Service]
Type=simple
User=pulse
Group=pulse
ExecStart=/usr/local/bin/pulse --config /etc/pulse/pulse.toml
Restart=on-failure
RestartSec=5

# Security hardening
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes

# Resource limits
LimitNOFILE=1000000

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable pulse
sudo systemctl start pulse
```

## System Tuning

### File Descriptors

Each connection uses a file descriptor. Increase limits:

```bash
# /etc/security/limits.conf
pulse soft nofile 1000000
pulse hard nofile 1000000

# Or with systemd (in service file)
LimitNOFILE=1000000
```

### TCP Tuning

```bash
# /etc/sysctl.conf

# Increase socket buffer sizes
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216

# Increase connection backlog
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535

# Enable TCP keepalive
net.ipv4.tcp_keepalive_time = 600
net.ipv4.tcp_keepalive_intvl = 60
net.ipv4.tcp_keepalive_probes = 3

# Apply
sudo sysctl -p
```

## Monitoring

### Prometheus

Pulse exports metrics on the configured metrics port:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'pulse'
    static_configs:
      - targets: ['localhost:9090']
```

### Grafana Dashboard

Import the provided dashboard from `examples/grafana-dashboard.json`.

### Health Check

```bash
curl http://localhost:8080/health
# {"status": "ok"}
```

## High Availability

### Load Balancing

For multiple Pulse instances:

```nginx
upstream pulse {
    least_conn;
    server pulse1:8080;
    server pulse2:8080;
    server pulse3:8080;
}
```

### Sticky Sessions

WebSocket connections require sticky sessions:

```nginx
upstream pulse {
    ip_hash;
    server pulse1:8080;
    server pulse2:8080;
}
```

## Security Checklist

- [ ] TLS enabled (via reverse proxy)
- [ ] Authentication tokens configured
- [ ] Rate limiting enabled
- [ ] Firewall rules configured
- [ ] File descriptor limits increased
- [ ] Logging to persistent storage
- [ ] Metrics collection enabled
- [ ] Health checks configured

## Troubleshooting

### Connection Refused

```bash
# Check if service is running
systemctl status pulse

# Check if port is listening
ss -tlnp | grep 8080
```

### Too Many Open Files

```bash
# Check current limit
ulimit -n

# Check process limit
cat /proc/$(pgrep pulse)/limits
```

### High Memory Usage

```bash
# Check memory
ps aux | grep pulse

# Enable memory profiling
PULSE_PROFILE=memory ./pulse
```

### Slow Performance

```bash
# Check CPU usage
top -p $(pgrep pulse)

# Check for connection issues
ss -s
```





