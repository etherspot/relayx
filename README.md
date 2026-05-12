A Rust implementation of the [Generic Relayer Architecture for Smart Accounts](https://hackmd.io/T4TkZYFQQnCupiuW231DYw) that enables gasless and sponsored transactions for smart accounts through a standardized JSON-RPC interface.

## Overview

RelayX is a high-performance off-chain relayer that accepts signed smart-account transactions, covers the gas, and submits them on-chain. It exposes a JSON-RPC 2.0 API that is fully compliant with the Generic Relayer Architecture specification.

### Key Features

- **Spec-Compliant API**: Full compliance with the Generic Relayer Architecture EIP
- **Gasless Transactions**: Native ETH, ERC-20 token, and sponsored payment modes
- **Task IDs**: 32-byte hex task IDs — auto-generated or client-provided
- **Transaction Simulation**: Pre-flight `eth_call` + `eth_estimateGas` against the wallet ABI
- **Multi-chain**: `relayer_sendTransactionMultichain` submits across N chains with a single payment
- **Webhook Callbacks**: POST final status to a URL when a transaction settles
- **Persistent Storage**: RocksDB-backed request, status, and receipt storage
- **Background Monitor**: Automatic receipt polling, gas-bump resubmission on stalls
- **Fee Data**: Live Chainlink-based token/native exchange rates
- **Health Check**: Built-in uptime and metrics endpoint

## Docker

Pre-built multi-platform image (`linux/amd64` + `linux/arm64`):

```bash
# Run with defaults
docker run --rm -p 4937:4937 0xpartha/relayx:latest

# Run with a config file
docker run --rm -p 4937:4937 \
  -e RELAYX_CONFIG=/app/config.json \
  -v /abs/path/config.json:/app/config.json:ro \
  0xpartha/relayx:latest
```

## Build and Run

### Requirements

- Rust stable + Cargo
- RocksDB system library

```bash
# macOS
brew install rocksdb

# Ubuntu/Debian
sudo apt-get install -y librocksdb-dev build-essential clang libclang-dev pkg-config cmake libssl-dev
```

### Build

```bash
cargo build --release
```

### Run

```bash
./target/release/relayx \
  --http-address 0.0.0.0 \
  --http-port 4937 \
  --config config.json
```

## Configuration

### CLI Flags / Environment Variables

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--http-address` | `HTTP_ADDRESS` | `127.0.0.1` | Bind address |
| `--http-port` | `HTTP_PORT` | `4937` | Bind port |
| `--http-cors` | `HTTP_CORS` | `*` | CORS origins |
| `--log-level` | `LOG_LEVEL` | `debug` | `trace`/`debug`/`info`/`warn`/`error` |
| `--db-path` | — | `./relayx_db` | RocksDB path |
| `--config` | `RELAYX_CONFIG` | — | JSON config file path |
| `--relayer-private-key` | `RELAYX_PRIVATE_KEY` | — | Hex-encoded signer key |
| `--disable-simulation` | `RELAYX_DISABLE_SIMULATION` | `false` | Skip pre-flight simulation (use default gas limit) |
| `--disable-multichain` | `RELAYX_DISABLE_MULTICHAIN` | `false` | Return `4212` for `relayer_sendTransactionMultichain` calls |

Additional environment variables:

| Variable | Description |
|----------|-------------|
| `RELAYX_FEE_COLLECTOR` | Address to receive relayer fees |
| `RELAYX_DEFAULT_TOKEN` | Fallback ERC-20 token address |
| `RELAYX_STUB_MODE=true` | Return deterministic stub responses (no RPC calls) |

### JSON Config File

Optional **`chainIndices`** maps EIP-155 `chainId` strings to OKX-style `chainIndex` values for [status webhooks](#webhook-callbacks). When omitted, the decimal `chainId` is sent as `chainIndex`.

```json
{
  "http_address": "0.0.0.0",
  "http_port": 4937,
  "http_cors": "*",
  "log_level": "info",
  "relayerPrivateKey": "0x...",
  "chainIndices": {
    "1": "1",
    "8453": "8453"
  },
  "feeCollector": "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
  "feeCollectors": {
    "1":   "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
    "137": "0x66f3a93f544e01ce4378d25e927d7c493b863bd7"
  },
  "defaultToken": "0x036CbD53842c5426634e7929541eC2318f3dCF7e",
  "disableMultichain": false,
  "rpcs": {
    "1":   "https://ethereum.publicnode.com",
    "137": "https://polygon-rpc.com"
  },
  "chainlink": {
    "tokenUsd": {
      "1": {
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48": {
          "oracle": "0x8fFfFD4AfB6115b954Bd326cbe7B4BA576818f6",
          "decimals": 6
        }
      },
      "137": {
        "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174": {
          "oracle": "0xAB594600376Ec9fD91F8e885dADF0CE036862dE0",
          "decimals": 6
        }
      }
    },
    "nativeUsd": {
      "1":   "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419",
      "137": "0xAB594600376Ec9fD91F8e885dADF0CE036862dE0"
    }
  }
}
```

> **`feeCollectors`** (optional) — per-chain overrides for the fee collector address. Takes precedence over the global `feeCollector` value for the specified chain.

## JSON-RPC Methods

All methods use JSON-RPC 2.0. The server listens on `POST /`.

---

### `relayer_sendTransaction`

Submit a single transaction for relay.

**Request params** — single object:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `chainId` | `string` | ✅ | Decimal chain ID (e.g. `"137"`) |
| `payment` | `Payment` | ✅ | How the relayer fee is paid |
| `to` | `string` | ✅ | Smart account (wallet) address |
| `data` | `string` | ✅ | ABI-encoded `executeWithRelayer(...)` calldata |
| `context` | `object` | — | Arbitrary metadata; `callbackUrl` and `expiry` are read from here |
| `authorizationList` | `AuthorizationItem[]` | — | EIP-7702 authorization entries |
| `taskId` | `string` | — | Client-provided 32-byte hex task ID (`0x`-prefixed, 66 chars) |

**Payment object:**

```json
{ "type": "token", "address": "0x0000000000000000000000000000000000000000" }
```

| `type` | `address` | Meaning |
|--------|-----------|---------|
| `"token"` | zero address | Pay fee in native ETH |
| `"token"` | ERC-20 address | Pay fee in that token |
| `"sponsored"` | — | Relayer covers the fee |

**AuthorizationItem** (EIP-7702):

```json
{
  "address":  "0x...",
  "chainId":  1,
  "nonce":    0,
  "r": "0x...", "s": "0x...", "yParity": 0
}
```

**Response** — task ID string:

```json
{
  "jsonrpc": "2.0",
  "result": "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331",
  "id": 1
}
```

**Example:**

```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_sendTransaction",
    "params": [{
      "chainId": "137",
      "payment": { "type": "sponsored" },
      "to": "0x742d35Cc6634e7929541eC2318f3dCF7e6C3C3f4",
      "data": "0xdeadbeef...",
      "context": { "callbackUrl": "https://yourapp.com/webhook/relay" }
    }],
    "id": 1
  }'
```

---

### `relayer_sendTransactionMultichain`

Submit transactions across multiple chains with a single payment. Accepts an **array** of `SendTransactionParams` objects.

**Rules:**
- The **first** item specifies the payment (`payment.type != "sponsored"`).
- All subsequent items **must** use `"sponsored"` payment.
- Minimum two items.

**Response** — array of task ID strings (one per item, same order):

```json
{
  "jsonrpc": "2.0",
  "result": [
    "0xabc123...taskid1...",
    "0xdef456...taskid2..."
  ],
  "id": 1
}
```

**Example:**

```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_sendTransactionMultichain",
    "params": [
      {
        "chainId": "1",
        "payment": { "type": "token", "address": "0x0000000000000000000000000000000000000000" },
        "to": "0xSmartAccount1...",
        "data": "0x..."
      },
      {
        "chainId": "137",
        "payment": { "type": "sponsored" },
        "to": "0xSmartAccount2...",
        "data": "0x..."
      }
    ],
    "id": 1
  }'
```

---

### `relayer_getStatus`

Query the status of a submitted transaction.

**Request params** — single object:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string` | Task ID returned by `sendTransaction` |
| `logs` | `bool` | Include event logs in the receipt |

**Status codes:**

| Code | Meaning |
|------|---------|
| `100` | Pending — received, not yet submitted on-chain |
| `110` | Submitted — on-chain, awaiting confirmation |
| `200` | Confirmed — successfully included in a block |
| `400` | Rejected — off-chain failure (simulation, relay error) |
| `500` | Reverted — included but reverted on-chain |

**Response** (varies by status):

```json
{
  "jsonrpc": "2.0",
  "result": {
    "chainId": "137",
    "createdAt": 1741234567,
    "status": 200,
    "hash": "0x9b7bb827...",
    "receipt": {
      "blockHash":        "0xf19bbafd...",
      "blockNumber":      "43981",
      "gasUsed":          "3567",
      "transactionHash":  "0x9b7bb827...",
      "logs": [
        {
          "address": "0xa922b547...",
          "topics":  ["0x5a2a9072..."],
          "data":    "0xabcd"
        }
      ]
    }
  },
  "id": 1
}
```

`receipt` is only present for status `200`. `message` and `data` are only present for status `400`/`500`.

**Example:**

```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getStatus",
    "params": { "id": "0x0e670ec6...", "logs": true },
    "id": 1
  }'
```

---

### `relayer_getCapabilities`

Discover which chains and tokens the relayer supports.

**Request params** — optional array of chain ID strings to filter:

```json
{ "params": ["1", "137"] }
```

**Response** — map of chain ID → capabilities:

```json
{
  "jsonrpc": "2.0",
  "result": {
    "1": {
      "feeCollector": "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
      "tokens": [
        { "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "decimals": 6 }
      ]
    },
    "137": {
      "feeCollector": "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
      "tokens": [
        { "address": "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", "decimals": 6 }
      ]
    }
  },
  "id": 1
}
```

---

### `relayer_getFeeData`

Get current fee pricing for a token on a chain.

**Request params** — single object:

| Field | Type | Description |
|-------|------|-------------|
| `chainId` | `string` | Decimal chain ID |
| `token` | `string` | Token address (zero address for native ETH) |

**Response:**

| Field | Description |
|-------|-------------|
| `chainId` | Chain the fee data applies to |
| `token` | Token descriptor (`address`, `decimals`) |
| `rate` | Tokens per 1 unit of native currency (e.g. USDC/ETH ≈ 2000.5); always `1.0` for native |
| `gasPrice` | Current gas price in wei (decimal string) |
| `maxFeePerGas` | EIP-1559 max fee per gas in wei (decimal string, omitted on pre-EIP-1559 chains) |
| `maxPriorityFeePerGas` | EIP-1559 max priority fee per gas in wei (decimal string, omitted on pre-EIP-1559 chains) |
| `feeCollector` | Address clients should send payment tokens to (mirrors `relayer_getCapabilities`) |
| `expiry` | Unix timestamp when this quote expires |
| `minFee` | Minimum fee in token units (optional) |
| `context` | Opaque signed quote context (optional) |

**Example:**

```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getFeeData",
    "params": [{ "chainId": "1", "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48" }],
    "id": 1
  }'
```

```json
{
  "jsonrpc": "2.0",
  "result": {
    "chainId": "1",
    "token": { "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "decimals": 6 },
    "rate": 2000.5,
    "gasPrice": "20000000000",
    "maxFeePerGas": "40000000000",
    "maxPriorityFeePerGas": "1000000000",
    "feeCollector": "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
    "expiry": 1741234867
  },
  "id": 1
}
```

---

### `health_check`

Returns service health and counters.

```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"health_check","params":[],"id":1}'
```

```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "healthy",
    "timestamp": "2026-03-12T08:00:00Z",
    "uptime_seconds": 86400,
    "total_requests": 1250,
    "pending_requests": 5,
    "completed_requests": 1200,
    "failed_requests": 45
  },
  "id": 1
}
```

---

## Webhook Callbacks

Pass a `callbackUrl` inside `context` to receive a `POST` when the transaction settles:

```json
{
  "chainId": "137",
  "payment": { "type": "sponsored" },
  "to": "0x...",
  "data": "0x...",
  "context": {
    "callbackUrl": "https://yourapp.com/webhook/relay-status",
    "expiry": 1741234867
  }
}
```

The HTTP body is a **JSON array** with one element, matching the OKX Wallet “Submit Intent Status” shape ([Relayer Integration API](https://hackmd.io/@michaelwong/SJVSZ1wcgx)): flattened fields, string timestamps and status codes, and `blockHeight` / `gasUsed` as `0x` hex quantities when a receipt is available.

```json
[
  {
    "timestamp": "1755914000",
    "requestId": "550e8400-e29b-41d4-a716-446655440000",
    "taskId": "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331",
    "chainIndex": "137",
    "status": "200",
    "blockHash": "0x6789b0746d84002f2f258129cfd9714d412e78b4d91b8e61608fac9165988baf",
    "blockHeight": "0x22a1e6e",
    "gasUsed": "0x9cf2",
    "txHash": "0xd9b01a72502e7f518fb043bfacd1e13b07f24995f404f8cbb60a1212ca8b4c42"
  }
]
```

For failures, `errorMessage` is set (and optional `errorData`) per the same integration guide. `chainIndex` defaults to the request’s decimal `chainId` unless you configure `chainIndices` in `config.json`.

Callbacks fire on all terminal states:

| Fired when | `status` |
|------------|---------|
| On-chain confirmation | `200` |
| Relay submission failure | `400` |
| On-chain revert | `500` |

Callback failures are logged and silently dropped; they never affect the relay flow.

---

## Error Codes

Full set of spec-defined error codes (per the [Generic Relayer Architecture spec](https://hackmd.io/T4TkZYFQQnCupiuW231DYw)):

| Code | Name | Where raised |
|------|------|-------------|
| `-32602` | Invalid Params | Missing or malformed request fields |
| `4200` | Insufficient Payment | Fee-verification middleware: payment below required minimum |
| `4201` | Invalid Signature | Signature-validation middleware: signer mismatch |
| `4202` | Unsupported Payment Token | ERC-20 token not in configured token list |
| `4203` | Rate Limit Exceeded | Rate-limiting middleware: per-address or per-key quota exceeded |
| `4204` | Quote Expired | `context.expiry` is in the past at submission time |
| `4205` | Insufficient Balance | Wallet native balance too low to cover gas cost |
| `4206` | Unsupported Chain | Chain ID not in relayer's config |
| `4207` | Transaction Too Large | Calldata exceeds 128 KB |
| `4208` | Unknown Transaction ID | No request found for the given task ID |
| `4209` | Unsupported Capability | Payment type not supported |
| `4210` | Invalid Authorization List | Malformed EIP-7702 authorization entries |
| `4211` | Simulation Failed | `eth_call` reverted during pre-flight check |
| `4212` | Multichain Not Supported | `--disable-multichain` is set on this instance |
| `4213` | Invalid Task ID | Client-provided `taskId` is not a valid 32-byte hex string |
| `4214` | Duplicate Task ID | Client-provided `taskId` already has an associated job |

> Codes `4200`, `4201`, and `4203` are available as `pub(crate)` helpers for operators adding auth or rate-limiting middleware; they are not raised by the core relay path itself.

---

## Transaction Simulation

Before accepting a transaction, the relayer:

1. Loads `resources/abi.json` (the smart-account wallet ABI).
2. Checks the first 4 bytes of `data` match the `executeWithRelayer` selector.
3. Calls `eth_call` on the chain — fails immediately if the tx would revert.
4. Calls `eth_estimateGas` — the result is stored as the gas limit for submission.

Simulation can be skipped by setting `--disable-simulation` or `RELAYX_STUB_MODE=true`.

---

## Architecture

```
Client
  │  relayer_sendTransaction / relayer_sendTransactionMultichain
  ▼
JSON-RPC Server (rpc.rs)
  ├── Validate params (chain, payment, selector)
  ├── Simulate (eth_call + eth_estimateGas)
  ├── Store request (RocksDB)
  └── Submit transaction on-chain
          │
          ▼
Background Monitor (tokio::spawn)
  ├── Poll pending/processing requests every 10s
  ├── Fetch receipts (eth_getTransactionReceipt)
  ├── Gas-bump resubmission on stalls
  └── Fire webhook callbacks on terminal state
```

### Storage Keys (RocksDB)

| Prefix | Contents |
|--------|----------|
| `request:<uuid>` | `RelayerRequest` JSON |
| `taskid:<task_id>` | Internal UUID (secondary index) |
| `receipt:<uuid>` | `SpecReceipt` JSON |
| `response:<uuid>` | `RelayerResponse` JSON |
| `resubmission:<uuid>:<chainId>:<hash>` | `Resubmission` JSON |

---

## Development

### Build and Test

```bash
# Build
cargo build --release

# Run all tests (40 tests: 12 unit + 28 integration, ~20ms)
cargo test

# Lint
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

### Project Structure

```
src/
├── main.rs       # CLI entry point
├── config.rs     # JSON + env config, supported_chain_ids()
├── storage.rs    # RocksDB layer: requests, receipts, task_id index
├── types.rs      # All JSON-RPC request/response types
├── rpc.rs        # Endpoint handlers, simulation, background monitor
└── lib.rs        # Module exports

resources/
└── abi.json      # Smart-account wallet ABI (executeWithRelayer selector)

tests/
└── rpc_tests.rs  # Integration tests (28 tests)
```

### Test Coverage

| Suite | Tests |
|-------|-------|
| `send_transaction_tests` | 8 |
| `send_transaction_multichain_tests` | 4 |
| `get_status_tests` | 2 |
| `fee_data_tests` | 3 |
| `quote_tests` | 2 |
| `storage_tests` | 7 |
| `config_tests` | 2 |
| **Total** | **28** |

---

## CI/CD

The GitHub Actions workflow (`.github/workflows/fast-ci.yml`) runs on every PR and push to main:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- Multi-arch Docker build (`linux/amd64` + `linux/arm64`) published to `0xpartha/relayx:latest`

### Build the image locally

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t 0xpartha/relayx:latest \
  --push .
```

---

## License

MIT

## Linting and Formatting

```bash
# format code
cargo fmt --all

# check formatting only
cargo fmt --all -- --check
# or: cargo fmt-check
# or: make fmt-check

# run clippy with warnings denied
cargo clippy --workspace --all-targets --all-features -- -D warnings
# or: cargo lint
# or: make clippy

# run both checks
make lint-check
```
