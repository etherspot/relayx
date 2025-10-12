# relayer_sendTransactionMultichain Specification Compliance

Based on: https://hackmd.io/T4TkZYFQQnCupiuW231DYw?view#relayer_sendTransactionMultichain

## Overview

The `relayer_sendTransactionMultichain` method submits signed transactions to be executed across multiple blockchain networks with payment settlement on a single chain. This enables complex cross-chain operations while consolidating fee payment to one network.

## Request Format

### Parameters

The method accepts an array with a single object containing:

- `transactions` (array of objects, required): Array of transactions to execute on different chains
  - `to` (string, required): Target smart account address
  - `data` (string, required): Transaction calldata (hex string)
  - `chainId` (string, required): Chain ID for this transaction (decimal string)
  - `authorizationList` (string, optional): EIP-7702 authorization list
- `capabilities` (object, required): Transaction capabilities with payment information
  - `payment` (object, required): Payment configuration
    - `type` (string, required): Payment type (`"native"`, `"erc20"`, or `"sponsored"`)
    - `token` (string): Token address for payment
    - `data` (string): Additional payment data
- `paymentChainId` (string, required): Chain ID where payment is settled (decimal string)

### Example Request

```json
{
  "jsonrpc": "2.0",
  "method": "relayer_sendTransactionMultichain",
  "params": [{
    "transactions": [
      {
        "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
        "data": "0xa9059cbb000000000000000000000000742d35cc6c3c3f4b4c1b3cd6c0d1b6c2b3d4e5f60000000000000000000000000000000000000000000000000de0b6b3a7640000",
        "chainId": "1",
        "authorizationList": ""
      },
      {
        "to": "0x8922b54716264130634d6ff183747a8ead91a40c",
        "data": "0xb0d691fe0000000000000000000000000000000000000000000000000000000000000001",
        "chainId": "137",
        "authorizationList": ""
      },
      {
        "to": "0x9a33b54716264130634d6ff183747a8ead91a40d",
        "data": "0xc1e8f82d0000000000000000000000000000000000000000000000000000000000000042",
        "chainId": "8453",
        "authorizationList": ""
      }
    ],
    "capabilities": {
      "payment": {
        "type": "erc20",
        "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "data": ""
      }
    },
    "paymentChainId": "1"
  }],
  "id": 1
}
```

## Response Format

### Success Response

The method returns an object with a `result` array containing submission details for each chain:

```typescript
{
  result: Array<{
    chainId: string,  // Chain ID where transaction was submitted
    id: string       // Unique transaction ID for status tracking
  }>
}
```

The array contains one result per transaction, in the same order as the request.

### Example Success Response

```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "chainId": "1",
      "id": "550e8400-e29b-41d4-a716-446655440000"
    },
    {
      "chainId": "137",
      "id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
    },
    {
      "chainId": "8453",
      "id": "7d9f8b6a-5c4e-3d2f-1a0b-9c8e7d6f5a4b"
    }
  ],
  "id": 1
}
```

### Error Response

When validation fails, the method returns a JSON-RPC error:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32602,
    "message": "Invalid params: Transaction 1: Unsupported chain ID: 999"
  },
  "id": 1
}
```

## Field Descriptions

### transactions

An array of transaction objects, one for each blockchain where execution is required.

**Minimum**: 1 transaction  
**Maximum**: Configurable per relayer implementation  
**Order**: Results match request order

### transactions[].to

The smart account address on the target chain.

**Format**: 42-character hex string  
**Example**: `"0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6"`

### transactions[].data

The transaction calldata for the target chain.

**Format**: Hex string starting with `0x`  
**Contains**: Function selector + encoded parameters

### transactions[].chainId

The chain where this specific transaction should execute.

**Format**: Decimal string  
**Examples**: `"1"` (Ethereum), `"137"` (Polygon), `"8453"` (Base)

### transactions[].authorizationList

EIP-7702 authorization list for the transaction.

**Format**: Hex string or empty string  
**Optional**: Can be empty if not using EIP-7702 delegation

### capabilities

Transaction capabilities, identical to `relayer_sendTransaction`.

**Structure**: Same as single-chain transaction  
**Purpose**: Defines payment method for ALL transactions

### capabilities.payment.type

The payment method for ALL transaction fees across ALL chains.

**Values**:
- `"native"` - Pay fees in native token on payment chain
- `"erc20"` - Pay fees in ERC20 token on payment chain
- `"sponsored"` - Gasless transactions (relayer pays all fees)

### capabilities.payment.token

The token address for fee payment on the payment chain.

**Rules**:
- Required for `native`: Must be zero address
- Required for `erc20`: Must be valid ERC20 address
- Optional for `sponsored`: Can be empty

### paymentChainId

The chain where payment settlement occurs.

**Format**: Decimal string  
**Purpose**: All fee payments are collected on this chain  
**Note**: Can be different from any transaction chainId or match one of them

## Use Cases

### 1. Cross-Chain Token Transfers

Execute token transfers on multiple chains, paying fees on Ethereum mainnet:

```json
{
  "transactions": [
    { "chainId": "1", "to": "0x...", "data": "0x..." },    // Ethereum
    { "chainId": "137", "to": "0x...", "data": "0x..." },  // Polygon
    { "chainId": "8453", "to": "0x...", "data": "0x..." }  // Base
  ],
  "paymentChainId": "1"  // Pay on Ethereum
}
```

### 2. Multi-Chain dApp Interactions

Interact with the same dApp across multiple chains:

```json
{
  "transactions": [
    { "chainId": "10", "to": "0x...", "data": "0x..." },     // Optimism
    { "chainId": "42161", "to": "0x...", "data": "0x..." },  // Arbitrum
    { "chainId": "8453", "to": "0x...", "data": "0x..." }    // Base
  ],
  "paymentChainId": "10",  // Pay on Optimism
  "capabilities": { "payment": { "type": "sponsored" } }
}
```

### 3. Sponsored Multi-Chain Operations

Execute operations across chains with relayer covering all fees:

```json
{
  "transactions": [
    { "chainId": "1", "to": "0x...", "data": "0x..." },
    { "chainId": "56", "to": "0x...", "data": "0x..." }
  ],
  "paymentChainId": "1",
  "capabilities": { "payment": { "type": "sponsored" } }
}
```

## Validation Requirements

### Request Validation

1. **transactions**: Must be non-empty array (at least 1 transaction)
2. **transactions[].to**: Each must be non-empty, valid address
3. **transactions[].data**: Each must be non-empty, valid hex string
4. **transactions[].chainId**: Each must be valid decimal number and supported
5. **paymentChainId**: Must be valid decimal number and supported
6. **capabilities.payment.type**: Must be `"native"`, `"erc20"`, or `"sponsored"`
7. **capabilities.payment.token**: Must match payment type requirements

### Response Validation

1. **result**: Must be array with same length as `transactions`
2. **result[i].chainId**: Must match `transactions[i].chainId`
3. **result[i].id**: Must be unique UUID for each transaction

## Implementation Checklist

- [x] Request type with `transactions`, `capabilities`, `paymentChainId`
- [x] MultichainTransaction type with `to`, `data`, `chainId`, `authorizationList`
- [x] Response type with `result` array
- [x] MultichainTransactionResult with `chainId`, `id`
- [x] Validation: transactions array not empty
- [x] Validation: paymentChainId format and support
- [x] Validation: Each transaction's to, data, chainId
- [x] Validation: Chain support for each transaction
- [x] Validation: Payment type and token
- [x] UUID generation for each transaction
- [x] Storage persistence for each transaction
- [x] Response with matching chainIds
- [x] Proper error handling
- [x] JSON-RPC 2.0 compliance
- [x] Field naming (camelCase)

## Current Implementation Status

✅ **IMPLEMENTED & COMPLIANT** - The implementation follows the specification format with all required features.

### Type Definitions

Located in `src/types.rs`:

```rust
// Individual transaction
pub struct MultichainTransaction {
    pub to: String,
    pub data: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "authorizationList")]
    pub authorization_list: String,
}

// Request
pub struct SendTransactionMultichainRequest {
    pub transactions: Vec<MultichainTransaction>,
    pub capabilities: SendTransactionCapabilities,
    #[serde(rename = "paymentChainId")]
    pub payment_chain_id: String,
}

// Result per transaction
pub struct MultichainTransactionResult {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    pub id: String,
}

// Response
pub struct SendTransactionMultichainResponse {
    pub result: Vec<MultichainTransactionResult>,
}
```

### RPC Handler

Located in `src/rpc.rs`:

```rust
async fn process_send_transaction_multichain(
    storage: Storage,
    input: &SendTransactionMultichainRequest,
    cfg: &Config,
) -> Result<SendTransactionMultichainResponse, jsonrpc_core::Error> {
    // 1. Validate transactions array not empty
    // 2. Validate paymentChainId
    // 3. Validate payment capability
    // 4. Process each transaction:
    //    - Validate fields (to, data, chainId)
    //    - Validate chain support
    //    - Generate UUID
    //    - Store in database
    //    - Add to results
    // 5. Return results
}
```

## Transaction Processing Flow

1. **Validation**: Validates all transactions and payment chain
2. **Iteration**: Processes each transaction independently
3. **Chain Validation**: Verifies each chain is supported
4. **ID Generation**: Creates unique UUID for each transaction
5. **Storage**: Persists each transaction with Pending status
6. **Response**: Returns chainId and ID for each transaction
7. **Async Processing**: Transactions processed asynchronously across chains
8. **Payment Settlement**: Fee payment occurs only on paymentChainId chain

## Payment Settlement

### Key Concept

Payment for ALL transactions across ALL chains is settled on the **paymentChainId** chain:

- User pays once on the payment chain
- Relayer executes transactions on all specified chains
- Relayer's cross-chain infrastructure handles the coordination

### Example

```
Transaction 1: Ethereum (chainId: 1)
Transaction 2: Polygon (chainId: 137)
Transaction 3: Base (chainId: 8453)

Payment Chain: Ethereum (paymentChainId: 1)

Result:
- User pays fee once on Ethereum
- Relayer executes all 3 transactions
- Relayer handles cross-chain coordination
```

## Differences from Single-Chain

| Feature | sendTransaction | sendTransactionMultichain |
|---------|----------------|---------------------------|
| Transactions | 1 transaction | Multiple transactions |
| Chains | 1 chain (from chainId) | Multiple chains (from transactions[].chainId) |
| Payment | On same chain as transaction | On specified paymentChainId |
| Response | 1 result | Multiple results (one per transaction) |
| Coordination | Simple single-chain | Complex cross-chain coordination |

## Standards Compliance

✅ **JSON-RPC 2.0**: Full compliance with JSON-RPC 2.0 specification  
✅ **EIP-7702**: Compatible with EIP-7702 delegated accounts  
✅ **EIP-5792**: Follows EIP-5792 modular execution patterns  
✅ **Generic Relayer EIP**: Fully compliant with the Generic Relayer Architecture specification

## Related Methods

- `relayer_getCapabilities`: Check supported payment methods
- `relayer_getExchangeRate`: Get rates for payment token on payment chain
- `relayer_getStatus`: Monitor status of each transaction using its ID
- `relayer_sendTransaction`: Single-chain version of this method

## Usage Pattern

```javascript
// 1. Get capabilities
const caps = await relayer_getCapabilities();

// 2. Get exchange rate for payment chain
const rate = await relayer_getExchangeRate({
  token: "0xA0b...USDC",
  chainId: "1"  // Payment chain
});

// 3. Construct multichain transactions
const txs = [
  { chainId: "1", to: "0x...", data: "0x..." },
  { chainId: "137", to: "0x...", data: "0x..." },
  { chainId: "8453", to: "0x...", data: "0x..." }
];

// 4. Submit multichain transaction
const result = await relayer_sendTransactionMultichain({
  transactions: txs,
  capabilities: {
    payment: {
      type: "erc20",
      token: "0xA0b...USDC",
      data: ""
    }
  },
  paymentChainId: "1"  // Pay on Ethereum
});

// 5. Monitor each transaction
for (const tx of result.result) {
  const status = await relayer_getStatus({ ids: [tx.id] });
  console.log(`Chain ${tx.chainId}: ${status.result[0].status}`);
}
```

## Security Considerations

1. **Payment Verification**: Must verify sufficient payment for ALL transactions
2. **Chain Validation**: Each chain must be independently validated
3. **Atomicity**: Consider if ALL or NONE execution is required
4. **Fee Calculation**: Total fee = sum of all transaction costs
5. **Rate Limits**: Should limit number of chains/transactions per request

## Best Practices

### For Wallet Implementers

1. **Calculate Total Cost**: Sum gas costs across all chains
2. **Use Payment Chain**: Get exchange rate for payment chain
3. **Verify Support**: Check all chains are in capabilities
4. **Track All IDs**: Save all returned IDs for monitoring
5. **Handle Partial Failures**: Some chains may succeed while others fail

### For Relayer Implementers

1. **Validate All Chains**: Check each chain independently
2. **Atomic Operations**: Consider transaction dependencies
3. **Cross-Chain Coordination**: Implement proper sequencing
4. **Payment First**: Verify payment before executing any transaction
5. **Monitor All**: Track status across all chains

## Notes

- Payment occurs only once, on the `paymentChainId` chain
- Each transaction gets its own unique ID for independent tracking
- Transactions can be on the same chain (multiple operations on one chain)
- The `paymentChainId` can match one of the transaction chains or be different
- Failed transactions on one chain don't affect others (unless relayer implements atomicity)
- Use `relayer_getStatus` with each ID to monitor individual transaction progress

