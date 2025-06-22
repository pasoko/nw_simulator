#!/bin/bash

echo "=== Testing local build ==="

# Build WebAssembly
echo "1. Building WebAssembly..."
wasm-pack build --target web --out-dir www/pkg

# Go to www directory
cd www

# Install dependencies
echo "2. Installing npm dependencies..."
npm install

# Build with webpack
echo "3. Building with webpack..."
npm run build

# Check the built file
echo "4. Checking built index.html for buttons..."
if [ -f dist/index.html ]; then
    echo "Found dist/index.html"
    echo "Searching for Delete Router button..."
    grep -n "Delete Router" dist/index.html || echo "Delete Router button NOT found"
    echo "Searching for Disconnect Routers button..."
    grep -n "Disconnect Routers" dist/index.html || echo "Disconnect Routers button NOT found"
else
    echo "ERROR: dist/index.html not found!"
fi

# List all files in dist
echo "5. Files in dist directory:"
ls -la dist/

echo "=== Test complete ==="