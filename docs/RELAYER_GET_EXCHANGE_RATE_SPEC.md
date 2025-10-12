# relayer_getExchangeRate Specification Compliance

Based on: https://hackmd.io/T4TkZYFQQnCupiuW231DYw?view#relayer_getExchangeRate

## Overview

The `relayer_getExchangeRate` method fetches token exchange rates for gas payment. It returns the current rate, gas price information, fee collector address, and expiry timestamp for the quote.

## Request Format

### Parameters

The method accepts an array with a single object containing:

- `token` (string, required): The token address for which to get the exchange rate
  - Native token: `"0x0000000000000000000000000000000000000000"`
  - ERC20 token: Valid 42-character hexadecimal address (e.g., `"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"`)
- `chainId` (string, required): The chain ID as a decimal string (e.g., `"1"` for Ethereum mainnet)

### Example Request

```json
{
  "jsonrpc": "2.0",
  "method": "relayer_getExchangeRate",
  "params": [{
    "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "chainId": "1"
  }],
  "id": 1
}
```

## Response Format

### Success Response

The method returns an object with a `result` array containing exchange rate information:

```typescript
{
  result: Array<{
    quote: {
      rate: number,           // Exchange rate for 1 unit of gas in token's decimals
      token: {
        decimals: number,     // Token decimals (e.g., 18 for ETH, 6 for USDC)
        address: string,      // Token contract address
        symbol?: string,      // Token symbol (e.g., "ETH", "USDC")
        name?: string        // Token name (e.g., "Ethereum", "USD Coin")
      }
    },
    gasPrice: string,         // Current gas price as hex string (e.g., "0x4a817c800")
    maxFeePerGas?: string,    // Optional: EIP-1559 max fee per gas (hex string)
    maxPriorityFeePerGas?: string, // Optional: EIP-1559 priority fee (hex string)
    feeCollector: string,     // Address where fees should be sent
    expiry: number           // Unix timestamp when this quote expires
  }>
}
```

### Example Success Response

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
    "maxFeePerGas": null,
    "maxPriorityFeePerGas": null,
    "feeCollector": "0x55f3a93f544e01ce4378d25e927d7c493b863bd6",
    "expiry": 1755917874
  }],
  "id": 1
}
```

### Error Response

When an error occurs, the result array can contain an error object:

```typescript
{
  result: Array<{
    error: {
      id: string,      // Error identifier
      message: string  // Human-readable error message
    }
  }>
}
```

### Example Error Response

```json
{
  "jsonrpc": "2.0",
  "result": [{
    "error": {
      "id": "UNSUPPORTED_TOKEN",
      "message": "Token not supported on this chain"
    }
  }],
  "id": 1
}
```

## Field Descriptions

### quote.rate

The exchange rate represents how much of the specified token is needed per unit of gas. For example:
- A rate of `0.001` for ETH means 0.001 ETH per gas unit
- A rate of `0.0032` for USDC means 0.0032 USDC per gas unit

### gasPrice

The current gas price on the network as a hexadecimal string. For legacy transactions, this is the gas price. For EIP-1559 transactions, this can be used as a fallback.

### maxFeePerGas / maxPriorityFeePerGas

Optional fields for EIP-1559 transaction gas pricing:
- `maxFeePerGas`: Maximum total fee per gas (base fee + priority fee)
- `maxPriorityFeePerGas`: Maximum priority fee per gas (tip)

These fields should be provided for chains that support EIP-1559.

### feeCollector

The Ethereum address where fee payments should be sent. This address is controlled by the relayer and is used to collect payment for the relay service.

### expiry

Unix timestamp (seconds since epoch) when this exchange rate quote expires. Clients should request a new quote after this time. Typical expiry times range from 30 seconds to 10 minutes depending on market volatility.

## Validation Requirements

### Request Validation

1. **token**: Must be a valid Ethereum address (42 characters, starting with `0x`)
2. **chainId**: Must be a valid decimal string representing a supported chain ID

### Response Validation

1. **result**: Must be an array with at least one element
2. **quote.rate**: Must be a positive number
3. **quote.token.decimals**: Must be between 0 and 255
4. **quote.token.address**: Must match the requested token address
5. **gasPrice**: Must be a valid hexadecimal string starting with `0x`
6. **feeCollector**: Must be a valid Ethereum address
7. **expiry**: Must be a Unix timestamp in the future

## Implementation Checklist

- [x] Request type with `token` and `chainId` fields
- [x] Response type with `result` array
- [x] Quote object with `rate` and `token` info
- [x] Token info with `decimals`, `address`, `symbol`, `name`
- [x] Gas price information (`gasPrice`, `maxFeePerGas`, `maxPriorityFeePerGas`)
- [x] Fee collector address
- [x] Expiry timestamp
- [x] Error handling with error variant
- [x] Proper JSON-RPC 2.0 format
- [x] Parameter validation
- [x] Support for both native and ERC20 tokens

## Current Implementation Status

✅ **Compliant** - The current implementation in `/Users/partha/relayx` includes all required fields and follows the specification format.

### Type Definitions

Located in `src/types.rs`:
- `ExchangeRateRequest` - Request structure
- `ExchangeRateResponse` - Response wrapper
- `ExchangeRateResultItem` - Enum for Success or Error
- `ExchangeRateSuccess` - Success response structure
- `ExchangeRateError` - Error response structure
- `ExchangeRateQuote` - Quote information
- `TokenInfo` - Token metadata

### RPC Handler

Located in `src/rpc.rs`:
- Method registration: `relayer_getExchangeRate`
- Implementation: `build_exchange_rate_response_stub`
- Parameter parsing and validation
- Response construction

## Testing

Run the compliance test:

```bash
chmod +x scripts/test_exchange_rate_spec.sh
./scripts/test_exchange_rate_spec.sh
```

## Notes

- The specification follows JSON-RPC 2.0 standards
- The method is designed to work with EIP-7702 smart accounts
- Exchange rates should be updated frequently based on market conditions
- Relayers may implement additional validation or rate limiting

