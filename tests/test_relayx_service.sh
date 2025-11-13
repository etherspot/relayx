#!/bin/bash

# RelayX Service Test Script
# Tests all JSON-RPC endpoints using curl commands

set -e

# Configuration
RELAYX_URL="${RELAYX_URL:-http://127.0.0.1:4937}"
COLOR_GREEN='\033[0;32m'
COLOR_RED='\033[0;31m'
COLOR_YELLOW='\033[1;33m'
COLOR_BLUE='\033[0;34m'
COLOR_RESET='\033[0m'

# Helper function to print colored output
print_test() {
    echo -e "${COLOR_BLUE}▶ Testing: $1${COLOR_RESET}"
}

print_success() {
    echo -e "${COLOR_GREEN}✓ $1${COLOR_RESET}"
}

print_error() {
    echo -e "${COLOR_RED}✗ $1${COLOR_RESET}"
}

print_info() {
    echo -e "${COLOR_YELLOW}ℹ $1${COLOR_RESET}"
}

# Helper function to make JSON-RPC requests
jsonrpc_request() {
    local method=$1
    local params=$2
    local id=${3:-1}
    
    curl -s -X POST "$RELAYX_URL" \
        -H "Content-Type: application/json" \
        -d "{
            \"jsonrpc\": \"2.0\",
            \"id\": $id,
            \"method\": \"$method\",
            \"params\": $params
        }"
}

echo "=========================================="
echo "RelayX Service Test Suite"
echo "=========================================="
echo "Testing against: $RELAYX_URL"
echo ""

# Test 1: Health Check
print_test "health_check"
HEALTH_RESPONSE=$(jsonrpc_request "health_check" "null" 1)
echo "$HEALTH_RESPONSE" | jq '.' 2>/dev/null || echo "$HEALTH_RESPONSE"
if echo "$HEALTH_RESPONSE" | grep -q '"status"'; then
    print_success "Health check passed"
else
    print_error "Health check failed"
fi
echo ""

# Test 2: Get Capabilities
print_test "relayer_getCapabilities"
CAPABILITIES_RESPONSE=$(jsonrpc_request "relayer_getCapabilities" "null" 2)
echo "$CAPABILITIES_RESPONSE" | jq '.' 2>/dev/null || echo "$CAPABILITIES_RESPONSE"
if echo "$CAPABILITIES_RESPONSE" | grep -q '"result"'; then
    print_success "Get capabilities passed"
else
    print_error "Get capabilities failed"
fi
echo ""

# Test 3: Get Fee Data (with native payment)
print_test "relayer_getFeeData (native payment)"
FEE_DATA_RESPONSE=$(jsonrpc_request "relayer_getFeeData" "[{
    \"token\": \"0x036CbD53842c5426634e7929541eC2318f3dCF7e\",
    \"chainId\": \"84532\"
}]" 3)
echo "$FEE_DATA_RESPONSE" | jq '.' 2>/dev/null || echo "$FEE_DATA_RESPONSE"
if echo "$FEE_DATA_RESPONSE" | grep -q '"result"'; then
    print_success "Get fee data passed"
else
    print_error "Get fee data failed"
fi
echo ""

# Test 4: Get Exchange Rate (legacy endpoint)
print_test "relayer_getExchangeRate"
EXCHANGE_RATE_RESPONSE=$(jsonrpc_request "relayer_getExchangeRate" "[{
    \"token\": \"0x036CbD53842c5426634e7929541eC2318f3dCF7e\",
    \"chainId\": \"84532\"
}]" 4)
echo "$EXCHANGE_RATE_RESPONSE" | jq '.' 2>/dev/null || echo "$EXCHANGE_RATE_RESPONSE"
if echo "$EXCHANGE_RATE_RESPONSE" | grep -q '"result"'; then
    print_success "Get exchange rate passed"
else
    print_error "Get exchange rate failed"
fi
echo ""

# Test 5: Get Quote
print_test "relayer_getQuote"
QUOTE_RESPONSE=$(jsonrpc_request "relayer_getQuote" "[{
    \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
    \"data\": \"0xa85a325c0000000000000000000000000000000000000000000000000000000000000040\",
    \"chainId\": \"84532\"
}]" 5)
echo "$QUOTE_RESPONSE" | jq '.' 2>/dev/null || echo "$QUOTE_RESPONSE"
if echo "$QUOTE_RESPONSE" | grep -q '"result"'; then
    print_success "Get quote passed"
else
    print_error "Get quote failed"
fi
echo ""

# Test 6: Send Transaction (native payment)
print_test "relayer_sendTransaction (native payment)"
SEND_TX_RESPONSE=$(jsonrpc_request "relayer_sendTransaction" "[{
    \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
    \"data\": \"0xa85a325c0000000000000000000000000000000000000000000000000000000000000040\",
    \"chainId\": \"84532\",
    \"capabilities\": {
        \"payment\": {
            \"type\": \"native\",
            \"token\": \"\",
            \"data\": \"\"
        }
    },
    \"authorizationList\": \"\"
}]" 6)
echo "$SEND_TX_RESPONSE" | jq '.' 2>/dev/null || echo "$SEND_TX_RESPONSE"
if echo "$SEND_TX_RESPONSE" | grep -q '"result"'; then
    print_success "Send transaction (native) passed"
    # Extract transaction ID for status check
    TX_ID=$(echo "$SEND_TX_RESPONSE" | jq -r '.result[0].id' 2>/dev/null || echo "")
    if [ -n "$TX_ID" ] && [ "$TX_ID" != "null" ]; then
        print_info "Transaction ID: $TX_ID"
        TX_IDS="$TX_ID"
    fi
else
    print_error "Send transaction (native) failed"
fi
echo ""

# Test 7: Send Transaction (ERC20 payment)
print_test "relayer_sendTransaction (ERC20 payment)"
SEND_TX_ERC20_RESPONSE=$(jsonrpc_request "relayer_sendTransaction" "[{
    \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
    \"data\": \"0xa85a325c0000000000000000000000000000000000000000000000000000000000000040\",
    \"chainId\": \"84532\",
    \"capabilities\": {
        \"payment\": {
            \"type\": \"erc20\",
            \"token\": \"0x036CbD53842c5426634e7929541eC2318f3dCF7e\",
            \"data\": \"\"
        }
    },
    \"authorizationList\": \"\"
}]" 7)
echo "$SEND_TX_ERC20_RESPONSE" | jq '.' 2>/dev/null || echo "$SEND_TX_ERC20_RESPONSE"
if echo "$SEND_TX_ERC20_RESPONSE" | grep -q '"result"'; then
    print_success "Send transaction (ERC20) passed"
    # Extract transaction ID
    TX_ID_ERC20=$(echo "$SEND_TX_ERC20_RESPONSE" | jq -r '.result[0].id' 2>/dev/null || echo "")
    if [ -n "$TX_ID_ERC20" ] && [ "$TX_ID_ERC20" != "null" ]; then
        print_info "Transaction ID: $TX_ID_ERC20"
        TX_IDS="$TX_IDS $TX_ID_ERC20"
    fi
else
    print_error "Send transaction (ERC20) failed"
fi
echo ""

# Test 8: Send Transaction (sponsored payment)
print_test "relayer_sendTransaction (sponsored payment)"
SEND_TX_SPONSORED_RESPONSE=$(jsonrpc_request "relayer_sendTransaction" "[{
    \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
    \"data\": \"0xa85a325c0000000000000000000000000000000000000000000000000000000000000040\",
    \"chainId\": \"84532\",
    \"capabilities\": {
        \"payment\": {
            \"type\": \"sponsored\",
            \"token\": \"\",
            \"data\": \"\"
        }
    },
    \"authorizationList\": \"\"
}]" 8)
echo "$SEND_TX_SPONSORED_RESPONSE" | jq '.' 2>/dev/null || echo "$SEND_TX_SPONSORED_RESPONSE"
if echo "$SEND_TX_SPONSORED_RESPONSE" | grep -q '"result"'; then
    print_success "Send transaction (sponsored) passed"
    # Extract transaction ID
    TX_ID_SPONSORED=$(echo "$SEND_TX_SPONSORED_RESPONSE" | jq -r '.result[0].id' 2>/dev/null || echo "")
    if [ -n "$TX_ID_SPONSORED" ] && [ "$TX_ID_SPONSORED" != "null" ]; then
        print_info "Transaction ID: $TX_ID_SPONSORED"
        TX_IDS="$TX_IDS $TX_ID_SPONSORED"
    fi
else
    print_error "Send transaction (sponsored) failed"
fi
echo ""

# Test 9: Send Transaction Multichain (DISABLED)
# print_test "relayer_sendTransactionMultichain"
# SEND_TX_MULTI_RESPONSE=$(jsonrpc_request "relayer_sendTransactionMultichain" "[{
#     \"transactions\": [
#         {
#             \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
#             \"data\": \"0xa85a325c0000000000000000000000000000000000000000000000000000000000000040\",
#             \"chainId\": \"84532\",
#             \"authorizationList\": \"\"
#         },
#         {
#             \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
#             \"data\": \"0xa85a325c0000000000000000000000000000000000000000000000000000000000000040\",
#             \"chainId\": \"1\",
#             \"authorizationList\": \"\"
#         }
#     ],
#     \"capabilities\": {
#         \"payment\": {
#             \"type\": \"sponsored\",
#             \"token\": \"\",
#             \"data\": \"\"
#         }
#     },
#     \"paymentChainId\": \"84532\"
# }]" 9)
# echo "$SEND_TX_MULTI_RESPONSE" | jq '.' 2>/dev/null || echo "$SEND_TX_MULTI_RESPONSE"
# if echo "$SEND_TX_MULTI_RESPONSE" | grep -q '"result"'; then
#     print_success "Send transaction multichain passed"
#     # Extract transaction IDs
#     MULTI_IDS=$(echo "$SEND_TX_MULTI_RESPONSE" | jq -r '.result[].id' 2>/dev/null | tr '\n' ' ' || echo "")
#     if [ -n "$MULTI_IDS" ]; then
#         print_info "Transaction IDs: $MULTI_IDS"
#         TX_IDS="$TX_IDS $MULTI_IDS"
#     fi
# else
#     print_error "Send transaction multichain failed"
# fi
# echo ""

# Test 9: Send Transaction with EIP-7702 Delegation (delegates private key wallet to contract)
print_test "relayer_sendTransaction (EIP-7702 delegation to 0xb7a972aee9fB89aaA39F9B42C11235A45E34C95F)"
# Note: This test attempts to delegate a private key wallet to the specified contract address
# For a valid authorization, you would need a properly signed EIP-7702 authorization
# This test uses an empty authorizationList - in production, this would contain RLP-encoded signed authorizations
SEND_TX_DELEGATION_RESPONSE=$(jsonrpc_request "relayer_sendTransaction" "[{
    \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
    \"data\": \"0xa85a325c0000000000000000000000000000000000000000000000000000000000000040\",
    \"chainId\": \"84532\",
    \"capabilities\": {
        \"payment\": {
            \"type\": \"sponsored\",
            \"token\": \"\",
            \"data\": \"\"
        }
    },
    \"authorizationList\": \"\"
}]" 9)
echo "$SEND_TX_DELEGATION_RESPONSE" | jq '.' 2>/dev/null || echo "$SEND_TX_DELEGATION_RESPONSE"
if echo "$SEND_TX_DELEGATION_RESPONSE" | grep -q '"result"'; then
    print_success "Send transaction (delegation) passed"
    # Extract transaction ID
    TX_ID_DELEGATION=$(echo "$SEND_TX_DELEGATION_RESPONSE" | jq -r '.result[0].id' 2>/dev/null || echo "")
    if [ -n "$TX_ID_DELEGATION" ] && [ "$TX_ID_DELEGATION" != "null" ]; then
        print_info "Transaction ID: $TX_ID_DELEGATION"
        print_info "Delegation target: 0xb7a972aee9fB89aaA39F9B42C11235A45E34C95F"
        TX_IDS="$TX_IDS $TX_ID_DELEGATION"
    fi
else
    print_error "Send transaction (delegation) failed"
    print_info "Note: For valid EIP-7702 delegation, authorizationList must contain RLP-encoded signed authorizations delegating to 0xb7a972aee9fB89aaA39F9B42C11235A45E34C95F"
fi
echo ""

# Test 10: Get Status (if we have transaction IDs)
if [ -n "$TX_IDS" ]; then
    print_test "relayer_getStatus"
    # Clean up TX_IDS and create array
    TX_ID_ARRAY=$(echo "$TX_IDS" | tr ' ' '\n' | grep -v '^$' | jq -R . | jq -s .)
    STATUS_RESPONSE=$(jsonrpc_request "relayer_getStatus" "{\"ids\": $TX_ID_ARRAY}" 10)
    echo "$STATUS_RESPONSE" | jq '.' 2>/dev/null || echo "$STATUS_RESPONSE"
    if echo "$STATUS_RESPONSE" | grep -q '"result"'; then
        print_success "Get status passed"
    else
        print_error "Get status failed"
    fi
    echo ""
else
    print_info "Skipping getStatus test (no transaction IDs available)"
    echo ""
fi

# Test 11: Error Cases - Invalid params
print_test "relayer_sendTransaction (invalid params - missing capabilities)"
INVALID_RESPONSE=$(jsonrpc_request "relayer_sendTransaction" "[{
    \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
    \"data\": \"0xa85a325c\",
    \"chainId\": \"84532\"
}]" 11)
echo "$INVALID_RESPONSE" | jq '.' 2>/dev/null || echo "$INVALID_RESPONSE"
if echo "$INVALID_RESPONSE" | grep -q '"error"'; then
    print_success "Error handling test passed (correctly rejected invalid params)"
else
    print_error "Error handling test failed (should have returned error)"
fi
echo ""

# Test 12: Error Cases - Invalid chain ID
print_test "relayer_sendTransaction (invalid chain ID)"
INVALID_CHAIN_RESPONSE=$(jsonrpc_request "relayer_sendTransaction" "[{
    \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
    \"data\": \"0xa85a325c\",
    \"chainId\": \"99999\",
    \"capabilities\": {
        \"payment\": {
            \"type\": \"native\",
            \"token\": \"\",
            \"data\": \"\"
        }
    },
    \"authorizationList\": \"\"
}]" 12)
echo "$INVALID_CHAIN_RESPONSE" | jq '.' 2>/dev/null || echo "$INVALID_CHAIN_RESPONSE"
if echo "$INVALID_CHAIN_RESPONSE" | grep -q '"error"'; then
    print_success "Error handling test passed (correctly rejected invalid chain ID)"
else
    print_error "Error handling test failed (should have returned error)"
fi
echo ""

# Test 13: Error Cases - Unsupported payment token
print_test "relayer_sendTransaction (unsupported payment token)"
UNSUPPORTED_TOKEN_RESPONSE=$(jsonrpc_request "relayer_sendTransaction" "[{
    \"to\": \"0x52d41be4e18012eb565d5406a41627361dddc41a\",
    \"data\": \"0xa85a325c\",
    \"chainId\": \"84532\",
    \"capabilities\": {
        \"payment\": {
            \"type\": \"erc20\",
            \"token\": \"0x0000000000000000000000000000000000000000\",
            \"data\": \"\"
        }
    },
    \"authorizationList\": \"\"
}]" 13)
echo "$UNSUPPORTED_TOKEN_RESPONSE" | jq '.' 2>/dev/null || echo "$UNSUPPORTED_TOKEN_RESPONSE"
if echo "$UNSUPPORTED_TOKEN_RESPONSE" | grep -q '"error"'; then
    print_success "Error handling test passed (correctly rejected unsupported token)"
else
    print_error "Error handling test failed (should have returned error)"
fi
echo ""

echo "=========================================="
echo "Test Suite Complete"
echo "=========================================="
print_info "All tests executed. Review the output above for results."
print_info "To test against a different URL, set RELAYX_URL environment variable:"
print_info "  RELAYX_URL=http://localhost:4937 ./tests/test_relayx_service.sh"

