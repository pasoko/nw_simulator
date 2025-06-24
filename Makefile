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
	sudo docker build -t ospf-network-simulator:latest .

# Run in production mode
run:
	sudo docker-compose up -d

# Run in development mode
dev:
	sudo docker-compose -f docker-compose.dev.yml up

# Stop containers
stop:
	sudo docker-compose down
	sudo docker-compose -f docker-compose.dev.yml down

# Clean up
clean: stop
	sudo docker rmi ospf-network-simulator:latest || true
	sudo docker system prune -f

# Show logs
logs:
	sudo docker-compose logs -f

# Complete clean rebuild
rebuild: clean
	@echo "Starting clean rebuild..."
	cd www && npm install
	wasm-pack build --target web --out-dir www/pkg
	sudo docker build -t ospf-network-simulator:latest .
	@echo "Rebuild complete!"