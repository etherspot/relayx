#!/bin/bash

# Test script to validate relayer_getExchangeRate compliance with EIP specification
# Based on: https://hackmd.io/T4TkZYFQQnCupiuW231DYw?view#relayer_getExchangeRate

set -e

RELAYER_URL="${RELAYER_URL:-http://localhost:4937}"

echo "Testing relayer_getExchangeRate compliance..."
echo "Target: $RELAYER_URL"
echo ""

# Test 1: Native token exchange rate
echo "Test 1: Native token (ETH) exchange rate"
response=$(curl -s -X POST "$RELAYER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getExchangeRate",
    "params": [{
      "token": "0x0000000000000000000000000000000000000000",
      "chainId": "1"
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
echo "✅ result is an array"

# Check if result has at least one item
if ! echo "$response" | jq -e '.result | length >= 1' > /dev/null; then
    echo "❌ FAIL: 'result' array must have at least one item"
    exit 1
fi
echo "✅ result array has items"

# Check if first result item has required fields
if ! echo "$response" | jq -e '.result[0].quote' > /dev/null; then
    echo "❌ FAIL: Missing 'quote' field"
    exit 1
fi
echo "✅ 'quote' field exists"

if ! echo "$response" | jq -e '.result[0].quote.rate' > /dev/null; then
    echo "❌ FAIL: Missing 'quote.rate' field"
    exit 1
fi
echo "✅ 'quote.rate' field exists"

if ! echo "$response" | jq -e '.result[0].quote.token' > /dev/null; then
    echo "❌ FAIL: Missing 'quote.token' field"
    exit 1
fi
echo "✅ 'quote.token' field exists"

if ! echo "$response" | jq -e '.result[0].quote.token.decimals' > /dev/null; then
    echo "❌ FAIL: Missing 'quote.token.decimals' field"
    exit 1
fi
echo "✅ 'quote.token.decimals' field exists"

if ! echo "$response" | jq -e '.result[0].quote.token.address' > /dev/null; then
    echo "❌ FAIL: Missing 'quote.token.address' field"
    exit 1
fi
echo "✅ 'quote.token.address' field exists"

if ! echo "$response" | jq -e '.result[0].feeCollector' > /dev/null; then
    echo "❌ FAIL: Missing 'feeCollector' field"
    exit 1
fi
echo "✅ 'feeCollector' field exists"

if ! echo "$response" | jq -e '.result[0].expiry' > /dev/null; then
    echo "❌ FAIL: Missing 'expiry' field"
    exit 1
fi
echo "✅ 'expiry' field exists"

if ! echo "$response" | jq -e '.result[0].gasPrice' > /dev/null; then
    echo "❌ FAIL: Missing 'gasPrice' field"
    exit 1
fi
echo "✅ 'gasPrice' field exists"

# Check that gasPrice is a hex string
gasPrice=$(echo "$response" | jq -r '.result[0].gasPrice')
if [[ ! "$gasPrice" =~ ^0x[0-9a-fA-F]+$ ]]; then
    echo "❌ FAIL: 'gasPrice' must be a hex string (got: $gasPrice)"
    exit 1
fi
echo "✅ 'gasPrice' is a valid hex string"

# Check that expiry is a number and in the future
expiry=$(echo "$response" | jq -r '.result[0].expiry')
current_time=$(date +%s)
if [ "$expiry" -le "$current_time" ]; then
    echo "❌ FAIL: 'expiry' must be in the future (expiry: $expiry, current: $current_time)"
    exit 1
fi
echo "✅ 'expiry' is in the future"

echo ""
echo "Test 2: ERC20 token (USDC) exchange rate"
response2=$(curl -s -X POST "$RELAYER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getExchangeRate",
    "params": [{
      "token": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
      "chainId": "1"
    }],
    "id": 2
  }')

echo "Response:"
echo "$response2" | jq '.'
echo ""

# Validate ERC20 response
if ! echo "$response2" | jq -e '.result[0].quote.token.address == "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"' > /dev/null; then
    echo "❌ FAIL: Token address doesn't match request"
    exit 1
fi
echo "✅ Token address matches request"

echo ""
echo "Test 3: Validate chainId in different formats"
# Test with different chain IDs
for chainId in "1" "137" "8453"; do
    echo "Testing chainId: $chainId"
    response3=$(curl -s -X POST "$RELAYER_URL" \
      -H "Content-Type: application/json" \
      -d "{
        \"jsonrpc\": \"2.0\",
        \"method\": \"relayer_getExchangeRate\",
        \"params\": [{
          \"token\": \"0x0000000000000000000000000000000000000000\",
          \"chainId\": \"$chainId\"
        }],
        \"id\": 3
      }")
    
    if ! echo "$response3" | jq -e '.result' > /dev/null; then
        echo "❌ FAIL: Request failed for chainId $chainId"
        exit 1
    fi
    echo "✅ chainId $chainId works"
done

echo ""
echo "=========================================="
echo "✅ All tests passed!"
echo "=========================================="
echo ""
echo "relayer_getExchangeRate implementation is compliant with the specification"

