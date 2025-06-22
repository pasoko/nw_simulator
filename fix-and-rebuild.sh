#!/bin/bash

echo "=== Fixing and rebuilding ==="

# Step 1: Stop all containers
echo "1. Stopping containers..."
sudo docker-compose down

# Step 2: Remove old image
echo "2. Removing old image..."
sudo docker rmi ospf-network-simulator:latest || true

# Step 3: Clean local build artifacts
echo "3. Cleaning local artifacts..."
rm -rf www/dist
rm -rf www/node_modules
rm -rf www/package-lock.json

# Step 4: Build everything fresh
echo "4. Building fresh Docker image..."
sudo docker build --no-cache -t ospf-network-simulator:latest .

# Step 5: Run the container
echo "5. Starting container..."
sudo docker-compose up -d

# Step 6: Wait and verify
echo "6. Waiting for container to start..."
sleep 5

# Step 7: Check if buttons are present
echo -e "\n=== Verifying buttons ==="
curl -s http://localhost:8080 | grep -q "Delete Router" && echo "✓ Delete Router button found!" || echo "✗ Delete Router button NOT found!"
curl -s http://localhost:8080 | grep -q "Disconnect Routers" && echo "✓ Disconnect Routers button found!" || echo "✗ Disconnect Routers button NOT found!"

echo -e "\n=== Complete! Visit http://localhost:8080 ==="