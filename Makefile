.PHONY: help build run stop clean dev logs rebuild

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