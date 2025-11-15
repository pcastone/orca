# ACO TUI Deployment Guide

## Quick Start

### Prerequisites

- Rust 1.75+ (for building from source)
- Linux/macOS/Windows terminal with UTF-8 support
- Orchestrator server running (or access to one)

### Build & Install

```bash
# Clone repository
git clone https://github.com/your-org/orca
cd orca/src/crates/aco

# Build release binary
cargo build --release

# Install globally (Linux/macOS)
sudo cp target/release/aco /usr/local/bin/

# Or install for current user
mkdir -p ~/.local/bin
cp target/release/aco ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"
```

### Verify Installation

```bash
aco --version
aco --help
```

## Deployment Scenarios

### 1. Local Development

**Use Case**: Development and testing on local machine

```bash
# Start orchestrator
cd src/crates/orchestrator
cargo run --release

# In another terminal, start TUI
cd src/crates/aco
cargo run --release -- tui
```

**Configuration**:
- Server: `http://localhost:50051` (default)
- Auth: None (development mode)
- Workspace: `./.aco/workspace`

### 2. Remote Server (SSH)

**Use Case**: Connect to remote orchestrator from local terminal

```bash
# SSH to remote server
ssh user@remote-server

# Start TUI with remote orchestrator
aco --server http://orchestrator.example.com:50051 tui

# Or with environment variable
export ACO_SERVER=http://orchestrator.example.com:50051
aco tui
```

**Configuration**:
```bash
# Save in ~/.bashrc or ~/.zshrc
export ACO_SERVER=http://orchestrator.example.com:50051
export ACO_CONNECT=user:password
```

### 3. Docker Container

**Use Case**: Containerized deployment with orchestrator

#### Dockerfile

```dockerfile
# Multi-stage build for minimal image size
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

# Build only aco crate
RUN cargo build --release -p aco

# Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/aco /usr/local/bin/aco

# Create workspace directory
RUN mkdir -p /workspace && chmod 755 /workspace

# Set working directory
WORKDIR /workspace

# Default command
ENTRYPOINT ["aco"]
CMD ["tui"]
```

#### Build & Run

```bash
# Build image
docker build -t aco-tui:latest -f Dockerfile .

# Run with server connection
docker run -it --rm \
  -e ACO_SERVER=http://orchestrator:50051 \
  -e RUST_LOG=info \
  aco-tui:latest

# Run with authentication
docker run -it --rm \
  -e ACO_SERVER=http://orchestrator:50051 \
  -e ACO_CONNECT=user:password \
  aco-tui:latest

# Run with custom workspace volume
docker run -it --rm \
  -v $(pwd)/workspace:/workspace \
  -e ACO_SERVER=http://orchestrator:50051 \
  aco-tui:latest
```

### 4. Docker Compose

**Use Case**: Deploy TUI with orchestrator as service

#### docker-compose.yml

```yaml
version: '3.8'

services:
  orchestrator:
    image: aco-orchestrator:latest
    ports:
      - "50051:50051"
    environment:
      - RUST_LOG=info
      - DATABASE_URL=sqlite:///data/orca.db
    volumes:
      - orchestrator-data:/data

  tui:
    image: aco-tui:latest
    depends_on:
      - orchestrator
    environment:
      - ACO_SERVER=http://orchestrator:50051
      - RUST_LOG=info
    stdin_open: true
    tty: true
    volumes:
      - tui-workspace:/workspace

volumes:
  orchestrator-data:
  tui-workspace:
```

#### Run

```bash
# Start services
docker-compose up

# Run TUI in separate terminal
docker-compose run --rm tui

# Stop services
docker-compose down
```

### 5. Kubernetes

**Use Case**: Production deployment in Kubernetes cluster

#### deployment.yaml

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: aco-config
data:
  server-url: "http://orchestrator-service:50051"

---

apiVersion: apps/v1
kind: Deployment
metadata:
  name: aco-tui
  labels:
    app: aco-tui
spec:
  replicas: 1
  selector:
    matchLabels:
      app: aco-tui
  template:
    metadata:
      labels:
        app: aco-tui
    spec:
      containers:
      - name: tui
        image: aco-tui:latest
        env:
        - name: ACO_SERVER
          valueFrom:
            configMapKeyRef:
              name: aco-config
              key: server-url
        - name: RUST_LOG
          value: "info"
        stdin: true
        tty: true
        resources:
          requests:
            memory: "64Mi"
            cpu: "100m"
          limits:
            memory: "128Mi"
            cpu: "200m"
```

#### Deploy

```bash
# Apply configuration
kubectl apply -f deployment.yaml

# Access TUI
kubectl exec -it deployment/aco-tui -- aco tui

# Or port-forward orchestrator and run locally
kubectl port-forward service/orchestrator-service 50051:50051 &
aco --server http://localhost:50051 tui
```

### 6. Systemd Service (Background)

**Use Case**: Run TUI as background service with tmux/screen

#### /etc/systemd/system/aco-tui.service

```ini
[Unit]
Description=ACO TUI Service
After=network.target orchestrator.service
Requires=orchestrator.service

[Service]
Type=forking
User=aco
Group=aco
Environment="ACO_SERVER=http://localhost:50051"
Environment="RUST_LOG=info"

# Start TUI in tmux session
ExecStart=/usr/bin/tmux new-session -d -s aco-tui '/usr/local/bin/aco tui'

# Attach to session
ExecStartPost=/bin/sleep 1

# Stop tmux session
ExecStop=/usr/bin/tmux kill-session -t aco-tui

Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

#### Setup

```bash
# Create user
sudo useradd -r -s /bin/false aco

# Install service
sudo cp aco-tui.service /etc/systemd/system/
sudo systemctl daemon-reload

# Enable and start
sudo systemctl enable aco-tui
sudo systemctl start aco-tui

# Attach to TUI
sudo -u aco tmux attach -t aco-tui

# Check status
sudo systemctl status aco-tui
```

## Production Considerations

### Security

#### Authentication

```bash
# Use environment variable (recommended)
export ACO_CONNECT=user:$(cat ~/.aco/password)
aco tui

# Or use secret management
export ACO_CONNECT=$(kubectl get secret aco-credentials -o jsonpath='{.data.auth}' | base64 -d)
aco tui
```

#### Network Security

```bash
# Use TLS for production
export ACO_SERVER=https://orchestrator.example.com:50051
aco tui

# Or use SSH tunnel
ssh -L 50051:localhost:50051 user@remote-server &
aco --server http://localhost:50051 tui
```

### Monitoring

#### Health Checks

```bash
# Check if TUI process is running
pgrep -f "aco tui"

# Check TUI logs
journalctl -u aco-tui -f

# Monitor resource usage
ps aux | grep "aco tui"
```

#### Logging

```bash
# Enable debug logging
RUST_LOG=debug aco tui

# Log to file
aco tui 2>&1 | tee aco-tui.log

# Structured logging with timestamps
RUST_LOG=info aco tui 2>&1 | while read line; do
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] $line"
done
```

### Performance Tuning

#### Resource Limits

```bash
# Limit memory usage (Linux)
ulimit -v 131072  # 128MB
aco tui

# Limit CPU (systemd)
# Add to service file:
CPUQuota=50%
```

#### Terminal Optimization

```bash
# Use fast terminal emulator
# Recommended: Alacritty, Kitty, WezTerm

# Disable mouse capture if not needed
# (modify code to skip EnableMouseCapture)

# Increase refresh interval
# (modify code to change auto-refresh from 10s to 30s)
```

## Backup & Recovery

### Workspace Backup

```bash
# Backup workspace
tar -czf aco-workspace-$(date +%Y%m%d).tar.gz ~/.aco/workspace

# Restore workspace
tar -xzf aco-workspace-20240101.tar.gz -C ~/
```

### Configuration Backup

```bash
# Export configuration
env | grep ACO_ > aco-config.env

# Import configuration
source aco-config.env
```

## Upgrades

### In-Place Upgrade

```bash
# Stop TUI
sudo systemctl stop aco-tui

# Backup current binary
sudo cp /usr/local/bin/aco /usr/local/bin/aco.bak

# Build new version
git pull
cargo build --release -p aco

# Install new binary
sudo cp target/release/aco /usr/local/bin/

# Start TUI
sudo systemctl start aco-tui

# Verify version
aco --version
```

### Rollback

```bash
# Stop TUI
sudo systemctl stop aco-tui

# Restore backup
sudo cp /usr/local/bin/aco.bak /usr/local/bin/aco

# Start TUI
sudo systemctl start aco-tui
```

## Troubleshooting

### Common Issues

#### 1. Connection Refused

```bash
# Check orchestrator is running
curl -v http://localhost:50051

# Check firewall
sudo ufw status
sudo ufw allow 50051/tcp

# Check network
ping orchestrator.example.com
```

#### 2. Permission Denied

```bash
# Fix binary permissions
sudo chmod +x /usr/local/bin/aco

# Fix workspace permissions
chmod -R 755 ~/.aco/workspace
```

#### 3. Display Issues

```bash
# Check terminal capabilities
echo $TERM
infocmp

# Update locale
sudo dpkg-reconfigure locales
export LANG=en_US.UTF-8

# Try different TERM
export TERM=xterm-256color
aco tui
```

### Debug Mode

```bash
# Run with full debug output
RUST_LOG=debug RUST_BACKTRACE=1 aco tui 2>&1 | tee debug.log

# Run with strace (Linux)
strace -o tui-trace.log aco tui

# Check system calls
grep -E "^(open|connect|read|write)" tui-trace.log
```

## Support

- **GitHub Issues**: https://github.com/your-org/orca/issues
- **Documentation**: https://github.com/your-org/orca/docs
- **Slack**: #aco-support

## Checklist

### Pre-Deployment

- [ ] Rust toolchain installed
- [ ] Binary built and tested
- [ ] Orchestrator server accessible
- [ ] Authentication configured
- [ ] Network connectivity verified
- [ ] Terminal capabilities checked

### Post-Deployment

- [ ] TUI starts successfully
- [ ] Tasks/workflows visible
- [ ] Execution streaming works
- [ ] Keyboard shortcuts functional
- [ ] Auto-refresh working
- [ ] Logs reviewed for errors

### Production

- [ ] TLS/SSL configured
- [ ] Authentication secured
- [ ] Monitoring enabled
- [ ] Backups configured
- [ ] Resource limits set
- [ ] Documentation updated
