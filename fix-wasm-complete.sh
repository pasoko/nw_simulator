#!/bin/bash

echo "=== Complete WASM fix ==="

# Step 1: Stop containers
echo "1. Stopping containers..."
sudo docker-compose down

# Step 2: Clean everything
echo "2. Cleaning build artifacts..."
cd www
rm -rf dist
rm -rf node_modules
rm -f package-lock.json

# Step 3: Install dependencies
echo "3. Installing dependencies..."
npm install

# Step 4: Build with clean config
echo "4. Building..."
npm run build

# Step 5: Verify build
echo -e "\n5. Verifying build:"
echo "WASM files in dist/pkg:"
ls -la dist/pkg/*.wasm
echo -e "\nChecking bundle.js for WASM references:"
grep -o "\.wasm" dist/bundle.js | head -5 || echo "Direct WASM refs"

cd ..

# Step 6: Build Docker image
echo -e "\n6. Building Docker image..."
sudo docker build --no-cache -t ospf-network-simulator:latest .

# Step 7: Run container
echo "7. Starting container..."
sudo docker-compose up -d

# Step 8: Wait
echo "8. Waiting for container..."
sleep 5

# Step 9: Final test
echo -e "\n9. Testing WASM accessibility:"
# Test if pkg directory is accessible
if curl -s -o /dev/null -w "%{http_code}" "http://localhost:8080/pkg/nw_simulator_bg.wasm" | grep -q "200"; then
    echo "✓ WASM file is accessible at /pkg/nw_simulator_bg.wasm"
else
    echo "✗ WASM file is NOT accessible"
fi

echo -e "\n=== Instructions ==="
echo "1. Open http://localhost:8080"
echo "2. Open browser console (F12)"
echo "3. Look for:"
echo "   - 'Starting initialization...'"
echo "   - 'WASM initialized'"
echo "   - 'Application ready'"
echo ""
echo "If you see WASM errors, check the Network tab to see what file is being requested."

echo -e "\n=== Complete ==="