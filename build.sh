#!/bin/bash

echo "Building WebAssembly module..."
wasm-pack build --target web --out-dir www/pkg

echo "Installing npm dependencies..."
cd www
npm install

echo "Build complete! Run 'npm start' in the www directory to start the development server."