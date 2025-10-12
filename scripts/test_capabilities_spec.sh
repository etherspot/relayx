#!/bin/bash

# Test script to validate relayer_getCapabilities compliance with EIP specification
# Based on: https://hackmd.io/T4TkZYFQQnCupiuW231DYw?view#relayer_getCapabilities

set -e

RELAYER_URL="${RELAYER_URL:-http://localhost:4937}"

echo "Testing relayer_getCapabilities compliance..."
echo "Target: $RELAYER_URL"
echo ""

# Test 1: Basic request with no parameters
echo "Test 1: Basic request (no parameters)"
response=$(curl -s -X POST "$RELAYER_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "relayer_getCapabilities",
    "params": [],
    "id": 1
  }')

echo "Response:"
echo "$response" | jq '.'
echo ""

# Validate response structure
echo "Validating response structure..."

# Check if result exists
if ! echo "$response" | jq -e '.result' > /dev/null; then
    echo "❌ FAIL: 'result' field is missing"
    exit 1
fi
echo "✅ 'result' field exists"

# Check if capabilities exists
if ! echo "$response" | jq -e '.result.capabilities' > /dev/null; then
    echo "❌ FAIL: 'capabilities' field is missing"
    exit 1
fi
echo "✅ 'capabilities' field exists"

# Check if capabilities.payment is an array
if ! echo "$response" | jq -e '.result.capabilities.payment | type == "array"' > /dev/null; then
    echo "❌ FAIL: 'capabilities.payment' must be an array"
    exit 1
fi
echo "✅ 'capabilities.payment' is an array"

# Check if payment array has at least one item
payment_count=$(echo "$response" | jq '.result.capabilities.payment | length')
if [ "$payment_count" -lt 1 ]; then
    echo "❌ FAIL: 'capabilities.payment' array must have at least one item"
    exit 1
fi
echo "✅ 'capabilities.payment' has $payment_count item(s)"

# Validate each payment item
echo ""
echo "Validating payment items..."

for i in $(seq 0 $((payment_count - 1))); do
    payment=$(echo "$response" | jq ".result.capabilities.payment[$i]")
    payment_type=$(echo "$payment" | jq -r '.type')
    
    echo "  Payment $i:"
    echo "    Type: $payment_type"
    
    # Check if type field exists
    if [ "$payment_type" = "null" ]; then
        echo "    ❌ FAIL: Missing 'type' field"
        exit 1
    fi
    
    # Validate based on type
    case "$payment_type" in
        "native")
            token=$(echo "$payment" | jq -r '.token')
            if [ "$token" != "0x0000000000000000000000000000000000000000" ]; then
                echo "    ❌ FAIL: Native payment must have zero address (got: $token)"
                exit 1
            fi
            echo "    ✅ Native payment with zero address"
            ;;
        "erc20")
            token=$(echo "$payment" | jq -r '.token')
            if [ "$token" = "null" ]; then
                echo "    ❌ FAIL: ERC20 payment must have 'token' field"
                exit 1
            fi
            if [[ ! "$token" =~ ^0x[0-9a-fA-F]{40}$ ]]; then
                echo "    ❌ FAIL: ERC20 token address must be valid (got: $token)"
                exit 1
            fi
            echo "    ✅ ERC20 payment with token: $token"
            ;;
        "sponsored")
            # Check that token field does NOT exist
            has_token=$(echo "$payment" | jq 'has("token")')
            if [ "$has_token" = "true" ]; then
                echo "    ❌ FAIL: Sponsored payment should NOT have 'token' field"
                exit 1
            fi
            echo "    ✅ Sponsored payment (no token field)"
            ;;
        *)
            echo "    ❌ FAIL: Unknown payment type: $payment_type"
            exit 1
            ;;
    esac
done

echo ""
echo "Testing payment type requirements..."

# Check for at least one valid payment type
has_native=$(echo "$response" | jq '.result.capabilities.payment | map(select(.type == "native")) | length > 0')
has_erc20=$(echo "$response" | jq '.result.capabilities.payment | map(select(.type == "erc20")) | length > 0')
has_sponsored=$(echo "$response" | jq '.result.capabilities.payment | map(select(.type == "sponsored")) | length > 0')

echo "  Native payment available: $has_native"
echo "  ERC20 payment available: $has_erc20"
echo "  Sponsored payment available: $has_sponsored"

if [ "$has_native" = "false" ] && [ "$has_erc20" = "false" ] && [ "$has_sponsored" = "false" ]; then
    echo "❌ FAIL: At least one payment type must be available"
    exit 1
fi
echo "✅ At least one payment type is available"

# Test 2: Verify JSON-RPC 2.0 format
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

# Test 3: Verify no unexpected fields
echo ""
echo "Test 3: Checking for unexpected fields"

# Check that response has only expected top-level fields
top_level_keys=$(echo "$response" | jq -r 'keys | @json')
expected_keys='["id","jsonrpc","result"]'

if [ "$top_level_keys" != "$expected_keys" ]; then
    echo "⚠️  WARNING: Response has unexpected top-level fields"
    echo "    Expected: $expected_keys"
    echo "    Got: $top_level_keys"
else
    echo "✅ Response has only expected top-level fields"
fi

# Test 4: Validate token addresses are unique for erc20
echo ""
echo "Test 4: Checking for duplicate token addresses"

erc20_tokens=$(echo "$response" | jq -r '.result.capabilities.payment | map(select(.type == "erc20") | .token) | unique | length')
total_erc20=$(echo "$response" | jq -r '.result.capabilities.payment | map(select(.type == "erc20")) | length')

if [ "$erc20_tokens" != "$total_erc20" ]; then
    echo "⚠️  WARNING: Found duplicate ERC20 token addresses"
else
    echo "✅ No duplicate ERC20 token addresses"
fi

echo ""
echo "=========================================="
echo "✅ All tests passed!"
echo "=========================================="
echo ""
echo "relayer_getCapabilities implementation is compliant with the specification"
echo ""
echo "Summary:"
echo "  - Total payment options: $payment_count"
echo "  - Native payment: $has_native"
echo "  - ERC20 payment: $has_erc20"  
echo "  - Sponsored payment: $has_sponsored"

