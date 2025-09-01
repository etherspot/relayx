# RelayX Build Instructions

## Prerequisites

### System Requirements
- **Operating System**: Linux, macOS, or Windows
- **Rust**: Version 1.70+ with Cargo
- **Memory**: At least 2GB RAM
- **Storage**: At least 1GB free space

### Install Rust
```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Install RocksDB Dependencies

#### macOS
```bash
brew install rocksdb
```

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install librocksdb-dev build-essential
```

#### CentOS/RHEL
```bash
sudo yum install rocksdb-devel gcc gcc-c++ make
```

#### Windows
```bash
# RocksDB is included in the crate, no additional installation needed
```

## Building the Service

### 1. Clone and Navigate
```bash
cd /path/to/relayx
```

### 2. Build the Service
```bash
# Development build
cargo build

# Release build (recommended for production)
cargo build --release
```

### 3. Verify Build
```bash
# Check if binary was created
ls -la target/debug/relayx
ls -la target/release/relayx

# Test CLI help
./target/release/relayx --help
```

## Running the Service

### Basic Usage
```bash
# Start with default settings
./target/release/relayx

# Start with custom configuration
./target/release/relayx \
  --rpc-host 0.0.0.0 \
  --rpc-port 8545 \
  --db-path /var/lib/relayx \
  --relayers "0x123...,0x456..." \
  --max-concurrent-requests 200 \
  --request-timeout 60
```

### Configuration Options

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `--rpc-host` | RPC server host address | `127.0.0.1` | `0.0.0.0` |
| `--rpc-port` | RPC server port | `8545` | `9000` |
| `--db-path` | Database path for RocksDB | `./relayx_db` | `/var/lib/relayx` |
| `--relayers` | Comma-separated relayer addresses | (empty) | `0x123...,0x456...` |
| `--max-concurrent-requests` | Max concurrent requests | `100` | `200` |
| `--request-timeout` | Request timeout in seconds | `30` | `60` |

### Production Deployment

#### Systemd Service (Linux)
```bash
# Create service file
sudo tee /etc/systemd/system/relayx.service > /dev/null <<EOF
[Unit]
Description=RelayX Relayer Service
After=network.target

[Service]
Type=simple
User=relayx
WorkingDirectory=/opt/relayx
ExecStart=/opt/relayx/relayx --rpc-host 0.0.0.0 --rpc-port 8545 --db-path /var/lib/relayx
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable relayx
sudo systemctl start relayx
```

#### Docker Deployment
```dockerfile
FROM rust:1.70 as builder
WORKDIR /usr/src/relayx
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y libgcc-s1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/relayx/target/release/relayx /usr/local/bin/
EXPOSE 8545
CMD ["relayx", "--rpc-host", "0.0.0.0", "--rpc-port", "8545"]
```

## Testing the Service

### 1. Start the Service
```bash
./target/release/relayx --rpc-host 127.0.0.1 --rpc-port 8545
```

### 2. Test Endpoints

#### Health Check
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "health_check",
    "params": null
  }'
```

#### Submit Request
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "submit_request",
    "params": {
      "from_address": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
      "to_address": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
      "amount": "1000000000000000000",
      "gas_limit": 21000,
      "gas_price": "20000000000",
      "data": "0x",
      "nonce": 0,
      "chain_id": 1
    }
  }'
```

#### Get Request Status
```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "get_request_status",
    "params": "UUID-FROM-PREVIOUS-RESPONSE"
  }'
```

### 3. Use the Test Script
```bash
# Make script executable
chmod +x scripts/test_service.sh

# Run the test
./scripts/test_service.sh
```

## Troubleshooting

### Common Issues

#### Build Errors
```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update
```

#### RocksDB Errors
```bash
# Check if RocksDB is installed
pkg-config --exists rocksdb

# Reinstall RocksDB
# macOS: brew reinstall rocksdb
# Ubuntu: sudo apt-get install --reinstall librocksdb-dev
```

#### Port Already in Use
```bash
# Check what's using the port
lsof -i :8545

# Kill the process or use a different port
./target/release/relayx --rpc-port 8546
```

#### Permission Denied
```bash
# Check file permissions
ls -la target/release/relayx

# Make executable
chmod +x target/release/relayx
```

### Logs and Debugging
```bash
# Enable verbose logging
RUST_LOG=debug ./target/release/relayx

# Check database files
ls -la relayx_db/

# Monitor service
tail -f /var/log/relayx.log  # if using systemd
```

## Performance Tuning

### Database Optimization
```bash
# Adjust RocksDB settings in src/storage.rs
opts.set_max_open_files(10000);        # Increase for high I/O
opts.set_bytes_per_sync(1024 * 1024);  # Adjust sync frequency
```

### Network Tuning
```bash
# Increase file descriptor limits
ulimit -n 65536

# Adjust TCP settings
echo 'net.core.somaxconn = 65535' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

## Security Considerations

### Firewall Configuration
```bash
# Allow only specific IPs
sudo ufw allow from 192.168.1.0/24 to any port 8545

# Or restrict to localhost only
./target/release/relayx --rpc-host 127.0.0.1
```

### Database Security
```bash
# Use secure database path
./target/release/relayx --db-path /var/lib/relayx

# Set proper permissions
sudo chown -R relayx:relayx /var/lib/relayx
sudo chmod 700 /var/lib/relayx
```

## Monitoring and Maintenance

### Health Checks
```bash
# Regular health monitoring
watch -n 30 'curl -s http://localhost:8545 -X POST -H "Content-Type: application/json" -d '"'"'{"jsonrpc":"2.0","id":1,"method":"health_check","params":null}'"'"' | jq'
```

### Database Maintenance
```bash
# Check database size
du -sh relayx_db/

# Backup database
cp -r relayx_db/ relayx_db_backup_$(date +%Y%m%d_%H%M%S)/
```

### Log Rotation
```bash
# Configure logrotate for systemd logs
sudo tee /etc/logrotate.d/relayx > /dev/null <<EOF
/var/log/relayx.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 644 relayx relayx
}
EOF
```

