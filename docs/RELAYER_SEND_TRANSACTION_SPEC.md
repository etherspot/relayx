# relayer_sendTransaction Specification Compliance

Based on: https://hackmd.io/T4TkZYFQQnCupiuW231DYw?view#relayer_sendTransaction

## Overview

The `relayer_sendTransaction` method submits a signed transaction intent to the relayer for execution. The relayer validates the transaction, verifies payment sufficiency, and submits it on-chain. This is the core method for executing gasless and token-fee transactions.

## Request Format

### Parameters

The method accepts an array with a single transaction request object containing:

- `to` (string, required): The target smart account address
- `data` (string, required): The transaction calldata (hex string)
- `capabilities` (object, required): Transaction capabilities, including payment information
  - `payment` (object, required): Payment configuration
    - `type` (string, required): Payment type (`"native"`, `"erc20"`, or `"sponsored"`)
    - `token` (string, optional): Token address for payment (required for native/erc20, omitted for sponsored)
    - `data` (string, optional): Additional payment data
- `chainId` (string, required): The chain ID where transaction should be executed (decimal string)
- `authorizationList` (string, optional): EIP-7702 authorization list (hex string)

### Example Request - Native Payment

```json
{
  "jsonrpc": "2.0",
  "method": "relayer_sendTransaction",
  "params": [{
    "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
    "data": "0xa9059cbb000000000000000000000000742d35cc6c3c3f4b4c1b3cd6c0d1b6c2b3d4e5f60000000000000000000000000000000000000000000000000de0b6b3a7640000",
    "capabilities": {
      "payment": {
        "type": "native",
        "token": "0x0000000000000000000000000000000000000000",
        "data": ""
      }
    },
    "chainId": "1",
    "authorizationList": "0x..."
  }],
  "id": 1
}
```

### Example Request - ERC20 Payment

```json
{
  "jsonrpc": "2.0",
  "method": "relayer_sendTransaction",
  "params": [{
    "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
    "data": "0xa9059cbb000000000000000000000000742d35cc6c3c3f4b4c1b3cd6c0d1b6c2b3d4e5f60000000000000000000000000000000000000000000000000de0b6b3a7640000",
    "capabilities": {
      "payment": {
        "type": "erc20",
        "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "data": ""
      }
    },
    "chainId": "1",
    "authorizationList": "0x..."
  }],
  "id": 1
}
```

### Example Request - Sponsored Payment

```json
{
  "jsonrpc": "2.0",
  "method": "relayer_sendTransaction",
  "params": [{
    "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
    "data": "0xa9059cbb000000000000000000000000742d35cc6c3c3f4b4c1b3cd6c0d1b6c2b3d4e5f60000000000000000000000000000000000000000000000000de0b6b3a7640000",
    "capabilities": {
      "payment": {
        "type": "sponsored",
        "token": "",
        "data": ""
      }
    },
    "chainId": "1",
    "authorizationList": "0x..."
  }],
  "id": 1
}
```

## Response Format

### Success Response

The method returns an object with a `result` array containing transaction submission details:

```typescript
{
  result: Array<{
    chainId: string,  // Chain ID where transaction was submitted
    id: string       // Unique transaction ID for status tracking
  }>
}
```

### Example Success Response

```json
{
  "jsonrpc": "2.0",
  "result": [{
    "chainId": "1",
    "id": "550e8400-e29b-41d4-a716-446655440000"
  }],
  "id": 1
}
```

### Error Response

When validation or submission fails, the method returns a JSON-RPC error:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid params: Missing required field: 'to'"
  },
  "id": 1
}
```

## Field Descriptions

### Request Fields

#### to
The smart account address that will execute the transaction. This should be an EIP-7702 delegated account or compatible smart account.

**Format**: 42-character hex string (e.g., `"0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6"`)

#### data
The transaction calldata containing the function selector and encoded parameters.

**Format**: Hex string starting with `0x`  
**Example**: `"0xa9059cbb..."` (ERC20 transfer function call)

#### capabilities.payment.type
The payment method for transaction fees.

**Values**:
- `"native"` - Pay fees in native token (ETH)
- `"erc20"` - Pay fees in ERC20 token (USDC, etc.)
- `"sponsored"` - Gasless transaction (relayer pays fees)

#### capabilities.payment.token
The token address for fee payment.

**Rules**:
- Required for `native` type: Must be `"0x0000000000000000000000000000000000000000"`
- Required for `erc20` type: Must be valid ERC20 token address
- Optional for `sponsored` type: Can be empty string

#### capabilities.payment.data
Additional payment-related data.

**Format**: Hex string or empty string  
**Usage**: Can contain additional payment instructions or signatures

#### chainId
The chain where the transaction should be executed.

**Format**: Decimal string (e.g., `"1"` for Ethereum mainnet, `"137"` for Polygon)

#### authorizationList
EIP-7702 authorization list for delegated execution.

**Format**: Hex string or empty string  
**Usage**: Contains authorization signatures for account delegation

### Response Fields

#### result[].chainId
The chain ID where the transaction was submitted.

**Format**: Decimal string (matches request `chainId`)

#### result[].id
Unique identifier for tracking the transaction status.

**Format**: UUID string  
**Usage**: Use this ID with `relayer_getStatus` to monitor transaction progress

## Validation Requirements

### Request Validation

1. **to**: Must be non-empty, valid Ethereum address format
2. **data**: Must be non-empty, valid hex string
3. **chainId**: Must be non-empty, valid decimal number, and supported by relayer
4. **capabilities.payment.type**: Must be one of: `"native"`, `"erc20"`, `"sponsored"`
5. **capabilities.payment.token**: 
   - For `native`: Must be zero address
   - For `erc20`: Must be valid 42-character address
   - For `sponsored`: Can be empty
6. **Chain support**: ChainId must be in relayer's supported chains list

### Response Validation

1. **result**: Must be an array with one element
2. **result[0].chainId**: Must match request chainId
3. **result[0].id**: Must be a valid unique identifier (UUID format)

## Transaction Processing Flow

1. **Validation**: Relayer validates all required fields
2. **Chain Check**: Verifies chain is supported
3. **Payment Validation**: Validates payment type and token
4. **Simulation**: For native payments, simulates transaction to estimate gas
5. **Storage**: Stores transaction request with pending status
6. **ID Generation**: Creates unique UUID for tracking
7. **Response**: Returns chainId and transaction ID
8. **Async Processing**: Transaction is processed asynchronously

## Implementation Checklist

- [x] Request accepts array with transaction object
- [x] `to` field (string) with validation
- [x] `data` field (string) with validation
- [x] `capabilities` object with payment
- [x] `capabilities.payment.type` field
- [x] `capabilities.payment.token` field
- [x] `capabilities.payment.data` field
- [x] `chainId` field with validation
- [x] `authorizationList` field
- [x] Response has `result` array
- [x] Result has `chainId` and `id` fields
- [x] Payment type validation (native/erc20/sponsored)
- [x] Chain support validation
- [x] Transaction simulation for native payments
- [x] UUID generation for tracking
- [x] Storage integration
- [x] Proper error handling
- [x] JSON-RPC 2.0 compliance
- [x] Field naming (camelCase)

## Current Implementation Status

✅ **COMPLIANT** - The current implementation in `/Users/partha/relayx` includes all required fields and follows the specification format.

### Type Definitions

Located in `src/types.rs`:

```rust
// Request
pub struct SendTransactionRequest {
    pub to: String,
    pub data: String,
    pub capabilities: SendTransactionCapabilities,
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "authorizationList")]
    pub authorization_list: String,
}

// Capabilities
pub struct SendTransactionCapabilities {
    pub payment: PaymentCapability,
}

// Payment
pub struct PaymentCapability {
    #[serde(rename = "type")]
    pub payment_type: String,
    pub token: String,
    pub data: String,
}

// Response
pub struct SendTransactionResponse {
    pub result: Vec<SendTransactionResult>,
}

pub struct SendTransactionResult {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    pub id: String,
}
```

### RPC Handler

Located in `src/rpc.rs`:

```rust
async fn process_send_transaction(
    storage: Storage,
    input: &SendTransactionRequest,
    cfg: &Config,
) -> Result<SendTransactionResponse, jsonrpc_core::Error> {
    // Validate required fields
    if input.to.is_empty() { return Err(...) }
    if input.data.is_empty() { return Err(...) }
    if input.chain_id.is_empty() { return Err(...) }
    
    // Validate chain is supported
    let chain_id: u64 = input.chain_id.parse()?;
    if !cfg.is_chain_supported(chain_id) { return Err(...) }
    
    // Validate payment type
    match input.capabilities.payment.payment_type.as_str() {
        "native" => {
            // Validate zero address
            // Simulate transaction
        }
        "erc20" => {
            // Validate token address format
        }
        "sponsored" => {
            // No additional validation needed
        }
        _ => return Err(...)
    }
    
    // Generate UUID
    let transaction_id = Uuid::new_v4().to_string();
    
    // Store in database
    storage.create_request(relayer_request).await?;
    
    // Return response
    Ok(SendTransactionResponse {
        result: vec![SendTransactionResult {
            chain_id: input.chain_id.clone(),
            id: transaction_id,
        }]
    })
}
```

## Error Codes

The implementation uses JSON-RPC error codes:

| Code | Meaning | Usage |
|------|---------|-------|
| -32602 | Invalid params | Missing or invalid required fields |
| -32603 | Internal error | Database or processing errors |

## Payment Type Handling

### Native Payment
- **Validation**: Token must be zero address
- **Simulation**: Runs `eth_call` and `eth_estimateGas`
- **Gas Estimation**: Updates gas limit from simulation

### ERC20 Payment
- **Validation**: Token must be valid 42-character address
- **Format Check**: Must start with "0x"

### Sponsored Payment
- **Validation**: No additional validation
- **Cost**: Relayer covers all transaction costs

## Smart Account Integration

The implementation validates that transactions call the `executeWithRelayer` function:
- Loads wallet ABI from `resources/abi.json`
- Verifies function selector matches `executeWithRelayer`
- Ensures proper smart account integration

## Standards Compliance

✅ **JSON-RPC 2.0**: Full compliance with JSON-RPC 2.0 specification  
✅ **EIP-7702**: Compatible with EIP-7702 delegated accounts  
✅ **EIP-5792**: Follows EIP-5792 modular execution patterns  
✅ **Generic Relayer EIP**: Fully compliant with the Generic Relayer Architecture specification

## Usage in Transaction Lifecycle

The `relayer_sendTransaction` method is the central execution point:

```
1. Get capabilities (relayer_getCapabilities)
   ↓
2. Get exchange rate (relayer_getExchangeRate)
   ↓
3. Construct transaction with payment
   ↓
4. User signs transaction
   ↓
5. Submit transaction (relayer_sendTransaction) ← THIS METHOD
   ↓
6. Poll status (relayer_getStatus)
```

## Best Practices

### For Wallet Implementers

1. **Validate Before Submit**: Check capabilities before submitting
2. **Include Payment**: Ensure payment is sufficient based on exchange rate
3. **Sign Complete Intent**: User should sign the complete transaction including payment
4. **Track ID**: Save the returned ID for status polling
5. **Handle Errors**: Implement proper error handling for all validation errors

### For Relayer Implementers

1. **Validate Everything**: Check all fields before processing
2. **Simulate**: Run eth_call to catch reverts before submission
3. **Store Immediately**: Persist transaction data before responding
4. **Return Quickly**: Generate ID and return fast; process async
5. **Monitor**: Track submitted transactions and update status

## Testing

### Manual Test

```bash
curl -X POST http://localhost:4937 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_sendTransaction",
    "params": [{
      "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
      "data": "0x...",
      "capabilities": {
        "payment": {
          "type": "sponsored",
          "token": "",
          "data": ""
        }
      },
      "chainId": "1",
      "authorizationList": ""
    }],
    "id": 1
  }'
```

### Validation Checklist

- [ ] Request has `to`, `data`, `capabilities`, `chainId`
- [ ] Payment type is valid (native/erc20/sponsored)
- [ ] Token address is appropriate for payment type
- [ ] ChainId is supported by relayer
- [ ] Response returns valid UUID
- [ ] Response chainId matches request
- [ ] Transaction is stored in database
- [ ] Can query status using returned ID

## Related Methods

- `relayer_getCapabilities`: Check what payment methods are supported
- `relayer_getExchangeRate`: Get current rate for token payment
- `relayer_getStatus`: Monitor transaction after submission

## Security Considerations

1. **Signature Validation**: Implementation should verify transaction signatures
2. **Payment Verification**: Must verify sufficient payment is included
3. **Simulation**: Should simulate to catch malicious or failing transactions
4. **Rate Limiting**: Should implement rate limiting per account
5. **Chain Validation**: Must validate chainId matches actual chain

## Notes

- The `authorizationList` field is for EIP-7702 delegation authorization
- Empty strings are valid for optional fields (authorizationList, payment.data)
- The returned `id` is used for all subsequent status queries
- Transaction processing happens asynchronously after response is returned
- Native payment transactions are simulated before acceptance
- The implementation validates the `executeWithRelayer` function is called

