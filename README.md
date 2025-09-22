A Rust implementation of the [Generic Relayer Architecture for Smart Accounts EIP](https://hackmd.io/T4TkZYFQQnCupiuW231DYw) that enables gasless and sponsored transactions for smart accounts through a standardized JSON-RPC interface.

## Overview

This relayer service provides a standardized off-chain architecture that allows smart accounts to execute gasless transactions and token-fee payments. The service acts as an intermediary between wallets/dApps and the blockchain, handling transaction submission, gas payment, and status tracking.

### Key Features

- **Gasless Transactions**: Users can pay transaction fees using ERC-20 tokens instead of native tokens
- **Transaction Relaying**: Submit signed transactions through a relayer instead of directly to the network  
- **Real-time Exchange Rates**: Get current token-to-gas conversion rates using Chainlink price feeds
- **Transaction Status Tracking**: Monitor the lifecycle of submitted transactions
- **Multi-chain Support**: Configurable support for multiple blockchain networks
- **Health Monitoring**: Built-in health check and metrics endpoints

## Architecture

The service implements a modular JSON-RPC server with the following components:

```
┌─────────────┐    ┌──────────────┐    ┌─────────────────┐    ┌─────────────┐
│   dApp/     │───▶│    Relayer   │───▶│   Blockchain    │───▶│   Smart     │
│   Wallet    │    │   RPC Server │    │    Networks     │    │  Accounts   │
└─────────────┘    └──────────────┘    └─────────────────┘    └─────────────┘
                          │
                          ▼
                   ┌─────────────┐
                   │  Storage    │
                   │  (Requests, │
                   │   Status)   │
                   └─────────────┘
```

## Build and Run

### Requirements
- Rust (stable) and Cargo
- RocksDB system libraries

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
CLI flags (env in parentheses) and defaults:
- --http-address (HTTP_ADDRESS): 127.0.0.1
- --http-port (HTTP_PORT): 4937
- --http-cors (HTTP_CORS): "*"
- --db-path: ./relayx_db
- --config (RELAYX_CONFIG): optional JSON with:
  - `http_address`, `http_port`, `http_cors`
  - `feeCollector`: address string
  - `rpcs`: { "1": "https://...", "137": "https://..." }
  - `chainlink.nativeUsd`: { chainId: feedAddress }
  - `chainlink.tokenUsd`: { chainId: { tokenAddressLowercased: feedAddress } }

## Supported JSON-RPC Methods

### Core Relayer Methods

1. **`relayer_getExchangeRate`** - Get token exchange rates for gas payment
2. **`relayer_getQuote`** - Simulate transactions and get gas estimates  
3. **`relayer_sendTransaction`** - Submit signed transactions for relay
4. **`relayer_getStatus`** - Check status of submitted transactions
5. **`health_check`** - Service health and metrics

## Usage Examples

### Get Exchange Rate

Request the current rate for paying gas fees with USDC:

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getExchangeRate", 
    "params": [{
      "chainId": "1",
      "token": "0xA0b86a33E6441e6ae7Db1Ce1E5fD4A2Df00b8BB0"
    }],
    "id": 1
  }'
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": [{
    "quote": {
      "rate": 30.5,
      "token": {
        "decimals": 6,
        "address": "0xA0b86a33E6441e6ae7Db1Ce1E5fD4A2Df00b8BB0",
        "symbol": "USDC",
        "name": "USD Coin"
      }
    },
    "gasPrice": "0x4a817c800",
    "feeCollector": "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
    "expiry": 1755917874
  }],
  "id": 1
}
```

### Get Transaction Quote

Simulate a transaction to get gas estimates and required fees:

```bash
curl -X POST http://localhost:8545 \
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
          "token": "0xA0b86a33E6441e6ae7Db1Ce1E5fD4A2Df00b8BB0"
        }
      }
    }],
    "id": 2
  }'
```

### Submit Transaction

Submit a signed transaction for relay:

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0", 
    "method": "relayer_sendTransaction",
    "params": [{
      "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
      "data": "0x...", 
      "chainId": "1",
      "capabilities": {
        "payment": {
          "type": "erc20", 
          "token": "0xA0b86a33E6441e6ae7Db1Ce1E5fD4A2Df00b8BB0"
        }
      }
    }],
    "id": 3
  }'
```

### Check Transaction Status

Query the status of a submitted transaction:

```bash  
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getStatus", 
    "params": {
      "ids": ["0x00000000000000000000000000000000000000000000000000000000000000000e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331"]
    },
    "id": 4
  }'
```

## Configuration

### Environment Variables

Key environment variables for configuration:

- `ETH_RPC_URL` / `RPC_URL` - Default RPC endpoint  
- `RELAYX_FEE_COLLECTOR` - Address to receive relayer fees
- `CHAINLINK_ETH_USD` - Chainlink ETH/USD price feed address
- `CHAINLINK_TOKEN_USD` - Chainlink token/USD price feed address

### Feature Flags

- `onchain` - Enable real blockchain interactions (default: enabled)
  - When disabled, returns stub responses for testing

## Development

### Project Structure

```
src/
├── main.rs              # Application entry point
├── config.rs            # Configuration management  
├── storage.rs           # Data persistence layer
├── types.rs            # JSON-RPC request/response types
└── rpc_server.rs       # Main RPC server implementation
```

### Key Components

#### RpcServer
The main server struct that:
- Handles JSON-RPC method routing
- Manages provider connections with caching
- Processes business logic for each endpoint

#### Storage Layer
Persistent storage for:
- Transaction requests and status
- Request metrics and counts  
- System uptime tracking

#### Price Feed Integration
When `onchain` feature is enabled:
- Fetches real-time gas prices from blockchain
- Queries Chainlink price feeds for token rates
- Calculates exchange rates: `(gasPrice * ETH_price) / token_price`

## Development
- lint: `make lint` (fmt, clippy, cargo-sort, udeps, audit)
- build: `make build`
- run example client: see `examples/test_client.rs`

## CI/CD
- PR CI runs fmt/clippy/sort/udeps/audit
- On PR merge to main/master, a multi-arch Docker image is published to GHCR

## License
MIT
