#!/bin/bash

echo "=== Testing JavaScript build ==="

cd www

# Clean and rebuild
echo "1. Cleaning dist directory..."
rm -rf dist

echo "2. Building with webpack..."
npm run build

echo "3. Checking build output..."
echo "Files in dist:"
ls -la dist/

echo -e "\n4. Checking bundle.js size:"
ls -lh dist/bundle.js

echo -e "\n5. Checking if PacketVisualizer is bundled:"
grep -c "PacketVisualizer" dist/bundle.js || echo "PacketVisualizer not found in bundle"

echo -e "\n6. Checking WebAssembly files:"
ls -la dist/pkg/*.wasm 2>/dev/null || echo "No WASM files found in dist/pkg/"

echo -e "\n7. Checking import statements in bundle:"
grep -o "import.*from" dist/bundle.js | head -5 || echo "No import statements found"

cd ..

echo -e "\n=== Test complete ==="