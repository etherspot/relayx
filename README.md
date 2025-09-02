# RelayX - Modular Relayer Service

A modular Rust-based relayer service that provides JSON-RPC endpoints for managing blockchain transaction relay requests with persistent storage using RocksDB.

## Features

- **JSON-RPC Service**: HTTP-based JSON-RPC server with three endpoints
- **Persistent Storage**: RocksDB-based key-value storage for all requests and responses
- **CLI Configuration**: Command-line interface for easy configuration
- **Modular Architecture**: Clean separation of concerns with dedicated modules
- **Async Runtime**: Built on Tokio for high-performance async operations
- **Comprehensive Logging**: Structured logging with tracing

## Architecture

The service is organized into the following modules:

- **`config`**: CLI argument parsing and configuration management
- **`rpc`**: JSON-RPC server implementation with three endpoints
- **`storage`**: RocksDB-based persistent storage layer
- **`types`**: Data structures and type definitions

## JSON-RPC Endpoints

### 1. `submit_request`

Submits a new relayer request for processing.

**Parameters:**
```json
{
  "from_address": "0x...",
  "to_address": "0x...",
  "amount": "1000000000000000000",
  "gas_limit": 21000,
  "gas_price": "20000000000",
  "data": "0x...",
  "nonce": 0,
  "chain_id": 1
}
```

**Response:**
```json
"uuid-of-request"
```

### 2. `get_request_status`

Retrieves the current status of a relayer request.

**Parameters:**
```json
"uuid-of-request"
```

**Response:**
```json
{
  "request_id": "uuid",
  "transaction_hash": null,
  "block_number": null,
  "gas_used": null,
  "status": "Pending",
  "completed_at": null,
  "error_message": null
}
```

### 3. `health_check`

Returns the health status and statistics of the service.

**Parameters:**
```json
null
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T00:00:00Z",
  "uptime_seconds": 3600,
  "total_requests": 100,
  "pending_requests": 10,
  "completed_requests": 85,
  "failed_requests": 5
}
```

## Building and Running

### Prerequisites

- Rust 1.70+ and Cargo
- RocksDB development libraries

### Install RocksDB Dependencies

#### macOS
```bash
brew install rocksdb
```

#### Ubuntu/Debian
```bash
sudo apt-get install librocksdb-dev
```

#### CentOS/RHEL
```bash
sudo yum install rocksdb-devel
```

### Build

```bash
cargo build --release
```

### Run

```bash
# Basic usage with defaults
./target/release/relayx

# Custom configuration
./target/release/relayx \
  --rpc-host 0.0.0.0 \
  --rpc-port 8545 \
  --db-path /var/lib/relayx \
  --relayers "0x123...,0x456..." \
  --max-concurrent-requests 200 \
  --request-timeout 60
```

## Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `--rpc-host` | RPC server host address | `127.0.0.1` |
| `--rpc-port` | RPC server port | `8545` |
| `--db-path` | Database path for RocksDB | `./relayx_db` |
| `--relayers` | Comma-separated list of relayer addresses | (empty) |
| `--max-concurrent-requests` | Maximum concurrent requests | `100` |
| `--request-timeout` | Request timeout in seconds | `30` |

## Example Usage

### Submit a Request

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
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

### Check Request Status

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "get_request_status",
    "params": "uuid-from-previous-response"
  }'
```

### Health Check

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "health_check",
    "params": null
  }'
```

## Development

### Project Structure

```
src/
├── main.rs          # Application entry point
├── lib.rs           # Module definitions
├── config.rs        # Configuration and CLI parsing
├── rpc.rs           # JSON-RPC server implementation
├── storage.rs       # RocksDB storage layer
└── types.rs         # Data structures and types
```

### Adding New Endpoints

To add new RPC endpoints, modify the `src/rpc.rs` file and add new methods to the `IoHandler`:

```rust
io.add_method("new_method", move |params: Value| {
    // Implementation here
    async move {
        // Async logic
    }
});
```

### Extending Storage

The storage layer in `src/storage.rs` can be extended with new methods for additional data types or query patterns.

## License

This project is licensed under the MIT License.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request
