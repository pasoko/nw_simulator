#!/bin/bash

echo "=== Checking files in Docker container ==="

# Get container ID
CONTAINER_ID=$(sudo docker ps -q -f "name=nw_simulator-nw-simulator-1")

if [ -z "$CONTAINER_ID" ]; then
    echo "Container not found. Please run 'make run' first."
    exit 1
fi

echo "Container ID: $CONTAINER_ID"

# Check WASM files in container
echo -e "\n1. WASM files in container root:"
sudo docker exec $CONTAINER_ID ls -la /usr/share/nginx/html/*.wasm 2>/dev/null || echo "No WASM files in root"

echo -e "\n2. WASM files in container pkg directory:"
sudo docker exec $CONTAINER_ID ls -la /usr/share/nginx/html/pkg/*.wasm 2>/dev/null || echo "No WASM files in pkg"

echo -e "\n3. All files in container root:"
sudo docker exec $CONTAINER_ID ls -la /usr/share/nginx/html/ | grep -E "(wasm|bundle.js|index.html)"

echo -e "\n4. Check bundle.js for WASM references:"
sudo docker exec $CONTAINER_ID grep -o '[a-f0-9]\{20\}\.wasm' /usr/share/nginx/html/bundle.js | head -5 || echo "No WASM references found"

echo -e "\n5. Check if bundle.js exists and its size:"
sudo docker exec $CONTAINER_ID ls -lh /usr/share/nginx/html/bundle.js

echo -e "\n6. First 100 chars of bundle.js:"
sudo docker exec $CONTAINER_ID head -c 100 /usr/share/nginx/html/bundle.js

echo -e "\n=== Complete ==="