.PHONY: help build run stop clean dev push deploy rebuild clean-rebuild fix-rebuild

# Default target
help:
	@echo "OSPF Network Simulator - Docker Commands"
	@echo ""
	@echo "Usage:"
	@echo "  make build         - Build Docker image"
	@echo "  make rebuild       - Rebuild from scratch (removes old image)"
	@echo "  make clean-rebuild - Complete clean rebuild (removes everything)"
	@echo "  make fix-wasm-complete - Complete fix for WebAssembly loading"
	@echo "  make run           - Run container in production mode"
	@echo "  make dev           - Run in development mode with hot reload"
	@echo "  make stop          - Stop all containers"
	@echo "  make clean         - Remove containers and images"
	@echo "  make logs          - Show container logs"
	@echo ""

# Build Docker image
build:
	sudo docker build -t ospf-network-simulator:latest .

# Rebuild from scratch (removes old image first)
rebuild:
	sudo docker-compose down
	sudo docker rmi ospf-network-simulator:latest || true
	sudo docker build --no-cache -t ospf-network-simulator:latest .
	sudo docker-compose up -d

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

# Complete clean rebuild
clean-rebuild:
	./clean-rebuild.sh

# Fix webpack config and rebuild
fix-rebuild:
	./fix-and-rebuild.sh

# Final fix with nginx CSP
final-fix:
	./final-fix.sh

# Build with simple nginx (no CSP)
build-simple:
	sudo docker build -f Dockerfile.simple-nginx -t ospf-network-simulator:latest .

# Complete fix for button functionality
fix-complete:
	./final-fix-complete.sh

# Fix WASM loading error
fix-wasm:
	./fix-wasm-error.sh

# Complete WASM fix
fix-wasm-complete:
	./fix-wasm-complete.sh

# Show logs
logs:
	sudo docker-compose logs -f

# Push to registry
push:
	@echo "Tagging image..."
	sudo docker tag ospf-network-simulator:latest $(DOCKER_REGISTRY)/ospf-network-simulator:$(IMAGE_TAG)
	@echo "Pushing to registry..."
	sudo docker push $(DOCKER_REGISTRY)/ospf-network-simulator:$(IMAGE_TAG)

# Deploy to Kubernetes
deploy:
	kubectl apply -f k8s-deployment.yaml

# Build for multiple platforms (requires Docker Buildx)
build-multiarch:
	sudo docker buildx build --platform linux/amd64,linux/arm64 -t ospf-network-simulator:latest .

# Health check
health:
	@curl -f http://localhost:8080/health.html || echo "Service is not healthy"