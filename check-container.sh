#!/bin/bash

echo "=== Checking Docker container status ==="

# Check if Docker is accessible
if ! docker ps >/dev/null 2>&1; then
    echo "Docker requires sudo. Checking with sudo..."
    
    if sudo docker ps >/dev/null 2>&1; then
        echo "Docker is accessible with sudo."
        DOCKER_CMD="sudo docker"
    else
        echo "ERROR: Cannot access Docker. Please make sure Docker is running."
        exit 1
    fi
else
    echo "Docker is accessible without sudo."
    DOCKER_CMD="docker"
fi

# Check running containers
echo -e "\n=== Running containers ==="
$DOCKER_CMD ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

# Check if our container is running
if $DOCKER_CMD ps | grep -q "nw_simulator"; then
    echo -e "\n✓ OSPF Network Simulator container is running"
    
    # Test HTTP access
    echo -e "\n=== Testing HTTP access ==="
    if curl -s -o /dev/null -w "%{http_code}" http://localhost:8080 | grep -q "200"; then
        echo "✓ Web server is responding (HTTP 200)"
    else
        echo "✗ Web server is not responding properly"
    fi
else
    echo -e "\n✗ OSPF Network Simulator container is NOT running"
    echo "Please run 'make run' first"
fi