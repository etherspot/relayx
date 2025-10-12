#!/bin/bash

# Test script to validate relayer_sendTransactionMultichain compliance with EIP specification
# Based on: https://hackmd.io/T4TkZYFQQnCupiuW231DYw?view#relayer_sendTransactionMultichain

set -e

RELAYER_URL="${RELAYER_URL:-http://localhost:4937}"

echo "Testing relayer_sendTransactionMultichain compliance..."
echo "Target: $RELAYER_URL"
echo ""

# Test 1: Multi-chain sponsored transaction
echo "Test 1: Multi-chain sponsored transaction (3 chains)"
response=$(curl -s -X POST "$RELAYER_URL" \
  -H "Content-Type: application/json" \
  -d '{
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
          "type": "sponsored",
          "token": "",
          "data": ""
        }
      },
      "paymentChainId": "1"
    }],
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

# Check if result has correct number of items (should match transactions count)
result_count=$(echo "$response" | jq '.result | length')
if [ "$result_count" -ne 3 ]; then
    echo "❌ FAIL: 'result' array must have 3 items (one per transaction, got: $result_count)"
    exit 1
fi
echo "✅ 'result' array has $result_count items (matches transaction count)"

# Validate each result
echo ""
echo "Validating individual results..."

for i in {0..2}; do
    echo "  Result $i:"
    
    # Check chainId field
    if ! echo "$response" | jq -e ".result[$i].chainId" > /dev/null; then
        echo "  ❌ FAIL: Missing 'chainId' field in result $i"
        exit 1
    fi
    echo "  ✅ 'chainId' field exists"
    
    # Check id field
    if ! echo "$response" | jq -e ".result[$i].id" > /dev/null; then
        echo "  ❌ FAIL: Missing 'id' field in result $i"
        exit 1
    fi
    
    tx_id=$(echo "$response" | jq -r ".result[$i].id")
    if [ -z "$tx_id" ]; then
        echo "  ❌ FAIL: Transaction ID is empty"
        exit 1
    fi
    echo "  ✅ Transaction ID generated: ${tx_id:0:20}..."
    
    # Verify chainId matches request
    result_chain=$(echo "$response" | jq -r ".result[$i].chainId")
    expected_chains=("1" "137" "8453")
    expected_chain="${expected_chains[$i]}"
    
    if [ "$result_chain" != "$expected_chain" ]; then
        echo "  ❌ FAIL: chainId mismatch (expected: $expected_chain, got: $result_chain)"
        exit 1
    fi
    echo "  ✅ chainId matches request: $result_chain"
done

# Test 2: ERC20 payment multichain
echo ""
echo "Test 2: Multi-chain ERC20 payment (2 chains)"
response2=$(curl -s -X POST "$RELAYER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_sendTransactionMultichain",
    "params": [{
      "transactions": [
        {
          "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
          "data": "0x1234",
          "chainId": "1",
          "authorizationList": ""
        },
        {
          "to": "0x8922b54716264130634d6ff183747a8ead91a40c",
          "data": "0x5678",
          "chainId": "10",
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
    "id": 2
  }')

echo "Response:"
echo "$response2" | jq '.'
echo ""

result2_count=$(echo "$response2" | jq '.result | length')
if [ "$result2_count" -ne 2 ]; then
    echo "❌ FAIL: Expected 2 results, got $result2_count"
    exit 1
fi
echo "✅ ERC20 multichain transaction accepted with $result2_count results"

# Test 3: Validation - Empty transactions array
echo ""
echo "Test 3: Validation - Empty transactions array (should fail)"
response3=$(curl -s -X POST "$RELAYER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_sendTransactionMultichain",
    "params": [{
      "transactions": [],
      "capabilities": {
        "payment": {
          "type": "sponsored",
          "token": "",
          "data": ""
        }
      },
      "paymentChainId": "1"
    }],
    "id": 3
  }')

if echo "$response3" | jq -e '.error' > /dev/null; then
    error_msg=$(echo "$response3" | jq -r '.error.message')
    echo "✅ Validation error for empty transactions: $error_msg"
else
    echo "❌ FAIL: Should reject empty transactions array"
    exit 1
fi

# Test 4: Validation - Invalid payment chain
echo ""
echo "Test 4: Validation - Unsupported payment chain (should fail)"
response4=$(curl -s -X POST "$RELAYER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_sendTransactionMultichain",
    "params": [{
      "transactions": [
        {
          "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
          "data": "0x1234",
          "chainId": "1",
          "authorizationList": ""
        }
      ],
      "capabilities": {
        "payment": {
          "type": "sponsored",
          "token": "",
          "data": ""
        }
      },
      "paymentChainId": "999999"
    }],
    "id": 4
  }')

if echo "$response4" | jq -e '.error' > /dev/null; then
    error_msg=$(echo "$response4" | jq -r '.error.message')
    echo "✅ Validation error for unsupported payment chain: $error_msg"
else
    echo "❌ FAIL: Should reject unsupported payment chain"
    exit 1
fi

# Test 5: Validation - Transaction with invalid chain
echo ""
echo "Test 5: Validation - Transaction with unsupported chain (should fail)"
response5=$(curl -s -X POST "$RELAYER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_sendTransactionMultichain",
    "params": [{
      "transactions": [
        {
          "to": "0x742d35Cc6C3C3f4b4C1b3cd6c0d1b6C2B3d4e5f6",
          "data": "0x1234",
          "chainId": "1",
          "authorizationList": ""
        },
        {
          "to": "0x8922b54716264130634d6ff183747a8ead91a40c",
          "data": "0x5678",
          "chainId": "999999",
          "authorizationList": ""
        }
      ],
      "capabilities": {
        "payment": {
          "type": "sponsored",
          "token": "",
          "data": ""
        }
      },
      "paymentChainId": "1"
    }],
    "id": 5
  }')

if echo "$response5" | jq -e '.error' > /dev/null; then
    error_msg=$(echo "$response5" | jq -r '.error.message')
    echo "✅ Validation error for unsupported transaction chain: $error_msg"
else
    echo "❌ FAIL: Should reject transaction with unsupported chain"
    exit 1
fi

# Test 6: JSON-RPC 2.0 format validation
echo ""
echo "Test 6: JSON-RPC 2.0 format validation"

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
echo "relayer_sendTransactionMultichain implementation is compliant with the specification"
echo ""
echo "Summary:"
echo "  - Multi-chain requests: ✅ Working"
echo "  - Response format: ✅ Correct (one result per transaction)"
echo "  - ChainId matching: ✅ Verified"
echo "  - Payment chain validation: ✅ Working"
echo "  - Transaction validation: ✅ Per-transaction validation"
echo "  - Error handling: ✅ Proper validation errors"
echo "  - JSON-RPC 2.0: ✅ Compliant"

