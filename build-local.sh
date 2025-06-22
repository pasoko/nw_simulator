#!/bin/bash

echo "=== Building locally first ==="

# Step 1: Build Rust/WebAssembly
echo "Building WebAssembly module..."
wasm-pack build --target web --out-dir www/pkg

# Step 2: Install npm dependencies
echo "Installing npm dependencies..."
cd www
npm install

# Step 3: Build frontend
echo "Building frontend..."
npm run build

cd ..

# Step 4: Build Docker image with pre-built files
echo "Building Docker image..."
sudo docker build -f Dockerfile.simple -t ospf-network-simulator:latest .

echo "=== Build complete ==="