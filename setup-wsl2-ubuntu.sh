#!/bin/bash

# WSL2 Ubuntu 24.04 Development Environment Setup Script
# For OSPF Network Simulator

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
    gnupg

# Step 3: Install Rust
print_step 3 "Installing Rust..."
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

# Step 4: Install wasm-pack
print_step 4 "Installing wasm-pack..."
if command_exists wasm-pack; then
    print_warning "wasm-pack is already installed"
    wasm-pack --version
else
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Step 5: Install Node.js (v20 LTS)
print_step 5 "Installing Node.js and npm..."
if command_exists node; then
    NODE_VERSION=$(node --version)
    print_warning "Node.js is already installed: $NODE_VERSION"
else
    # Add NodeSource repository
    curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
    sudo apt-get install -y nodejs
fi

# Step 6: Docker setup (without sudo)
print_step 6 "Setting up Docker..."
if command_exists docker; then
    print_warning "Docker is already installed"
else
    # Add Docker's official GPG key
    sudo install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    sudo chmod a+r /etc/apt/keyrings/docker.gpg

    # Add the repository to Apt sources
    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
      $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
      sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

    # Update and install Docker
    sudo apt-get update
    sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
fi

# Configure Docker to run without sudo
print_step 7 "Configuring Docker for non-root access..."
if ! groups $USER | grep -q docker; then
    sudo groupadd -f docker
    sudo usermod -aG docker $USER
    print_warning "You have been added to the docker group."
    print_warning "You need to log out and back in for this to take effect."
    print_warning "Alternatively, run: newgrp docker"
fi

# Step 8: Verify installations
print_step 8 "Verifying installations..."
echo ""
echo "Checking installed tools:"
echo "========================="
check_version "git" "git --version"
check_version "gcc" "gcc --version | head -n1"
check_version "rustc" "rustc --version"
check_version "cargo" "cargo --version"
check_version "wasm-pack" "wasm-pack --version"
check_version "node" "node --version"
check_version "npm" "npm --version"
check_version "docker" "docker --version"
check_version "docker-compose" "docker compose version"

# Step 9: Project setup (optional)
echo ""
print_step 9 "Project setup..."
if [ -f "Cargo.toml" ] && [ -d "www" ]; then
    echo "Detected project files. Would you like to build the project now? (y/n)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        print_step 10 "Building the project..."
        
        # Install npm dependencies
        cd www && npm install
        cd ..
        
        # Build WebAssembly module
        wasm-pack build --target web --out-dir www/pkg
        
        echo -e "${GREEN}✓${NC} Project build completed!"
        echo ""
        echo "You can now run the project with:"
        echo "  - Development mode: cd www && npm start"
        echo "  - Docker mode: make -f Makefile.nosudo run"
    fi
else
    print_warning "Not in project directory. Skipping project build."
fi

# Final instructions
echo ""
echo "=========================================="
echo -e "${GREEN}Setup completed successfully!${NC}"
echo "=========================================="
echo ""
echo "Next steps:"
echo "1. If you were added to the docker group, run: newgrp docker"
echo "2. Clone the project: git clone <repository-url>"
echo "3. Navigate to project: cd nw_simulator"
echo "4. Build the project: make -f Makefile.nosudo setup-local"
echo "5. Start development server: cd www && npm start"
echo ""
echo "For Docker operations without sudo:"
echo "  - Build: make -f Makefile.nosudo build"
echo "  - Run: make -f Makefile.nosudo run"
echo ""

# Check if we need to reload shell
if ! groups $USER | grep -q docker; then
    echo -e "${YELLOW}Important:${NC} Run 'newgrp docker' or log out and back in to use Docker without sudo."
fi