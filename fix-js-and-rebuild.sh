#!/bin/bash

echo "=== Fixing JavaScript issues and rebuilding ==="

# Step 1: Stop containers
echo "1. Stopping containers..."
sudo docker-compose down

# Step 2: Test local build first
echo "2. Testing local build..."
cd www
rm -rf dist
npm run build

# Check if PacketVisualizer is in bundle
if grep -q "PacketVisualizer" dist/bundle.js; then
    echo "✓ PacketVisualizer is bundled correctly"
else
    echo "✗ PacketVisualizer is NOT bundled - this is the problem!"
fi

cd ..

# Step 3: Rebuild Docker image
echo "3. Rebuilding Docker image..."
sudo docker build --no-cache -t ospf-network-simulator:latest .

# Step 4: Run container
echo "4. Starting container..."
sudo docker-compose up -d

# Step 5: Wait for container
echo "5. Waiting for container to start..."
sleep 5

# Step 6: Test buttons
echo -e "\n6. Testing application..."
echo "Please open:"
echo "- http://localhost:8080 - Main application"
echo "- http://localhost:8080/debug.html - Debug page (if available)"

echo -e "\n=== Complete ==="