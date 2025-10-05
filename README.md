A Rust implementation of the [Generic Relayer Architecture for Smart Accounts EIP](https://hackmd.io/T4TkZYFQQnCupiuW231DYw) that enables gasless and sponsored transactions for smart accounts through a standardized JSON-RPC interface.

## Overview

This relayer service provides a standardized off-chain architecture that enables smart accounts to execute gasless transactions and token-fee payments. The service implements a simplified, high-performance JSON-RPC server that focuses on core relayer functionality without blockchain dependencies, making it ideal for testing, development, and integration scenarios.

### Key Features

- **Gasless Transactions**: Support for ERC-20 token-based transaction fee payments
- **Transaction Relaying**: Submit signed transactions through a standardized relayer interface
- **Exchange Rate Simulation**: Get token-to-gas conversion rates with stub responses for fast testing
- **Transaction Status Tracking**: Monitor the lifecycle of submitted transactions with persistent storage
- **Multi-token Support**: Configurable support for multiple ERC-20 tokens across different networks
- **Capability Discovery**: Automatically discover supported payment methods and tokens from configuration
- **Health Monitoring**: Built-in health check and metrics endpoints for monitoring
- **Simplified Architecture**: No blockchain dependencies for fast, reliable operation

## Architecture

The service implements a simplified, high-performance JSON-RPC server with the following components:

```
┌─────────────┐    ┌──────────────┐    ┌─────────────────┐
│   dApp/     │───▶│    Relayer   │───▶│   Smart         │
│   Wallet    │    │   RPC Server │    │   Account       │
│             │    │              │    │   Integration   │
└─────────────┘    └──────────────┘    └─────────────────┘
                          │
                          ▼
                   ┌─────────────────┐
                   │  Storage Layer  │
                   │  ┌─────────────┐│
                   │  │  RocksDB    ││
                   │  │  (Requests, ││
                   │  │   Status,   ││
                   │  │   Metrics)  ││
                   │  └─────────────┘│
                   └─────────────────┘
                          │
                          ▼
                   ┌─────────────────┐
                   │  Configuration  │
                   │  (JSON Config,  │
                   │   Environment   │
                   │   Variables)    │
                   └─────────────────┘
```

### Simplified Design Principles

- **No Blockchain Dependencies**: Eliminates complex blockchain RPC calls and provider management
- **Fast Response Times**: Stub responses provide immediate feedback without network latency
- **Reliable Operation**: No external service dependencies for core functionality
- **Easy Testing**: Predictable responses make integration testing straightforward
- **Development Friendly**: Ideal for development, testing, and integration scenarios

## Build and Run

### Requirements
- Rust (stable) and Cargo
- RocksDB system libraries (for persistent storage)

**Simplified Dependencies:**
- No blockchain RPC libraries required
- No complex web3 dependencies
- Minimal system requirements

### Install system dependencies
- macOS:
```bash
brew install rocksdb
```
- Ubuntu/Debian:
```bash
sudo apt-get update && sudo apt-get install -y librocksdb-dev
```

### Build
```bash
cargo build --release
```

### Run (CLI)
```bash
./target/release/relayx \
  --http-address 0.0.0.0 \
  --http-port 4937 \
  --http-cors "*" \
  --db-path ./relayx_db
```

You can also configure via a JSON file (and override with CLI/env):
- Set RELAYX_CONFIG=/path/to/config.json
- File supports fields: `http_address`, `http_port`, `http_cors`, `feeCollector`, `rpcs`, and `chainlink` feeds.

### Run (Docker)
```bash
docker build -t relayx:latest .
# minimal
docker run --rm -p 4937:4937 relayx:latest
# with a config file
docker run --rm -p 4937:4937 -e RELAYX_CONFIG=/app/config.json \
  -v /abs/path/config.json:/app/config.json:ro relayx:latest
```

## Configuration

### CLI Flags and Environment Variables

**Basic Configuration:**
- `--http-address` (`HTTP_ADDRESS`): Server bind address (default: 127.0.0.1)
- `--http-port` (`HTTP_PORT`): Server port (default: 4937)
- `--http-cors` (`HTTP_CORS`): CORS origins (default: "*")
- `--db-path`: RocksDB storage path (default: ./relayx_db)
- `--config` (`RELAYX_CONFIG`): Path to JSON configuration file

**JSON Configuration File:**

The relayer supports streamlined configuration via JSON file:

```json
{
  "http_address": "127.0.0.1",
  "http_port": 4937,
  "http_cors": "*",
  "feeCollector": "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
  "defaultToken": "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
  "chainlink": {
    "tokenUsd": {
      "1": {
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48": "0x8fffffd4afb6115b954bd326cbe7b4ba576818f6",
        "0x036CbD53842c5426634e7929541eC2318f3dCF7e": "0x8fffffd4afb6115b954bd326cbe7b4ba576818f6",
        "0xdAC17F958D2ee523a2206206994597C13D831ec7": "0x3E7d1eAB13ad8aB6F6c6b9b8B7c6e8f7a9c8b7a6"
      },
      "137": {
        "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174": "0xAB594600376Ec9fD91F8e885dADF0CE036862dE0",
        "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063": "0xAB594600376Ec9fD91F8e885dADF0CE036862dE0"
      }
    }
  }
}
```

**Note**: The `rpcs` and `chainlink.nativeUsd` configurations are no longer required as the relayer uses stub responses for exchange rates instead of making blockchain calls.

### Token Discovery

The relayer automatically discovers supported ERC20 tokens from the `chainlink.tokenUsd` configuration:

- **Multi-chain Support**: Tokens are extracted from all configured chains
- **Automatic Deduplication**: Duplicate tokens across chains are automatically removed
- **Sorted Results**: Tokens are returned in sorted order for consistency
- **Fallback Support**: If no tokens are configured, falls back to `defaultToken` or environment variable

### Environment Variables

**Token Configuration:**
- `RELAYX_DEFAULT_TOKEN`: Default ERC20 token address for fallback
- `RELAYX_FEE_COLLECTOR`: Address to receive relayer fees

**Note**: Blockchain RPC and Chainlink feed configurations are no longer required as the relayer uses simplified stub responses for exchange rates and capabilities discovery.

## Supported JSON-RPC Methods

### Core Relayer Methods

1. **`relayer_getCapabilities`** - Discover supported payment methods and tokens
2. **`relayer_getExchangeRate`** - Get token exchange rates for gas payment
3. **`relayer_getQuote`** - Simulate transactions and get gas estimates  
4. **`relayer_sendTransaction`** - Submit signed transactions for relay
5. **`relayer_getStatus`** - Check status of submitted transactions
6. **`health_check`** - Service health and metrics

## Usage Examples

### 1. Get Relayer Capabilities

Discover all supported payment methods and tokens:

**Request:**
```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getCapabilities",
    "params": [],
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "capabilities": {
      "payment": [
        {
          "type": "erc20",
          "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        },
        {
          "type": "erc20", 
          "token": "0x036CbD53842c5426634e7929541eC2318f3dCF7e"
        },
        {
          "type": "erc20",
          "token": "0xdAC17F958D2ee523a2206206994597C13D831ec7"
        },
        {
          "type": "native",
          "token": "0x0000000000000000000000000000000000000000"
        },
        {
          "type": "sponsored"
        }
      ]
    }
  },
  "id": 1
}
```

### 2. Get Exchange Rate

Get current token-to-gas conversion rates:

**Request:**
```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getExchangeRate", 
    "params": [{
      "chainId": "1",
      "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    }],
    "id": 2
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [{
    "quote": {
      "rate": 0.0032,
      "token": {
        "decimals": 6,
        "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "symbol": "USDC",
        "name": "USD Coin"
      }
    },
    "gasPrice": "0x4a817c800",
    "feeCollector": "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
    "expiry": 1755917874
  }],
  "id": 2
}
```

### 3. Get Transaction Quote

Simulate a transaction to get gas estimates and required fees:

**Request:**
```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getQuote",
    "params": [{
      "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
      "data": "0xa9059cbb000000000000000000000000742d35cc6c3c3f4b4c1b3cd6c0d1b6c2b3d4e5f60000000000000000000000000000000000000000000000000de0b6b3a7640000",
      "chainId": "1",
      "capabilities": {
        "payment": {
          "type": "erc20",
          "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        }
      }
    }],
    "id": 3
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "quote": {
      "fee": 21000,
      "rate": 0.0032,
      "token": {
        "decimals": 6,
        "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "symbol": "USDC",
        "name": "USD Coin"
      }
    },
    "relayerCalls": [
      {
        "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
        "data": "0xa9059cbb000000000000000000000000742d35cc6c3c3f4b4c1b3cd6c0d1b6c2b3d4e5f60000000000000000000000000000000000000000000000000de0b6b3a7640000"
      }
    ],
    "feeCollector": "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
    "revertReason": "0x"
  },
  "id": 3
}
```

### 4. Submit Transaction

Submit a signed transaction for relay:

**Request:**
```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0", 
    "method": "relayer_sendTransaction",
    "params": [{
      "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
      "data": "0xa9059cbb000000000000000000000000742d35cc6c3c3f4b4c1b3cd6c0d1b6c2b3d4e5f60000000000000000000000000000000000000000000000000de0b6b3a7640000",
      "chainId": "1",
      "authorizationList": "0x",
      "capabilities": {
        "payment": {
          "type": "erc20", 
          "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
        }
      }
    }],
    "id": 4
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "chainId": "1",
      "id": "0x00000000000000000000000000000000000000000000000000000000000000000e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331"
    }
  ],
  "id": 4
}
```

### 5. Check Transaction Status

Query the status of submitted transactions:

**Request:**
```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getStatus", 
    "params": {
      "ids": ["0x00000000000000000000000000000000000000000000000000000000000000000e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331"]
    },
    "id": 5
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "version": "2.0.0",
      "id": "0x00000000000000000000000000000000000000000000000000000000000000000e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331",
      "status": 200,
      "receipts": [
        {
          "logs": [
            {
              "address": "0xa922b54716264130634d6ff183747a8ead91a40b",
              "topics": ["0x5a2a90727cc9d000dd060b1132a5c977c9702bb3a52afe360c9c22f0e9451a68"],
              "data": "0xabcd"
            }
          ],
          "status": "0x1",
          "blockHash": "0xf19bbafd9fd0124ec110b848e8de4ab4f62bf60c189524e54213285e7f540d4a",
          "blockNumber": "0xabcd",
          "gasUsed": "0xdef",
          "transactionHash": "0x9b7bb827c2e5e3c1a0a44dc53e573aa0b3af3bd1f9f5ed03071b100bb039eaff",
          "chainId": "1"
        }
      ],
      "resubmissions": [
        {
          "status": 200,
          "transactionHash": "0x9b7bb827c2e5e3c1a0a44dc53e573aa0b3af3bd1f9f5ed03071b100bb039eaf3",
          "chainId": "1"
        }
      ],
      "offchainFailure": [
        {
          "message": "insufficient fee provided"
        }
      ],
      "onchainFailure": [
        {
          "transactionHash": "0x9b7bb827c2e5e3c1a0a44dc53e573aa0b3af3bd1f9f5ed03071b100bb039eaf2",
          "chainId": "1",
          "message": "execution reverted: transfer failed",
          "data": "0x08c379a000000000000000000000000000000000000000000000000000000000"
        }
      ]
    }
  ],
  "id": 5
}
```

### 6. Health Check

Monitor service health and metrics:

**Request:**
```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "health_check",
    "params": [],
    "id": 6
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "healthy",
    "timestamp": "2025-01-27T10:30:00Z",
    "uptime_seconds": 86400,
    "total_requests": 1250,
    "pending_requests": 5,
    "completed_requests": 1200,
    "failed_requests": 45
  },
  "id": 6
}
```


## Development

### Project Structure

```
src/
git s├── main.rs              # Application entry point and CLI argument parsing
├── config.rs            # Configuration management with JSON and environment support
├── storage.rs           # RocksDB-based data persistence layer
├── types.rs            # JSON-RPC request/response types and data structures
├── rpc.rs              # Main RPC server implementation with endpoint handlers
└── lib.rs              # Library exports and module definitions

examples/
├── test_client.rs       # Example client for testing basic functionality
└── test_capabilities.rs # Example client for testing capabilities endpoint

config.json.default      # Default configuration template
Dockerfile              # Docker container configuration
Cargo.toml              # Rust project dependencies and features
```

### Key Components

#### RpcServer
The main server struct that:
- Handles JSON-RPC method routing for all endpoints
- Processes business logic for each endpoint with stub responses
- Supports multiple payment methods and token discovery
- No blockchain dependencies for simplified operation

#### Configuration Management
Streamlined configuration system:
- **CLI Arguments**: Command-line flags with environment variable fallbacks
- **JSON Configuration**: Structured configuration file support for token discovery
- **Token Discovery**: Automatic extraction of supported tokens from configuration
- **Simplified Setup**: No blockchain RPC or complex feed configurations required

#### Storage Layer
RocksDB-based persistent storage for:
- Transaction requests and status tracking
- Request metrics and performance counts  
- System uptime and health monitoring
- Request lifecycle management

#### Exchange Rate Management
Simplified exchange rate handling:
- Returns predictable stub exchange rates for testing and development
- Supports both native and ERC20 token rate calculations
- Configurable fee collector addresses
- Fast response times without blockchain network dependencies

#### Capability Discovery
Automatic capability detection:
- **Token Discovery**: Extracts supported ERC20 tokens from configuration
- **Payment Methods**: Supports native, ERC20, and sponsored payments
- **Multi-chain Tokens**: Aggregates tokens across all configured chains
- **Fallback Support**: Graceful degradation when configuration is incomplete

## Development

### Prerequisites

**System Requirements:**
- **Operating System**: Linux, macOS, or Windows
- **Rust**: Version 1.70+ with Cargo
- **Memory**: At least 2GB RAM
- **Storage**: At least 1GB free space

**Install Rust:**
```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

**Install RocksDB Dependencies:**
- **macOS**: `brew install rocksdb`
- **Ubuntu/Debian**: `sudo apt-get install librocksdb-dev build-essential`
- **CentOS/RHEL**: `sudo yum install rocksdb-devel gcc gcc-c++ make`
- **Windows**: RocksDB is included in the crate

### Building and Testing

**Build Commands:**
```bash
# Check code without building
cargo check

# Build in debug mode
cargo build

# Build optimized release
cargo build --release

# Run linting
make lint  # (fmt, clippy, cargo-sort, udeps, audit)
```

**Running the Server:**
```bash
# Start the relayer server
cargo run --release

# Start with custom configuration
cargo run --release -- --config config.json.default

# Start with environment variables
RELAYX_CONFIG=config.json.default cargo run --release
```

### Testing Examples

**Basic Functionality Test:**
```bash
# Run the basic test client
cargo run --example test_client
```

**Capabilities Endpoint Test:**
```bash
# Run the capabilities test client
cargo run --example test_capabilities
```

**Manual Testing with curl:**

Test the capabilities endpoint:
```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getCapabilities",
    "params": [],
    "id": 1
  }'
```

Test the health check:
```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "health_check",
    "params": [],
    "id": 1
  }'
```

### Development Features

**Simplified Architecture Benefits:**
- **Fast Build Times**: No heavy blockchain dependencies to compile
- **Reliable Testing**: Predictable stub responses for consistent testing
- **Easy Integration**: Simple JSON-RPC interface without blockchain complexity
- **Quick Setup**: Minimal configuration required to get started

**Configuration Testing:**
- Use `config.json.default` as a starting point for your configuration
- Test different token configurations by modifying the `chainlink.tokenUsd` section
- Verify token discovery by checking the `relayer_getCapabilities` response
- All endpoints return immediate responses without network delays

**Performance Optimization:**
- Configuration parsing is cached for efficiency
- No database queries for capabilities endpoint (stateless)
- No blockchain RPC calls - uses stub responses for fast testing
- Automatic token deduplication and sorting
- Minimal memory footprint and fast startup times

## CI/CD and Deployment

### GitHub Workflows

The project uses a single, unified CI/CD workflow optimized for all scenarios:

#### **Unified CI Pipeline** (`.github/workflows/fast-ci.yml`)
- **Comprehensive Testing**: Format, clippy, cargo-sort, test, build
- **Advanced Caching**: Multi-level cache with cargo registry and build caching
- **Parallel Execution**: Jobs run simultaneously for speed
- **Docker Build**: Multi-arch images published to GHCR on main branch
- **Dual Triggers**: Runs on both PRs and main branch pushes
- **Performance**: ~3-4 minutes with warm cache, ~1-2 minutes for PR checks


### Performance Optimizations

**CI/CD Speed Improvements:**
- **Format Check**: ~30 seconds → ~5 seconds (6x faster)
- **Clippy**: ~2-3 minutes → ~30-45 seconds (4x faster)
- **Full CI**: ~8-10 minutes → ~3-4 minutes (2.5x faster)
- **Docker Build**: ~5-7 minutes → ~2-3 minutes (2.5x faster)

**Caching Strategy:**
- **Registry Cache**: `~/.cargo/registry` - Cached dependencies
- **Index Cache**: `~/.cargo/git` - Git-based dependencies
- **Build Cache**: `target/` - Compiled artifacts
- **Bin Cache**: `~/.cargo/bin` - Cached tools like cargo-sort

### Production Deployment

#### **Systemd Service (Linux)**
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
ExecStart=/opt/relayx/relayx --http-address 0.0.0.0 --http-port 4937 --db-path /var/lib/relayx
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

#### **Docker Deployment**
```bash
# Build image
docker build -t relayx:latest .

# Run with minimal config
docker run --rm -p 4937:4937 relayx:latest

# Run with config file
docker run --rm -p 4937:4937 -e RELAYX_CONFIG=/app/config.json \
  -v /abs/path/config.json:/app/config.json:ro relayx:latest
```

#### **Dockerfile Optimizations**
```dockerfile
FROM rust:1.70 as builder
WORKDIR /usr/src/relayx
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y libgcc-s1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/relayx/target/release/relayx /usr/local/bin/
EXPOSE 4937
CMD ["relayx", "--http-address", "0.0.0.0", "--http-port", "4937"]
```

### Security and Monitoring

#### **Firewall Configuration**
```bash
# Allow only specific IPs
sudo ufw allow from 192.168.1.0/24 to any port 4937

# Or restrict to localhost only
./target/release/relayx --http-address 127.0.0.1
```

#### **Database Security**
```bash
# Use secure database path
./target/release/relayx --db-path /var/lib/relayx

# Set proper permissions
sudo chown -R relayx:relayx /var/lib/relayx
sudo chmod 700 /var/lib/relayx
```

#### **Health Monitoring**
```bash
# Regular health monitoring
watch -n 30 'curl -s http://localhost:4937 -X POST -H "Content-Type: application/json" -d '"'"'{"jsonrpc":"2.0","id":1,"method":"health_check","params":null}'"'"' | jq'
```

### Troubleshooting

#### **Common Issues**
```bash
# Build errors - clean and rebuild
cargo clean && cargo build

# RocksDB errors - check installation
pkg-config --exists rocksdb

# Port already in use
lsof -i :4937

# Permission denied
chmod +x target/release/relayx
```

#### **Performance Tuning**
```bash
# Increase file descriptor limits
ulimit -n 65536

# Adjust TCP settings
echo 'net.core.somaxconn = 65535' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

## License
MIT
