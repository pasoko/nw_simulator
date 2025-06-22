#!/bin/bash

echo "=== Quick fix and rebuild ==="

# Stop containers
sudo docker-compose down

# Rebuild and run
sudo docker build --no-cache -t ospf-network-simulator:latest .
sudo docker-compose up -d

echo -e "\n=== Waiting for container to start ==="
sleep 5

echo -e "\n=== Testing pages ==="
echo "1. Main application: http://localhost:8080"
echo "2. Test page: http://localhost:8080/test.html"
echo ""
echo "Please open the test page first to diagnose any JavaScript errors."

echo -e "\n=== Complete ==="