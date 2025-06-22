#!/bin/bash

echo "=== Debugging Docker container ==="

# Check if container is running
echo -e "\n1. Checking running containers:"
sudo docker ps | grep ospf-network-simulator

# Get container ID
CONTAINER_ID=$(sudo docker ps -q -f "name=nw_simulator-nw-simulator-1")

if [ -z "$CONTAINER_ID" ]; then
    echo "Container not found. Please run 'make run' first."
    exit 1
fi

echo -e "\n2. Container ID: $CONTAINER_ID"

# Check the content of index.html in the container
echo -e "\n3. Checking index.html content in container:"
echo "Looking for Delete Router and Disconnect Routers buttons..."
sudo docker exec $CONTAINER_ID cat /usr/share/nginx/html/index.html | grep -E "(Delete Router|Disconnect Routers|delete-router-btn|disconnect-routers-btn)" || echo "Buttons not found in index.html"

# List files in the container
echo -e "\n4. Files in /usr/share/nginx/html:"
sudo docker exec $CONTAINER_ID ls -la /usr/share/nginx/html/

# Check if the actual file is being served
echo -e "\n5. Testing HTTP response:"
curl -s http://localhost:8080 | grep -E "(Delete Router|Disconnect Routers|delete-router-btn|disconnect-routers-btn)" || echo "Buttons not found in HTTP response"

echo -e "\n=== Debug complete ==="