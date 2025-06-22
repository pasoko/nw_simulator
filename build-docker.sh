#!/bin/bash

echo "Building OSPF Network Simulator Docker image..."

# Build the Docker image
docker build -t ospf-network-simulator:latest .

if [ $? -eq 0 ]; then
    echo "Build successful!"
    echo ""
    echo "To run the simulator:"
    echo "  docker run -p 8080:80 ospf-network-simulator:latest"
    echo ""
    echo "Or use Docker Compose:"
    echo "  docker-compose up"
    echo ""
    echo "Then access the simulator at http://localhost:8080"
else
    echo "Build failed!"
    exit 1
fi