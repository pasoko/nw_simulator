#!/bin/bash

# Stop and remove existing containers
echo "Stopping existing containers..."
sudo docker-compose down

# Remove existing image to force rebuild
echo "Removing existing image..."
sudo docker rmi ospf-network-simulator:latest || true

# Build fresh image
echo "Building fresh image..."
sudo docker build -t ospf-network-simulator:latest .

# Run the container
echo "Starting container..."
sudo docker-compose up -d

echo "Rebuild complete! Access the simulator at http://localhost:8080"