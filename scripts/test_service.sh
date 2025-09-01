#!/bin/bash

# RelayX Service Test Script

set -e

echo "🚀 Starting RelayX Service Test"
echo "================================"

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Cargo is not installed. Please install Rust first."
    exit 1
fi

# Build the service
echo "🔨 Building RelayX service..."
cargo build --release

# Check if build was successful
if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful!"

# Create database directory if it doesn't exist
mkdir -p ./relayx_db

# Start the service in the background
echo "🚀 Starting RelayX service on localhost:8545..."
./target/release/relayx --rpc-host 127.0.0.1 --rpc-port 8545 --db-path ./relayx_db &
SERVICE_PID=$!

# Wait a moment for the service to start
echo "⏳ Waiting for service to start..."
sleep 3

# Check if service is running
if ! kill -0 $SERVICE_PID 2>/dev/null; then
    echo "❌ Service failed to start!"
    exit 1
fi

echo "✅ Service started with PID: $SERVICE_PID"

# Test the service endpoints
echo "🧪 Testing service endpoints..."

# Test 1: Health check
echo "📊 Testing health check..."
HEALTH_RESPONSE=$(curl -s -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "health_check",
    "params": null
  }')

echo "Health Response: $HEALTH_RESPONSE"

# Test 2: Submit a request
echo "📝 Testing request submission..."
SUBMIT_RESPONSE=$(curl -s -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "submit_request",
    "params": {
      "from_address": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
      "to_address": "0x742d35Cc6634C0532925a3b8D4C9db96C4b4d8b6",
      "amount": "1000000000000000000",
      "gas_limit": 21000,
      "gas_price": "20000000000",
      "data": "0x",
      "nonce": 0,
      "chain_id": 1
    }
  }')

echo "Submit Response: $SUBMIT_RESPONSE"

# Extract request ID for status check
REQUEST_ID=$(echo $SUBMIT_RESPONSE | grep -o '"[a-f0-9-]*"' | head -1 | tr -d '"')

if [ ! -z "$REQUEST_ID" ]; then
    echo "📋 Testing status check for request: $REQUEST_ID"
    
    STATUS_RESPONSE=$(curl -s -X POST http://localhost:8545 \
      -H "Content-Type: application/json" \
      -d "{
        \"jsonrpc\": \"2.0\",
        \"id\": 3,
        \"method\": \"get_request_status\",
        \"params\": \"$REQUEST_ID\"
      }")
    
    echo "Status Response: $STATUS_RESPONSE"
else
    echo "⚠️  Could not extract request ID for status check"
fi

echo ""
echo "🎉 Service test completed successfully!"
echo "Service is running with PID: $SERVICE_PID"
echo ""
echo "To stop the service, run: kill $SERVICE_PID"
echo "To view logs, run: tail -f /dev/null & echo $SERVICE_PID > /tmp/relayx.pid"
echo ""
echo "Service endpoints available at: http://localhost:8545"
