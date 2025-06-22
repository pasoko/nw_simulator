#!/bin/bash

echo "=== Debug build ==="

# Stop existing containers
sudo docker-compose down

# Build with debug Dockerfile
echo "Building with debug output..."
sudo docker build --no-cache -f Dockerfile.debug -t ospf-network-simulator:debug .

# Tag as latest
sudo docker tag ospf-network-simulator:debug ospf-network-simulator:latest

# Run container
sudo docker-compose up -d

echo "=== Build complete. Check the output above for button presence ==="