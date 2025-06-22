#!/bin/bash

echo "=== Quick local test ==="

cd www

# Clean dist directory
rm -rf dist

# Build
echo "Building..."
npm run build

# Check result
echo -e "\n=== Checking buttons in dist/index.html ==="
if [ -f dist/index.html ]; then
    grep -c "Delete Router" dist/index.html && echo "✓ Delete Router button found" || echo "✗ Delete Router button NOT found"
    grep -c "Disconnect Routers" dist/index.html && echo "✓ Disconnect Routers button found" || echo "✗ Disconnect Routers button NOT found"
    grep -c "button danger" dist/index.html && echo "✓ danger button CSS found" || echo "✗ danger button CSS NOT found"
else
    echo "ERROR: dist/index.html not found!"
fi

cd ..

echo -e "\n=== Test complete ==="