#!/bin/bash

# WSL2 Ubuntu 24.04 Development Environment Setup Script
# For OSPF Network Simulator
# This script clones the project and sets up a complete development environment
# including Volta for JavaScript tool management, Rust, Docker, and all dependencies

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${GREEN}[Step $1]${NC} $2"
}

print_warning() {
    echo -e "${YELLOW}[Warning]${NC} $1"
}

print_error() {
    echo -e "${RED}[Error]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check version
check_version() {
    local cmd=$1
    local version_cmd=$2
    if command_exists "$cmd"; then
        echo -e "${GREEN}✓${NC} $cmd is installed: $($version_cmd)"
        return 0
    else
        echo -e "${RED}✗${NC} $cmd is not installed"
        return 1
    fi
}

echo "=========================================="
echo "OSPF Network Simulator Setup for WSL2"
echo "Ubuntu 24.04 Development Environment"
echo "=========================================="
echo ""

# Step 1: Update system packages
print_step 1 "Updating system packages..."
sudo apt-get update
sudo apt-get upgrade -y

# Step 2: Install build essentials and dependencies
print_step 2 "Installing build essentials and system dependencies..."
sudo apt-get install -y \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    make \
    ca-certificates \
    docker-compose \
    gnupg

# Step 3: Clone project and setup Volta/Node.js environment
print_step 3 "Cloning project and setting up Volta/Node.js environment..."
if [ ! -d "nw_simulator" ]; then
    git clone https://github.com/pasoko/nw_simulator.git
else
    print_warning "nw_simulator directory already exists, skipping clone"
fi

cd ./nw_simulator

# Install Volta (JavaScript Tool Manager)
if ! command_exists volta; then
    echo "Installing Volta..."
    curl https://get.volta.sh | bash
    source ~/.bashrc
    export VOLTA_HOME="$HOME/.volta"
    export PATH="$VOLTA_HOME/bin:$PATH"
else
    print_warning "Volta is already installed"
fi

# Install Node.js and Yarn using Volta
echo "Installing Node.js and Yarn with Volta..."
volta install node
volta install yarn

# Install project dependencies
echo "Installing project dependencies..."
yarn install

# Pin Node.js and Yarn versions for this project
echo "Pinning Node.js and Yarn versions..."
volta pin node@22
volta pin yarn@4

echo -e "${GREEN}✓${NC} Project cloned and Node.js environment configured"
echo ""

# Step 4: Install Rust
print_step 4 "Installing Rust..."
if command_exists rustc; then
    print_warning "Rust is already installed"
    rustc --version
else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    # Add to shell profile
    echo 'source "$HOME/.cargo/env"' >> ~/.bashrc
    echo 'source "$HOME/.cargo/env"' >> ~/.profile
fi

# Ensure cargo is in PATH for this session
export PATH="$HOME/.cargo/bin:$PATH"

# Update Rust to stable
rustup default stable
rustup update

# Step 5: Install wasm-pack
print_step 5 "Installing wasm-pack..."
if command_exists wasm-pack; then
    print_warning "wasm-pack is already installed"
    wasm-pack --version
else
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Step 6: Configure Docker to run without sudo
print_step 6 "Configuring Docker for non-root access..."
if ! groups $USER | grep -q docker; then
    sudo groupadd -f docker
    sudo usermod -aG docker $USER
    print_warning "You have been added to the docker group."
    print_warning "You need to log out and back in for this to take effect."
    print_warning "Alternatively, run: newgrp docker"
fi

# Step 7: Verify installations
print_step 7 "Verifying installations..."
echo ""
echo "Checking installed tools:"
echo "========================="
check_version "git" "git --version"
check_version "gcc" "gcc --version | head -n1"
check_version "rustc" "rustc --version"
check_version "cargo" "cargo --version"
check_version "wasm-pack" "wasm-pack --version"
check_version "volta" "volta --version"
check_version "node" "node --version"
check_version "yarn" "yarn --version"
check_version "docker" "docker --version"
check_version "docker-compose" "docker compose version"

# Step 8: Build project (optional)
echo ""
print_step 8 "Building project..."
if [ -f "Cargo.toml" ] && [ -d "www" ]; then
    echo "Would you like to build the WebAssembly module now? (y/n)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        echo "Building WebAssembly module..."
        
        # Build WebAssembly module
        wasm-pack build --target web --out-dir www/pkg
        
        echo -e "${GREEN}✓${NC} Project build completed!"
        echo ""
        echo "You can now run the project with:"
        echo "  - Development mode: cd www && yarn start"
        echo "  - Docker mode: make run"
    fi
else
    print_warning "Project files not found. Please check the directory structure."
fi

# Final instructions
echo ""
echo "=========================================="
echo -e "${GREEN}Setup completed successfully!${NC}"
echo "=========================================="
echo ""
echo "Next steps:"
echo "1. If you were added to the docker group, run: newgrp docker"
echo "2. Navigate to project directory: cd nw_simulator (if not already there)"
echo "3. Build WebAssembly module: wasm-pack build --target web --out-dir www/pkg"
echo "4. Start development server: cd www && yarn start"
echo ""
echo "For Docker operations:"
echo "  - Build: make build"
echo "  - Run: make run"
echo ""

# Check if we need to reload shell
if ! groups $USER | grep -q docker; then
    echo -e "${YELLOW}Important:${NC} Run 'newgrp docker' or log out and back in to use Docker without sudo."
fi