#!/bin/bash

echo "=== Complete clean and rebuild ==="

# Step 1: Stop all containers
echo "Stopping all containers..."
sudo docker-compose down
sudo docker-compose -f docker-compose.dev.yml down

# Step 2: Remove all related images
echo "Removing all related images..."
sudo docker rmi ospf-network-simulator:latest || true
sudo docker rmi $(sudo docker images -q --filter "dangling=true") || true

# Step 3: Clean Docker build cache
echo "Cleaning Docker build cache..."
sudo docker builder prune -f

# Step 4: Remove local build artifacts
echo "Removing local build artifacts..."
rm -rf www/dist
rm -rf www/node_modules
rm -rf www/pkg
rm -rf target

# Step 5: Build everything from scratch
echo "Building from scratch..."
sudo docker build --no-cache -t ospf-network-simulator:latest .

# Step 6: Run the container
echo "Starting container..."
sudo docker-compose up -d

# Step 7: Wait for container to start
echo "Waiting for container to start..."
sleep 5

# Step 8: Check the result
echo -e "\n=== Checking result ==="
curl -s http://localhost:8080 | grep -E "(Delete Router|Disconnect Routers)" && echo -e "\n✓ Buttons found!" || echo -e "\n✗ Buttons NOT found!"

echo -e "\n=== Complete! Visit http://localhost:8080 ==="