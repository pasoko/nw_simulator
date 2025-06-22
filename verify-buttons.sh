#!/bin/bash

echo "=== Verifying all buttons ==="

# Check if container is running
CONTAINER_ID=$(sudo docker ps -q -f "name=nw_simulator-nw-simulator-1")

if [ -z "$CONTAINER_ID" ]; then
    echo "Container not running. Please run 'make run' first."
    exit 1
fi

echo "Checking buttons in running container..."

# Test via HTTP
echo -e "\n=== Testing via HTTP ==="
RESPONSE=$(curl -s http://localhost:8080)

echo "Checking for all buttons:"
echo "$RESPONSE" | grep -q "Add Router" && echo "✓ Add Router button found" || echo "✗ Add Router button NOT found"
echo "$RESPONSE" | grep -q "Connect Routers" && echo "✓ Connect Routers button found" || echo "✗ Connect Routers button NOT found"
echo "$RESPONSE" | grep -q "Delete Router" && echo "✓ Delete Router button found" || echo "✗ Delete Router button NOT found"
echo "$RESPONSE" | grep -q "Disconnect Routers" && echo "✓ Disconnect Routers button found" || echo "✗ Disconnect Routers button NOT found"
echo "$RESPONSE" | grep -q "Start Simulation" && echo "✓ Start Simulation button found" || echo "✗ Start Simulation button NOT found"
echo "$RESPONSE" | grep -q "Export Log" && echo "✓ Export Log button found" || echo "✗ Export Log button NOT found"
echo "$RESPONSE" | grep -q "Clear Log" && echo "✓ Clear Log button found" || echo "✗ Clear Log button NOT found"

echo -e "\nChecking for button styles:"
echo "$RESPONSE" | grep -q "button danger" && echo "✓ Danger button style found" || echo "✗ Danger button style NOT found"

echo -e "\n=== Verification complete ==="