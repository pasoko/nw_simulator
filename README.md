# OSPF Network Simulator

A WebAssembly-based network simulator for visualizing OSPFv2 protocol operations.

## Features

- Real-time network simulation with OSPFv2 protocol
- Visual packet flow animation
- Routing table calculation using SPF algorithm
- Interactive GUI for network topology creation
- Log export functionality

## Quick Start with Docker

### Prerequisites
- Docker
- Docker Compose

### Running the Simulator

1. Clone the repository:
```bash
git clone <repository-url>
cd nw_simulator
```

2. Build and run with Docker Compose:
```bash
docker-compose up --build
```

3. Access the simulator at `http://localhost:8080`

### Using Docker directly

Build the image:
```bash
docker build -t ospf-simulator .
```

Run the container:
```bash
docker run -p 8080:80 ospf-simulator
```

## Development Setup

### Prerequisites
- Rust (latest stable)
- wasm-pack
- Node.js (v16+)

### Building from Source

1. Install dependencies:
```bash
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

2. Build the project:
```bash
./build.sh
```

3. Start development server:
```bash
cd www
npm start
```

## Usage

1. **Add Routers**: Click "Add Router" and click on the canvas to place routers
2. **Connect Routers**: Click "Connect Routers", then select two routers to connect
3. **Enable OSPF**: Click "Enable OSPF" button for each router
4. **Start Simulation**: Click "Start Simulation" to begin packet exchange
5. **View Routing Tables**: Select a router from the dropdown to see its routing table

## Architecture

- **Backend**: Rust compiled to WebAssembly
- **Frontend**: Vanilla JavaScript with Canvas API
- **Web Server**: Nginx (in Docker container)
- **Protocol**: OSPFv2 implementation with neighbor discovery and SPF calculation

## License

This project is for educational purposes.