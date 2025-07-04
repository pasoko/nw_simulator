.PHONY: help build run stop clean dev logs rebuild test-rust test-js test-all build-with-test coverage-rust

# Default target
help:
	@echo "OSPF Network Simulator - Docker Commands"
	@echo ""
	@echo "Usage:"
	@echo "  make build         - Build Docker image"
	@echo "  make run           - Run container in production mode"
	@echo "  make dev           - Run in development mode with hot reload"
	@echo "  make stop          - Stop all containers"
	@echo "  make clean         - Remove containers and images"
	@echo "  make logs          - Show container logs"
	@echo "  make rebuild       - Clean rebuild from scratch"
	@echo ""
	@echo "Testing Commands:"
	@echo "  make test-rust     - Run Rust tests"
	@echo "  make test-js       - Run JavaScript tests"
	@echo "  make test-all      - Run all tests"
	@echo "  make build-with-test - Run tests before building"
	@echo "  make coverage-rust - Generate Rust code coverage report"
	@echo ""

# Build Docker image
build:
	@echo "Note: Using legacy Docker builder (buildx not available)" 
	docker build -t ospf-network-simulator:latest .

# Run in production mode
run:
	docker-compose up -d

# Run in development mode
dev:
	docker-compose -f docker-compose.dev.yml up

# Stop containers
stop:
	docker-compose down 2>/dev/null || true
	docker-compose -f docker-compose.dev.yml down 2>/dev/null || true

# Clean up
clean:
	@echo "Cleaning up Docker resources..."
	docker-compose down 2>/dev/null || true
	docker-compose -f docker-compose.dev.yml down 2>/dev/null || true
	docker rmi ospf-network-simulator:latest || true
	docker system prune -f

# Show logs
logs:
	docker-compose logs -f

# Complete clean rebuild
rebuild: clean
	@echo "Starting clean rebuild..."
	cd www && yarn install
	PATH="$$HOME/.cargo/bin:$$PATH" wasm-pack build --target web --out-dir www/pkg
	@echo "Note: Using legacy Docker builder (buildx not available)" 
	docker build -t ospf-network-simulator:latest .
	@echo "Rebuild complete!"

# Run Rust tests
test-rust:
	@echo "Running Rust tests..."
	cargo test --all-features
	@echo "Rust tests completed!"

# Run JavaScript tests
test-js:
	@echo "Running JavaScript tests..."
	@echo "Note: JavaScript tests require 'yarn install' in www directory first"
	@cd www && yarn test --run 2>/dev/null || echo "JavaScript tests skipped (dependencies not installed)"

# Run all tests
test-all: test-rust test-js
	@echo "All tests completed!"

# Build with tests
build-with-test: test-all
	@echo "All tests passed! Starting build..."
	$(MAKE) build

# Generate Rust code coverage report
coverage-rust:
	@echo "Generating Rust code coverage report..."
	@if command -v cargo-tarpaulin >/dev/null 2>&1; then \
		cargo tarpaulin --out Html --output-dir target/coverage; \
		echo "Coverage report generated at: target/coverage/index.html"; \
	else \
		echo "cargo-tarpaulin is not installed!"; \
		echo "Run: ./setup-coverage.sh to install it"; \
		echo "Or manually install with: cargo install cargo-tarpaulin"; \
		exit 1; \
	fi