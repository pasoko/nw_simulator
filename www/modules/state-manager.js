/**
 * State Manager Module
 * Manages all application state in a centralized location
 */

class StateManager {
    constructor() {
        // Core state
        this.simulator = null;
        this.mode = 'add-router';
        this.selectedRouters = [];
        this.routers = [];
        this.connections = [];
        
        // Canvas and rendering state
        this.canvas = null;
        this.ctx = null;
        this.packetVisualizer = null;
        this.canvasRenderer = null;
        
        // Available modes
        this.modes = {
            ADD_ROUTER: 'add-router',
            MOVE_ROUTER: 'move-router',
            CONNECT_ROUTERS: 'connect-routers',
            DELETE_ROUTER: 'delete-router',
            DISCONNECT_ROUTERS: 'disconnect-routers',
            TOGGLE_FAILURE: 'toggle-failure'
        };
        
        // Simulation state
        this.simulationRunning = false;
        this.simulationPaused = false;
        this.simulationTime = 0;
        this.simulationInterval = null;
        
        // UI state
        this.draggingRouter = null;
        this.dragOffset = { x: 0, y: 0 };
        this.lastMouseX = 0;
        this.lastMouseY = 0;
        
        // Event tracking
        this.lastEventTime = -1;
        this.processedEvents = new Set();
        
        // Log state
        this.logEntries = [];
        
        // Update interval
        this.updateInterval = null;
    }
    
    // Mode management
    setMode(newMode) {
        this.mode = newMode;
        this.selectedRouters = [];
    }
    
    getMode() {
        return this.mode;
    }
    
    // Router management
    addRouter(router) {
        this.routers.push(router);
    }
    
    removeRouter(routerId) {
        this.routers = this.routers.filter(r => r.id !== routerId);
        this.connections = this.connections.filter(c => 
            c.from_router_id !== routerId && c.to_router_id !== routerId
        );
        this.selectedRouters = this.selectedRouters.filter(id => id !== routerId);
    }
    
    findRouterById(routerId) {
        return this.routers.find(r => r.id === routerId);
    }
    
    findRouterAt(x, y, radius = 20) {
        return this.routers.find(router => {
            const dx = router.x - x;
            const dy = router.y - y;
            return dx * dx + dy * dy < radius * radius;
        });
    }
    
    updateRouterPosition(routerId, x, y) {
        const router = this.findRouterById(routerId);
        if (router) {
            router.x = x;
            router.y = y;
        }
    }
    
    // Connection management
    addConnection(connection) {
        this.connections.push(connection);
    }
    
    removeConnection(fromId, toId) {
        this.connections = this.connections.filter(c => 
            !((c.from_router_id === fromId && c.to_router_id === toId) ||
              (c.from_router_id === toId && c.to_router_id === fromId))
        );
    }
    
    connectionExists(fromId, toId) {
        return this.connections.some(c => 
            (c.from_router_id === fromId && c.to_router_id === toId) ||
            (c.from_router_id === toId && c.to_router_id === fromId)
        );
    }
    
    // Selection management
    toggleRouterSelection(routerId) {
        if (this.selectedRouters.includes(routerId)) {
            this.selectedRouters = this.selectedRouters.filter(id => id !== routerId);
        } else {
            this.selectedRouters.push(routerId);
        }
    }
    
    clearSelection() {
        this.selectedRouters = [];
    }
    
    isRouterSelected(routerId) {
        return this.selectedRouters.includes(routerId);
    }
    
    // Simulation state
    setSimulationRunning(running) {
        this.simulationRunning = running;
    }
    
    setSimulationPaused(paused) {
        this.simulationPaused = paused;
    }
    
    resetSimulationTime() {
        this.simulationTime = 0;
        this.lastEventTime = -1;
        this.processedEvents = new Set();
    }
    
    incrementSimulationTime(delta) {
        this.simulationTime += delta;
    }
    
    // Drag state
    startDragging(router, offsetX, offsetY) {
        this.draggingRouter = router;
        this.dragOffset.x = offsetX;
        this.dragOffset.y = offsetY;
    }
    
    stopDragging() {
        this.draggingRouter = null;
    }
    
    isDragging() {
        return this.draggingRouter !== null;
    }
    
    // Mouse position
    updateMousePosition(x, y) {
        this.lastMouseX = x;
        this.lastMouseY = y;
    }
    
    // Event tracking
    hasProcessedEvent(eventKey) {
        return this.processedEvents.has(eventKey);
    }
    
    markEventProcessed(eventKey) {
        this.processedEvents.add(eventKey);
        
        // Keep set size manageable
        if (this.processedEvents.size > 1000) {
            const sortedEvents = Array.from(this.processedEvents).sort();
            const toRemove = sortedEvents.slice(0, sortedEvents.length - 500);
            toRemove.forEach(key => this.processedEvents.delete(key));
        }
    }
    
    // Log management
    addLogEntry(entry) {
        this.logEntries.push(entry);
    }
    
    clearLog() {
        this.logEntries = [];
    }
    
    // Sync with simulator
    syncWithSimulator(simulator) {
        const routersJson = simulator.get_routers_json();
        const connectionsJson = simulator.get_connections_json();
        
        if (routersJson) {
            this.routers = JSON.parse(routersJson);
        }
        
        if (connectionsJson) {
            this.connections = JSON.parse(connectionsJson);
        }
    }
}

// Export as singleton
export default new StateManager();