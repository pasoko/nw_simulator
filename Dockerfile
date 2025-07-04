# Build stage for Rust/WebAssembly
FROM rust:1.88-slim AS rust-builder

# Install required dependencies
RUN apt-get update && apt-get install -y \
    curl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /app

# Copy Rust source files
COPY Cargo.toml ./
COPY src ./src

# Create dummy benchmark file to satisfy Cargo.toml reference
RUN mkdir -p benches && touch benches/ospf_benchmark.rs

# Generate Cargo.lock if needed
RUN cargo generate-lockfile

# Build WebAssembly module
RUN wasm-pack build --target web --out-dir pkg

# Frontend build stage
FROM node:22-alpine AS frontend-builder

WORKDIR /app/www

# Copy package files
COPY www/package.json www/yarn.lock* ./

# Install dependencies
RUN yarn install

# Copy frontend source files
COPY www/index.html ./
COPY www/index.js ./
COPY www/packet-visualizer.js ./
COPY www/modules ./modules
COPY www/webpack.config.js ./
COPY www/health.html ./

# Copy WebAssembly output from rust builder
COPY --from=rust-builder /app/pkg ./pkg

# Build frontend
RUN yarn build

# Production stage with nginx
FROM nginx:alpine

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Copy built files from frontend builder stage
COPY --from=frontend-builder /app/www/dist /usr/share/nginx/html

# Copy health check file
COPY --from=frontend-builder /app/www/health.html /usr/share/nginx/html/

# Expose port 80
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]