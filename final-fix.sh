#!/bin/bash

echo "=== Final fix for button functionality ==="

# Step 1: Stop containers
echo "1. Stopping containers..."
sudo docker-compose down

# Step 2: Remove old images
echo "2. Removing old images..."
sudo docker rmi ospf-network-simulator:latest || true

# Step 3: Rebuild with fixed nginx.conf
echo "3. Building Docker image with fixed nginx.conf..."
sudo docker build --no-cache -t ospf-network-simulator:latest .

# Step 4: Run container
echo "4. Starting container..."
sudo docker-compose up -d

# Step 5: Wait for container
echo "5. Waiting for container to start..."
sleep 5

# Step 6: Test functionality
echo -e "\n=== Testing functionality ==="
echo "Please open http://localhost:8080 and test the buttons"
echo ""
echo "If buttons still don't work, also try:"
echo "1. http://localhost:8080/test.html - Test page for debugging"
echo "2. Clear browser cache (Ctrl+Shift+R)"
echo "3. Check browser console for any errors (F12)"

echo -e "\n=== Complete ==="