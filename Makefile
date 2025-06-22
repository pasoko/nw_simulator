.PHONY: help build run stop clean dev push deploy

# Default target
help:
	@echo "OSPF Network Simulator - Docker Commands"
	@echo ""
	@echo "Usage:"
	@echo "  make build       - Build Docker image"
	@echo "  make run         - Run container in production mode"
	@echo "  make dev         - Run in development mode with hot reload"
	@echo "  make stop        - Stop all containers"
	@echo "  make clean       - Remove containers and images"
	@echo "  make push        - Push image to registry"
	@echo "  make deploy      - Deploy to Kubernetes"
	@echo "  make logs        - Show container logs"
	@echo ""

# Build Docker image
build:
	docker build -t ospf-network-simulator:latest .

# Run in production mode
run:
	docker-compose up -d

# Run in development mode
dev:
	docker-compose -f docker-compose.dev.yml up

# Stop containers
stop:
	docker-compose down
	docker-compose -f docker-compose.dev.yml down

# Clean up
clean: stop
	docker rmi ospf-network-simulator:latest || true
	docker system prune -f

# Show logs
logs:
	docker-compose logs -f

# Push to registry
push:
	@echo "Tagging image..."
	docker tag ospf-network-simulator:latest $(DOCKER_REGISTRY)/ospf-network-simulator:$(IMAGE_TAG)
	@echo "Pushing to registry..."
	docker push $(DOCKER_REGISTRY)/ospf-network-simulator:$(IMAGE_TAG)

# Deploy to Kubernetes
deploy:
	kubectl apply -f k8s-deployment.yaml

# Build for multiple platforms (requires Docker Buildx)
build-multiarch:
	docker buildx build --platform linux/amd64,linux/arm64 -t ospf-network-simulator:latest .

# Health check
health:
	@curl -f http://localhost:8080/health.html || echo "Service is not healthy"