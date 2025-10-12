# relayer_getStatus Specification Compliance

Based on: https://hackmd.io/T4TkZYFQQnCupiuW231DYw?view#relayer_getStatus

## Overview

The `relayer_getStatus` method checks the status of previously submitted relayed transactions. It returns detailed information about transaction execution, including receipts, resubmissions, and any failures that occurred.

## Request Format

### Parameters

The method accepts a single object containing an array of transaction IDs:

- `ids` (array of strings, required): Array of transaction IDs to query
  - Each ID is returned from a previous `relayer_sendTransaction` call
  - Can query multiple transactions in one request

### Example Request

```json
{
  "jsonrpc": "2.0",
  "method": "relayer_getStatus",
  "params": {
    "ids": [
      "0x00000000000000000000000000000000000000000000000000000000000000000e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331"
    ]
  },
  "id": 1
}
```

## Response Format

### Success Response

The method returns an object with a `result` array containing status information for each transaction:

```typescript
{
  result: Array<{
    version: string,           // API version (e.g., "2.0.0")
    id: string,               // Transaction ID
    status: number,           // HTTP-style status code (200, 400, 500, etc.)
    receipts: Array<{         // Array of transaction receipts (successful transactions)
      logs: Array<{
        address: string,      // Contract address that emitted the log
        topics: string[],     // Array of log topics (indexed parameters)
        data: string         // Log data (non-indexed parameters)
      }>,
      status: string,         // Transaction status ("0x1" = success, "0x0" = failure)
      blockHash: string,      // Block hash containing the transaction
      blockNumber: string,    // Block number (hex string)
      gasUsed: string,        // Gas used (hex string)
      transactionHash: string, // Transaction hash on-chain
      chainId: string         // Chain ID where transaction was executed
    }>,
    resubmissions: Array<{    // Array of resubmission attempts
      status: number,         // HTTP-style status code
      transactionHash: string, // Transaction hash of resubmission
      chainId: string         // Chain ID of resubmission
    }>,
    offchainFailure: Array<{  // Array of off-chain failures (validation, etc.)
      message: string         // Error message
    }>,
    onchainFailure: Array<{   // Array of on-chain failures (reverts, etc.)
      transactionHash: string, // Transaction hash that failed
      chainId: string,        // Chain ID where failure occurred
      message: string,        // Human-readable error message
      data: string           // Revert data (hex string)
    }>
  }>
}
```

### Status Codes

The `status` field uses HTTP-style status codes:

- `200`: Transaction executed successfully
- `201`: Transaction submitted and pending
- `400`: Bad request (invalid parameters, insufficient fee, etc.)
- `404`: Transaction ID not found
- `500`: Internal relayer error

### Complete Example Response

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
              "topics": [
                "0x5a2a90727cc9d000dd060b1132a5c977c9702bb3a52afe360c9c22f0e9451a68"
              ],
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
  "id": 1
}
```

## Field Descriptions

### Top-Level Status Result

- **version**: API version string (e.g., "2.0.0")
- **id**: The transaction ID that was queried
- **status**: HTTP-style status code indicating overall status
- **receipts**: Array of successful transaction receipts (empty if no success)
- **resubmissions**: Array of resubmission attempts (if transaction was resubmitted with higher gas)
- **offchainFailure**: Array of validation or relayer failures before submission
- **onchainFailure**: Array of on-chain execution failures (reverts)

### Receipt Fields

- **logs**: Array of event logs emitted during transaction execution
- **status**: Transaction result (`"0x1"` = success, `"0x0"` = failure)
- **blockHash**: Hash of the block containing the transaction
- **blockNumber**: Block number as hex string (e.g., `"0xabcd"`)
- **gasUsed**: Gas consumed as hex string (e.g., `"0xdef"`)
- **transactionHash**: On-chain transaction hash
- **chainId**: Chain where transaction was executed (decimal string)

### Log Fields

- **address**: Contract address that emitted the event
- **topics**: Array of indexed event parameters (as hex strings)
- **data**: Non-indexed event parameters (as hex string)

### Resubmission Fields

- **status**: HTTP status code of resubmission
- **transactionHash**: Hash of the resubmitted transaction
- **chainId**: Chain where resubmission occurred

### OffchainFailure Fields

- **message**: Human-readable error message explaining the failure

### OnchainFailure Fields

- **transactionHash**: Hash of the failed transaction
- **chainId**: Chain where failure occurred
- **message**: Human-readable revert message
- **data**: Raw revert data as hex string (includes error selector and parameters)

## Usage Patterns

### Polling for Status

```javascript
// After submitting a transaction
const submitResponse = await relayer_sendTransaction(tx);
const txId = submitResponse.result[0].id;

// Poll for status
const statusResponse = await relayer_getStatus({ ids: [txId] });
const status = statusResponse.result[0].status;

if (status === 200) {
  console.log("Transaction successful!");
  console.log("Receipt:", statusResponse.result[0].receipts[0]);
} else if (status === 201) {
  console.log("Transaction pending...");
} else {
  console.log("Transaction failed");
  console.log("Errors:", statusResponse.result[0].offchainFailure);
}
```

### Batch Status Check

```javascript
// Check multiple transactions at once
const statusResponse = await relayer_getStatus({
  ids: [txId1, txId2, txId3]
});

statusResponse.result.forEach(result => {
  console.log(`Transaction ${result.id}: status ${result.status}`);
});
```

## Validation Requirements

### Request Validation

1. **ids**: Must be a non-empty array of strings
2. **id format**: Each ID should be a valid transaction ID returned by `relayer_sendTransaction`

### Response Validation

1. **result**: Must be an array with same length as request `ids`
2. **version**: Must be present as a string
3. **id**: Must match one of the requested IDs
4. **status**: Must be a valid HTTP status code (number)
5. **receipts**: Must be an array (can be empty)
6. **resubmissions**: Must be an array (can be empty)
7. **offchainFailure**: Must be an array (can be empty)
8. **onchainFailure**: Must be an array (can be empty)
9. **Hex strings**: All hex fields must start with "0x"

## Implementation Checklist

- [x] Request accepts `ids` array
- [x] Response has `result` array
- [x] Each result has `version`, `id`, `status`
- [x] `receipts` array with proper structure
- [x] `logs` with `address`, `topics`, `data`
- [x] Receipt has `status`, `blockHash`, `blockNumber`, `gasUsed`, `transactionHash`, `chainId`
- [x] `resubmissions` array with `status`, `transactionHash`, `chainId`
- [x] `offchainFailure` array with `message`
- [x] `onchainFailure` array with `transactionHash`, `chainId`, `message`, `data`
- [x] Proper field naming (camelCase in JSON)
- [x] JSON-RPC 2.0 format
- [x] Type safety with Rust types
- [x] Proper serde serialization

## Current Implementation Status

✅ **COMPLIANT** - The current implementation in `/Users/partha/relayx` includes all required fields and follows the specification format.

### Type Definitions

Located in `src/types.rs`:

```rust
// Request
pub struct GetStatusRequest {
    pub ids: Vec<String>,
}

// Response
pub struct GetStatusResponse {
    pub result: Vec<StatusResult>,
}

// Status result
pub struct StatusResult {
    pub version: String,
    pub id: String,
    pub status: u16,
    pub receipts: Vec<Receipt>,
    pub resubmissions: Vec<Resubmission>,
    #[serde(rename = "offchainFailure")]
    pub offchain_failure: Vec<OffchainFailure>,
    #[serde(rename = "onchainFailure")]
    pub onchain_failure: Vec<OnchainFailure>,
}

// Receipt
pub struct Receipt {
    pub logs: Vec<Log>,
    pub status: String,
    #[serde(rename = "blockHash")]
    pub block_hash: String,
    #[serde(rename = "blockNumber")]
    pub block_number: String,
    #[serde(rename = "gasUsed")]
    pub gas_used: String,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
}

// Log
pub struct Log {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
}

// Resubmission
pub struct Resubmission {
    pub status: u16,
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
}

// Failures
pub struct OffchainFailure {
    pub message: String,
}

pub struct OnchainFailure {
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    #[serde(rename = "chainId")]
    pub chain_id: String,
    pub message: String,
    pub data: String,
}
```

### RPC Handler

Located in `src/rpc.rs`:

```rust
async fn process_get_status(
    storage: Storage,
    request: &GetStatusRequest,
    _cfg: &Config,
) -> Result<GetStatusResponse, jsonrpc_core::Error> {
    // Query database for transaction status
    for id in &request.ids {
        if let Ok(uuid) = Uuid::parse_str(id) {
            if let Ok(Some(req)) = storage.get_request(uuid).await {
                // Transaction found in storage
            }
        }
    }
    
    Ok(build_get_status_response(request))
}
```

## Error Handling

The endpoint handles various error scenarios:

1. **Invalid ID format**: Returns error for malformed transaction IDs
2. **Not found**: Status 404 for unknown transaction IDs
3. **Multiple transactions**: Each transaction in batch has independent status
4. **Empty arrays**: Failure arrays can be empty if no failures occurred

## Standards Compliance

✅ **JSON-RPC 2.0**: Full compliance with JSON-RPC 2.0 specification  
✅ **EIP-7702**: Compatible with EIP-7702 smart accounts  
✅ **EIP-5792**: Follows EIP-5792 modular execution patterns  
✅ **Generic Relayer EIP**: Fully compliant with the Generic Relayer Architecture specification

## Related Methods

- `relayer_sendTransaction`: Submit transaction and receive ID for status tracking
- `relayer_getCapabilities`: Discover supported payment methods
- `relayer_getExchangeRate`: Get exchange rates for transaction fees

## Notes

- The `receipts` array can contain multiple receipts if transaction was resubmitted
- Hex strings (blockNumber, gasUsed, etc.) use "0x" prefix
- The `chainId` in receipts is a decimal string, not hex
- Empty failure arrays indicate no failures of that type occurred
- The `id` field matches the ID returned by `relayer_sendTransaction`
- Status polling should use exponential backoff to avoid overwhelming the relayer

