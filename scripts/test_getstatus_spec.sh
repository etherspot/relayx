#!/bin/bash

# Test script to validate relayer_getStatus compliance with EIP specification
# Based on: https://hackmd.io/T4TkZYFQQnCupiuW231DYw?view#relayer_getStatus

set -e

RELAYER_URL="${RELAYER_URL:-http://localhost:4937}"

echo "Testing relayer_getStatus compliance..."
echo "Target: $RELAYER_URL"
echo ""

# Test 1: Basic status request
echo "Test 1: Basic status request with single ID"
response=$(curl -s -X POST "$RELAYER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getStatus",
    "params": {
      "ids": ["0x00000000000000000000000000000000000000000000000000000000000000000e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331"]
    },
    "id": 1
  }')

echo "Response:"
echo "$response" | jq '.'
echo ""

# Validate response structure
echo "Validating response structure..."

# Check if result exists and is an array
if ! echo "$response" | jq -e '.result | type == "array"' > /dev/null; then
    echo "❌ FAIL: 'result' must be an array"
    exit 1
fi
echo "✅ 'result' is an array"

# Check if result has at least one item
if ! echo "$response" | jq -e '.result | length >= 1' > /dev/null; then
    echo "❌ FAIL: 'result' array must have at least one item"
    exit 1
fi
echo "✅ 'result' array has items"

# Validate first status result
echo ""
echo "Validating status result fields..."

# Check version field
if ! echo "$response" | jq -e '.result[0].version' > /dev/null; then
    echo "❌ FAIL: Missing 'version' field"
    exit 1
fi
echo "✅ 'version' field exists"

# Check id field
if ! echo "$response" | jq -e '.result[0].id' > /dev/null; then
    echo "❌ FAIL: Missing 'id' field"
    exit 1
fi
echo "✅ 'id' field exists"

# Check status field (must be a number)
if ! echo "$response" | jq -e '.result[0].status | type == "number"' > /dev/null; then
    echo "❌ FAIL: 'status' field must be a number"
    exit 1
fi
status_code=$(echo "$response" | jq -r '.result[0].status')
echo "✅ 'status' field exists (value: $status_code)"

# Validate status code is valid HTTP-style code
if [ "$status_code" -lt 100 ] || [ "$status_code" -gt 599 ]; then
    echo "❌ FAIL: 'status' must be valid HTTP status code (100-599)"
    exit 1
fi
echo "✅ 'status' is a valid HTTP status code"

# Check receipts array
if ! echo "$response" | jq -e '.result[0].receipts | type == "array"' > /dev/null; then
    echo "❌ FAIL: 'receipts' must be an array"
    exit 1
fi
echo "✅ 'receipts' is an array"

# Check resubmissions array
if ! echo "$response" | jq -e '.result[0].resubmissions | type == "array"' > /dev/null; then
    echo "❌ FAIL: 'resubmissions' must be an array"
    exit 1
fi
echo "✅ 'resubmissions' is an array"

# Check offchainFailure array
if ! echo "$response" | jq -e '.result[0].offchainFailure | type == "array"' > /dev/null; then
    echo "❌ FAIL: 'offchainFailure' must be an array"
    exit 1
fi
echo "✅ 'offchainFailure' is an array"

# Check onchainFailure array
if ! echo "$response" | jq -e '.result[0].onchainFailure | type == "array"' > /dev/null; then
    echo "❌ FAIL: 'onchainFailure' must be an array"
    exit 1
fi
echo "✅ 'onchainFailure' is an array"

# Validate receipts structure (if any exist)
echo ""
echo "Validating receipts structure..."
receipt_count=$(echo "$response" | jq '.result[0].receipts | length')
echo "Found $receipt_count receipt(s)"

if [ "$receipt_count" -gt 0 ]; then
    # Check receipt fields
    if ! echo "$response" | jq -e '.result[0].receipts[0].logs' > /dev/null; then
        echo "❌ FAIL: Receipt missing 'logs' field"
        exit 1
    fi
    echo "✅ Receipt has 'logs' field"
    
    if ! echo "$response" | jq -e '.result[0].receipts[0].status' > /dev/null; then
        echo "❌ FAIL: Receipt missing 'status' field"
        exit 1
    fi
    echo "✅ Receipt has 'status' field"
    
    if ! echo "$response" | jq -e '.result[0].receipts[0].blockHash' > /dev/null; then
        echo "❌ FAIL: Receipt missing 'blockHash' field"
        exit 1
    fi
    echo "✅ Receipt has 'blockHash' field"
    
    if ! echo "$response" | jq -e '.result[0].receipts[0].blockNumber' > /dev/null; then
        echo "❌ FAIL: Receipt missing 'blockNumber' field"
        exit 1
    fi
    echo "✅ Receipt has 'blockNumber' field"
    
    if ! echo "$response" | jq -e '.result[0].receipts[0].gasUsed' > /dev/null; then
        echo "❌ FAIL: Receipt missing 'gasUsed' field"
        exit 1
    fi
    echo "✅ Receipt has 'gasUsed' field"
    
    if ! echo "$response" | jq -e '.result[0].receipts[0].transactionHash' > /dev/null; then
        echo "❌ FAIL: Receipt missing 'transactionHash' field"
        exit 1
    fi
    echo "✅ Receipt has 'transactionHash' field"
    
    if ! echo "$response" | jq -e '.result[0].receipts[0].chainId' > /dev/null; then
        echo "❌ FAIL: Receipt missing 'chainId' field"
        exit 1
    fi
    echo "✅ Receipt has 'chainId' field"
    
    # Validate hex strings in receipt
    block_hash=$(echo "$response" | jq -r '.result[0].receipts[0].blockHash')
    if [[ ! "$block_hash" =~ ^0x[0-9a-fA-F]+$ ]]; then
        echo "❌ FAIL: 'blockHash' must be a hex string (got: $block_hash)"
        exit 1
    fi
    echo "✅ 'blockHash' is a valid hex string"
    
    # Validate logs structure (if any exist)
    log_count=$(echo "$response" | jq '.result[0].receipts[0].logs | length')
    echo "Found $log_count log(s) in receipt"
    
    if [ "$log_count" -gt 0 ]; then
        if ! echo "$response" | jq -e '.result[0].receipts[0].logs[0].address' > /dev/null; then
            echo "❌ FAIL: Log missing 'address' field"
            exit 1
        fi
        echo "✅ Log has 'address' field"
        
        if ! echo "$response" | jq -e '.result[0].receipts[0].logs[0].topics | type == "array"' > /dev/null; then
            echo "❌ FAIL: Log 'topics' must be an array"
            exit 1
        fi
        echo "✅ Log has 'topics' array"
        
        if ! echo "$response" | jq -e '.result[0].receipts[0].logs[0].data' > /dev/null; then
            echo "❌ FAIL: Log missing 'data' field"
            exit 1
        fi
        echo "✅ Log has 'data' field"
    fi
fi

# Validate resubmissions structure (if any exist)
echo ""
echo "Validating resubmissions structure..."
resubmission_count=$(echo "$response" | jq '.result[0].resubmissions | length')
echo "Found $resubmission_count resubmission(s)"

if [ "$resubmission_count" -gt 0 ]; then
    if ! echo "$response" | jq -e '.result[0].resubmissions[0].status | type == "number"' > /dev/null; then
        echo "❌ FAIL: Resubmission 'status' must be a number"
        exit 1
    fi
    echo "✅ Resubmission has 'status' field"
    
    if ! echo "$response" | jq -e '.result[0].resubmissions[0].transactionHash' > /dev/null; then
        echo "❌ FAIL: Resubmission missing 'transactionHash' field"
        exit 1
    fi
    echo "✅ Resubmission has 'transactionHash' field"
    
    if ! echo "$response" | jq -e '.result[0].resubmissions[0].chainId' > /dev/null; then
        echo "❌ FAIL: Resubmission missing 'chainId' field"
        exit 1
    fi
    echo "✅ Resubmission has 'chainId' field"
fi

# Validate offchainFailure structure (if any exist)
echo ""
echo "Validating offchainFailure structure..."
offchain_failure_count=$(echo "$response" | jq '.result[0].offchainFailure | length')
echo "Found $offchain_failure_count offchain failure(s)"

if [ "$offchain_failure_count" -gt 0 ]; then
    if ! echo "$response" | jq -e '.result[0].offchainFailure[0].message' > /dev/null; then
        echo "❌ FAIL: OffchainFailure missing 'message' field"
        exit 1
    fi
    echo "✅ OffchainFailure has 'message' field"
fi

# Validate onchainFailure structure (if any exist)
echo ""
echo "Validating onchainFailure structure..."
onchain_failure_count=$(echo "$response" | jq '.result[0].onchainFailure | length')
echo "Found $onchain_failure_count onchain failure(s)"

if [ "$onchain_failure_count" -gt 0 ]; then
    if ! echo "$response" | jq -e '.result[0].onchainFailure[0].transactionHash' > /dev/null; then
        echo "❌ FAIL: OnchainFailure missing 'transactionHash' field"
        exit 1
    fi
    echo "✅ OnchainFailure has 'transactionHash' field"
    
    if ! echo "$response" | jq -e '.result[0].onchainFailure[0].chainId' > /dev/null; then
        echo "❌ FAIL: OnchainFailure missing 'chainId' field"
        exit 1
    fi
    echo "✅ OnchainFailure has 'chainId' field"
    
    if ! echo "$response" | jq -e '.result[0].onchainFailure[0].message' > /dev/null; then
        echo "❌ FAIL: OnchainFailure missing 'message' field"
        exit 1
    fi
    echo "✅ OnchainFailure has 'message' field"
    
    if ! echo "$response" | jq -e '.result[0].onchainFailure[0].data' > /dev/null; then
        echo "❌ FAIL: OnchainFailure missing 'data' field"
        exit 1
    fi
    echo "✅ OnchainFailure has 'data' field"
fi

# Test 2: JSON-RPC 2.0 format validation
echo ""
echo "Test 2: JSON-RPC 2.0 format validation"

if ! echo "$response" | jq -e '.jsonrpc == "2.0"' > /dev/null; then
    echo "❌ FAIL: Response must have 'jsonrpc': '2.0'"
    exit 1
fi
echo "✅ JSON-RPC version is 2.0"

if ! echo "$response" | jq -e '.id == 1' > /dev/null; then
    echo "❌ FAIL: Response must have matching 'id'"
    exit 1
fi
echo "✅ Response ID matches request"

echo ""
echo "=========================================="
echo "✅ All tests passed!"
echo "=========================================="
echo ""
echo "relayer_getStatus implementation is compliant with the specification"
echo ""
echo "Summary:"
echo "  - Status code: $status_code"
echo "  - Receipts: $receipt_count"
echo "  - Resubmissions: $resubmission_count"
echo "  - Offchain failures: $offchain_failure_count"
echo "  - Onchain failures: $onchain_failure_count"

