# Build stage for Rust/WebAssembly
FROM rust:1.79-slim AS rust-builder

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

# Generate Cargo.lock if needed
RUN cargo generate-lockfile

# Build WebAssembly module
RUN wasm-pack build --target web --out-dir pkg

# Frontend build stage
FROM node:20-alpine AS frontend-builder

WORKDIR /app

# Copy package files
COPY www/package.json www/package-lock.json* ./

# Install dependencies
RUN npm install

# Copy frontend source and WebAssembly output
COPY www ./
COPY --from=rust-builder /app/pkg ./pkg

# Build frontend
RUN npm run build

# Production stage with nginx
FROM nginx:alpine

# Copy nginx configuration
COPY nginx.conf /etc/nginx/nginx.conf

# Copy built files from frontend builder stage
COPY --from=frontend-builder /app/dist /usr/share/nginx/html

# Expose port 80
EXPOSE 80

# Start nginx
CMD ["nginx", "-g", "daemon off;"]