#!/bin/bash

echo "=== Testing Current Deployment ==="

echo -e "\n1. Testing main page:"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080)
echo "HTTP Status: $HTTP_CODE"

echo -e "\n2. Testing WASM file at various paths:"
# Test different possible WASM paths
PATHS=(
    "/b57ea1b4dc8b0d72fa91.wasm"
    "/1497e889520afbc86913.wasm"
    "/pkg/nw_simulator_bg.wasm"
)

for path in "${PATHS[@]}"; do
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:8080$path")
    if [ "$HTTP_CODE" = "200" ]; then
        echo "✓ Found at $path (HTTP $HTTP_CODE)"
        # Check if it's actually WASM
        HEADER=$(curl -s -I "http://localhost:8080$path" | grep -i content-type)
        echo "  Content-Type: $HEADER"
    else
        echo "✗ Not found at $path (HTTP $HTTP_CODE)"
    fi
done

echo -e "\n3. Checking what bundle.js is requesting:"
curl -s http://localhost:8080/bundle.js | grep -o '[a-f0-9]\{20\}\.wasm' | head -5 || echo "No direct WASM references found"

echo -e "\n4. Testing direct WASM test page:"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/direct-wasm-test.html)
echo "direct-wasm-test.html: HTTP $HTTP_CODE"

echo -e "\n=== Complete ==="