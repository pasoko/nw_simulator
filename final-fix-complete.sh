#!/bin/bash

echo "=== Complete fix for button functionality ==="

# Step 1: Stop containers
echo "1. Stopping containers..."
sudo docker-compose down

# Step 2: Clean everything
echo "2. Cleaning old build artifacts..."
cd www
rm -rf dist
rm -rf node_modules
rm -f package-lock.json
cd ..

# Step 3: Rebuild locally first to test
echo "3. Testing local build..."
cd www
npm install
npm run build

# Check if build was successful
if [ -f dist/bundle.js ]; then
    echo "✓ bundle.js created"
else
    echo "✗ bundle.js not created - build failed!"
    exit 1
fi

cd ..

# Step 4: Build Docker image
echo "4. Building Docker image..."
sudo docker build --no-cache -t ospf-network-simulator:latest .

# Step 5: Run container
echo "5. Starting container..."
sudo docker-compose up -d

# Step 6: Wait for container
echo "6. Waiting for container to start..."
sleep 5

# Step 7: Instructions
echo -e "\n=== Testing Instructions ==="
echo "1. Main application: http://localhost:8080"
echo "   - Open browser console (F12) to see debug logs"
echo "   - Try clicking buttons and check console for messages"
echo ""
echo "2. Standalone test: http://localhost:8080/index-standalone.html"
echo "   - This version has simple alerts to test button clicks"
echo ""
echo "3. Simple test: http://localhost:8080/simple-test.html"
echo "   - Tests basic JavaScript and WASM loading"
echo ""
echo "If buttons still don't work:"
echo "- Clear browser cache (Ctrl+Shift+R)"
echo "- Check browser console for error messages"
echo "- Try a different browser"

echo -e "\n=== Complete ==="