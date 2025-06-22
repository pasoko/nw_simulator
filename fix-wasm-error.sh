#!/bin/bash

echo "=== Fixing WASM loading error ==="

# Step 1: Stop containers
echo "1. Stopping containers..."
sudo docker-compose down

# Step 2: Clean and rebuild locally
echo "2. Testing local build..."
cd www
rm -rf dist
npm run build

# Check WASM files
echo -e "\n3. Checking WASM files in dist:"
ls -la dist/*.wasm 2>/dev/null || echo "No WASM files in dist root"
ls -la dist/pkg/*.wasm 2>/dev/null || echo "No WASM files in dist/pkg"

# Check for duplicate script tags
echo -e "\n4. Checking for duplicate script tags:"
grep -n "script.*bundle.js" dist/index.html

cd ..

# Step 3: Rebuild Docker image
echo -e "\n5. Building Docker image..."
sudo docker build --no-cache -t ospf-network-simulator:latest .

# Step 4: Run container
echo "6. Starting container..."
sudo docker-compose up -d

# Step 5: Wait and check
echo "7. Waiting for container..."
sleep 5

# Step 6: Test WASM loading
echo -e "\n8. Testing WASM loading:"
curl -s http://localhost:8080 > /tmp/index.html
if grep -q "bundle.js.*bundle.js" /tmp/index.html; then
    echo "✗ WARNING: Duplicate bundle.js script tags found!"
else
    echo "✓ No duplicate script tags"
fi

# Check if WASM files are accessible
echo -e "\n9. Checking WASM accessibility:"
WASM_FILES=$(ls www/dist/*.wasm 2>/dev/null | head -1)
if [ -n "$WASM_FILES" ]; then
    WASM_NAME=$(basename "$WASM_FILES")
    echo "Testing access to: $WASM_NAME"
    if curl -s -o /dev/null -w "%{http_code}" "http://localhost:8080/$WASM_NAME" | grep -q "200"; then
        echo "✓ WASM file is accessible"
    else
        echo "✗ WASM file is NOT accessible (404)"
    fi
fi

echo -e "\n=== Instructions ==="
echo "1. Open http://localhost:8080"
echo "2. Open browser console (F12)"
echo "3. Check for any WASM-related errors"
echo "4. If errors persist, check the Network tab to see what WASM file is being requested"

echo -e "\n=== Complete ==="