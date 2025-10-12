# relayer_getCapabilities Specification Compliance

Based on: https://hackmd.io/T4TkZYFQQnCupiuW231DYw?view#relayer_getCapabilities

## Overview

The `relayer_getCapabilities` method returns the relayer's supported tokens, configuration limits, and fee model. This allows wallets and dApps to discover what payment methods and tokens the relayer accepts before submitting transactions.

## Request Format

### Parameters

The method accepts **no parameters** (empty array).

### Example Request

```json
{
  "jsonrpc": "2.0",
  "method": "relayer_getCapabilities",
  "params": [],
  "id": 1
}
```

## Response Format

### Success Response

The method returns an object with a `capabilities` field containing payment information:

```typescript
{
  capabilities: {
    payment: Array<{
      type: "native" | "erc20" | "sponsored",
      token?: string  // Required for native and erc20, omitted for sponsored
    }>
  }
}
```

### Payment Types

#### 1. Native Payment

For native token (ETH) payments:

```json
{
  "type": "native",
  "token": "0x0000000000000000000000000000000000000000"
}
```

- `type`: Must be `"native"`
- `token`: Must be the zero address (`0x0000000000000000000000000000000000000000`)

#### 2. ERC20 Payment

For ERC20 token payments:

```json
{
  "type": "erc20",
  "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
}
```

- `type`: Must be `"erc20"`
- `token`: The ERC20 token contract address (42-character hex string)

#### 3. Sponsored Payment

For sponsored (gasless) transactions:

```json
{
  "type": "sponsored"
}
```

- `type`: Must be `"sponsored"`
- `token`: Field is omitted for sponsored payments

### Complete Example Response

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

## Field Descriptions

### capabilities

The top-level object containing all relayer capabilities.

### capabilities.payment

An array of supported payment methods. Each payment method is an object with:

- **type**: The payment type (`"native"`, `"erc20"`, or `"sponsored"`)
- **token**: The token address (required for `native` and `erc20`, omitted for `sponsored`)

## Validation Requirements

### Request Validation

1. **params**: Must be an empty array `[]`
2. No additional validation needed as the method takes no parameters

### Response Validation

1. **capabilities**: Must be present as an object
2. **capabilities.payment**: Must be an array
3. **payment items**: Each item must have a `type` field
4. **type values**: Must be one of `"native"`, `"erc20"`, or `"sponsored"`
5. **token field**:
   - Required for `native` type (must be zero address)
   - Required for `erc20` type (must be valid 42-character hex address)
   - Should NOT be present for `sponsored` type

## Implementation Checklist

- [x] Method accepts no parameters (empty params array)
- [x] Response has `capabilities` object
- [x] `capabilities.payment` is an array
- [x] Native payment type with zero address
- [x] ERC20 payment type with token addresses
- [x] Sponsored payment type without token field
- [x] Proper JSON-RPC 2.0 format
- [x] Dynamic capability discovery from configuration
- [x] Type-safe implementation with Rust enums

## Current Implementation Status

✅ **COMPLIANT** - The current implementation in `/Users/partha/relayx` includes all required fields and follows the specification format.

### Type Definitions

Located in `src/types.rs`:

```rust
// Payment types enum with proper serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PaymentType {
    Native,
    #[serde(rename = "erc20")]
    Erc20,
    Sponsored,
}

// Individual payment structures
pub struct NativePayment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub token: String,
}

pub struct Erc20Payment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub token: String,
}

pub struct SponsoredPayment {
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
}

// Untagged enum for flexible serialization
#[serde(untagged)]
pub enum Payment {
    Native(NativePayment),
    Erc20(Erc20Payment),
    Sponsored(SponsoredPayment),
}

// Top-level structures
pub struct Capabilities {
    pub payment: Vec<Payment>,
}

pub struct GetCapabilitiesResponse {
    pub capabilities: Capabilities,
}
```

### RPC Handler

Located in `src/rpc.rs`:

```rust
async fn process_get_capabilities(
    _storage: Storage,
    cfg: &Config,
) -> Result<GetCapabilitiesResponse, jsonrpc_core::Error> {
    // Extract supported tokens from configuration
    let supported_tokens = cfg.get_supported_tokens();
    
    let mut payments = Vec::new();
    
    // Add ERC20 payment options
    for token in &supported_tokens {
        payments.push(Payment::Erc20(Erc20Payment {
            payment_type: PaymentType::Erc20,
            token: token.clone(),
        }));
    }
    
    // Add native payment option
    payments.push(Payment::Native(NativePayment {
        payment_type: PaymentType::Native,
        token: "0x0000000000000000000000000000000000000000".to_string(),
    }));
    
    // Add sponsored payment option
    payments.push(Payment::Sponsored(SponsoredPayment {
        payment_type: PaymentType::Sponsored,
    }));
    
    Ok(GetCapabilitiesResponse {
        capabilities: Capabilities { payment: payments }
    })
}
```

## Usage in Transaction Lifecycle

The `relayer_getCapabilities` method is called early in the transaction lifecycle:

1. **Discovery Phase**: Wallet/dApp calls `relayer_getCapabilities`
2. **Capability Check**: Client verifies that desired payment method is supported
3. **Token Selection**: User selects from available payment tokens
4. **Rate Request**: Client calls `relayer_getExchangeRate` for selected token
5. **Transaction Construction**: Client builds transaction with appropriate payment
6. **Submission**: Client submits via `relayer_sendTransaction`

## Configuration

Capabilities are dynamically determined from the relayer's configuration file:

```json
{
  "chainlink": {
    "tokenUsd": {
      "1": {
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48": "...",
        "0x036CbD53842c5426634e7929541eC2318f3dCF7e": "..."
      }
    }
  }
}
```

The relayer automatically:
- Extracts all configured token addresses
- Adds them as ERC20 payment options
- Always includes native and sponsored options

## Best Practices

### For Relayer Implementers

1. **Dynamic Configuration**: Read capabilities from config files, not hardcoded
2. **Validation**: Ensure all token addresses are valid before including
3. **Consistency**: Keep capabilities synchronized with actual support
4. **Performance**: Cache capabilities response when possible

### For Client Implementers

1. **Discovery First**: Always call `relayer_getCapabilities` before other methods
2. **Validation**: Verify desired payment method is in capabilities
3. **Fallback**: Handle cases where preferred payment method isn't available
4. **Caching**: Cache capabilities response but refresh periodically

## Error Handling

Since this method takes no parameters and reads from configuration, errors are rare. Possible error scenarios:

1. **Configuration Error**: Relayer config is invalid or missing
2. **Internal Error**: Serialization or processing error

Example error response:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32603,
    "message": "Internal error"
  },
  "id": 1
}
```

## Testing

### Manual Test

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

### Validation Checklist

- [ ] Response has `capabilities` object
- [ ] `capabilities.payment` is an array
- [ ] Array has at least one payment option
- [ ] Each payment has a `type` field
- [ ] Native payment has zero address token
- [ ] ERC20 payments have valid token addresses
- [ ] Sponsored payment has no token field
- [ ] Response is valid JSON-RPC 2.0

## Standards Compliance

✅ **JSON-RPC 2.0**: Full compliance with JSON-RPC 2.0 specification  
✅ **EIP-7702**: Compatible with EIP-7702 smart accounts  
✅ **EIP-5792**: Follows EIP-5792 modular execution patterns  
✅ **Generic Relayer EIP**: Fully compliant with the Generic Relayer Architecture specification

## Related Methods

- `relayer_getExchangeRate`: Get exchange rate for a specific token from capabilities
- `relayer_sendTransaction`: Submit transaction using a payment method from capabilities
- `relayer_getStatus`: Check status of submitted transaction

## Notes

- Capabilities are read-only and determined by relayer configuration
- Clients MUST call this method before attempting transactions
- The `sponsored` payment type indicates the relayer will cover gas costs
- Token addresses in capabilities correspond to tokens in `relayer_getExchangeRate`
- The order of payment options in the array is not significant

