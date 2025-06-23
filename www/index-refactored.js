/**
 * Main Application Entry Point
 * Refactored to use modular architecture
 */

// Import without bundling WASM
import initWasm, { NetworkSimulator } from './pkg/nw_simulator.js';
import { PacketVisualizer } from './packet-visualizer.js';

// Import modules
import stateManager from './modules/state-manager.js';
import canvasRenderer from './modules/canvas-renderer.js';
import routerManager from './modules/router-manager.js';
import connectionManager from './modules/connection-manager.js';
import eventLogger from './modules/event-logger.js';
import simulationController from './modules/simulation-controller.js';

// Override the default WASM path
const init = () => initWasm('./pkg/nw_simulator_bg.wasm');

// Make modules available globally for onclick handlers
window.modules = {
    routerManager,
    connectionManager,
    eventLogger,
    simulationController
};

async function run() {
    try {
        await init();
        
        // Initialize simulator
        stateManager.simulator = new NetworkSimulator();
        
        // Get canvas element
        const canvas = document.getElementById('network-canvas');
        if (!canvas) {
            throw new Error('Canvas element not found');
        }
        
        // Initialize packet visualizer
        stateManager.packetVisualizer = new PacketVisualizer(canvas, canvas.getContext('2d'));
        
        // Initialize renderer
        canvasRenderer.initialize(canvas, stateManager.packetVisualizer);
        
        // Setup event listeners
        setupEventListeners();
        
        // Initial render
        canvasRenderer.render();
        
        // Start periodic updates for router details
        stateManager.updateInterval = setInterval(() => {
            if (!stateManager.simulationRunning) {
                simulationController.updateRoutersList();
            }
        }, 1000); // Update every second when not simulating
        
        eventLogger.log('Network Simulator initialized');
    } catch (error) {
        console.error('Error during initialization:', error);
        eventLogger.log(`Error: ${error.message}`);
        showError(`Initialization Error: ${error.message}`);
    }
}

function setupEventListeners() {
    const canvas = document.getElementById('network-canvas');
    
    // Canvas events
    canvas.addEventListener('click', handleCanvasClick);
    canvas.addEventListener('mousedown', handleMouseDown);
    canvas.addEventListener('mousemove', handleMouseMove);
    canvas.addEventListener('mouseup', handleMouseUp);
    canvas.addEventListener('mouseleave', handleMouseUp);
    
    // Button events
    setupButtonListeners();
}

function setupButtonListeners() {
    const buttons = {
        'add-router-btn': () => setMode('add-router'),
        'move-router-btn': () => setMode('move-router'),
        'connect-routers-btn': () => setMode('connect-routers'),
        'delete-router-btn': () => setMode('delete-router'),
        'disconnect-routers-btn': () => setMode('disconnect-routers'),
        'simulate-btn': () => simulationController.startSimulation(),
        'export-log-btn': () => eventLogger.exportLog(),
        'clear-log-btn': () => eventLogger.clearLog()
    };
    
    Object.entries(buttons).forEach(([id, handler]) => {
        const btn = document.getElementById(id);
        if (btn) {
            btn.addEventListener('click', handler);
        }
    });
}

function setMode(newMode) {
    stateManager.setMode(newMode);
    updateModeIndicator();
    canvasRenderer.render();
}

function updateModeIndicator() {
    const indicator = document.getElementById('mode-indicator');
    const mode = stateManager.getMode();
    
    const modeConfig = {
        'add-router': {
            text: 'Mode: Add Router - Click on canvas to place router',
            color: '#ffc107',
            cursor: 'crosshair'
        },
        'move-router': {
            text: 'Mode: Move Router - Drag router to new position',
            color: '#17a2b8',
            cursor: 'grab'
        },
        'connect-routers': {
            text: 'Mode: Connect Routers - Select first router',
            color: '#17a2b8',
            cursor: 'pointer'
        },
        'delete-router': {
            text: 'Mode: Delete Router - Click on router to delete',
            color: '#dc3545',
            cursor: 'pointer'
        },
        'disconnect-routers': {
            text: 'Mode: Disconnect Routers - Select first router',
            color: '#dc3545',
            cursor: 'pointer'
        }
    };
    
    const config = modeConfig[mode];
    if (config) {
        indicator.textContent = config.text;
        indicator.style.backgroundColor = config.color;
        canvasRenderer.updateCursor(config.cursor);
    }
}

function handleCanvasClick(event) {
    if (stateManager.isDragging()) return;
    
    const rect = event.target.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    const mode = stateManager.getMode();
    const clickedRouter = stateManager.findRouterAt(x, y);
    
    switch (mode) {
        case 'add-router':
            handleAddRouter(x, y);
            break;
        case 'connect-routers':
            handleConnectMode(clickedRouter);
            break;
        case 'delete-router':
            handleDeleteRouter(clickedRouter);
            break;
        case 'disconnect-routers':
            handleDisconnectMode(clickedRouter);
            break;
    }
}

function handleAddRouter(x, y) {
    const name = prompt('Enter router name:');
    if (name && name.trim()) {
        const router = routerManager.createRouter(name, x, y);
        if (router) {
            simulationController.updateRoutersList();
            canvasRenderer.render();
            eventLogger.log(`Router ${router.name} created with OSPF enabled`);
        }
    }
}

function handleConnectMode(clickedRouter) {
    if (!clickedRouter) return;
    
    const result = connectionManager.handleConnectionMode(clickedRouter);
    const indicator = document.getElementById('mode-indicator');
    
    if (result.action === 'connected') {
        indicator.textContent = 'Mode: Connect Routers - Connection created! Select first router for next connection';
        indicator.style.backgroundColor = '#17a2b8';
        
        if (result.success) {
            const fromRouter = stateManager.findRouterById(result.from);
            const toRouter = stateManager.findRouterById(result.to);
            eventLogger.log(`Connected routers ${fromRouter.name} and ${toRouter.name}`);
        }
    } else if (result.count === 1) {
        indicator.textContent = `Mode: Connect Routers - First router selected (${clickedRouter.name}). Select second router`;
        indicator.style.backgroundColor = '#28a745';
    }
    
    canvasRenderer.render();
}

function handleDeleteRouter(clickedRouter) {
    if (!clickedRouter) return;
    
    if (routerManager.deleteRouter(clickedRouter.id)) {
        simulationController.updateRoutersList();
        canvasRenderer.render();
        eventLogger.log(`Router ${clickedRouter.name} deleted`);
    }
}

function handleDisconnectMode(clickedRouter) {
    if (!clickedRouter) return;
    
    const result = connectionManager.handleDisconnectionMode(clickedRouter);
    const indicator = document.getElementById('mode-indicator');
    
    if (result.action === 'disconnected') {
        indicator.textContent = 'Mode: Disconnect Routers - Connection removed! Select first router for next disconnection';
        
        if (result.success) {
            eventLogger.log(`Disconnected routers ${result.from} and ${result.to}`);
        }
    } else if (result.count === 1) {
        indicator.textContent = `Mode: Disconnect Routers - First router selected (${clickedRouter.name}). Select second router`;
        indicator.style.backgroundColor = '#dc3545';
    }
    
    canvasRenderer.render();
}

function handleMouseDown(event) {
    const rect = event.target.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    const clickedRouter = stateManager.findRouterAt(x, y);
    if (clickedRouter && stateManager.getMode() === 'move-router') {
        stateManager.startDragging(clickedRouter, x - clickedRouter.x, y - clickedRouter.y);
        canvasRenderer.updateCursor('grabbing');
        event.preventDefault();
    }
}

function handleMouseMove(event) {
    const rect = event.target.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    
    stateManager.updateMousePosition(x, y);
    
    if (stateManager.isDragging()) {
        const router = stateManager.draggingRouter;
        const newX = x - stateManager.dragOffset.x;
        const newY = y - stateManager.dragOffset.y;
        
        routerManager.updateRouterPosition(router.id, newX, newY);
        canvasRenderer.render();
    } else {
        updateHoverCursor(x, y);
    }
}

function handleMouseUp(event) {
    if (stateManager.isDragging()) {
        stateManager.stopDragging();
        canvasRenderer.updateCursor('default');
    }
}

function updateHoverCursor(x, y) {
    const hoverRouter = stateManager.findRouterAt(x, y);
    const mode = stateManager.getMode();
    
    if (hoverRouter) {
        switch (mode) {
            case 'move-router':
                canvasRenderer.updateCursor('grab');
                break;
            case 'delete-router':
            case 'connect-routers':
            case 'disconnect-routers':
                canvasRenderer.updateCursor('pointer');
                break;
            default:
                canvasRenderer.updateCursor('default');
        }
    } else if (mode === 'add-router') {
        canvasRenderer.updateCursor('crosshair');
    } else {
        canvasRenderer.updateCursor('default');
    }
    
    // Re-render for hover effects in delete mode
    if (mode === 'delete-router') {
        canvasRenderer.render();
    }
}

function showError(message) {
    const errorDiv = document.createElement('div');
    errorDiv.style.cssText = 'position: fixed; top: 10px; left: 50%; transform: translateX(-50%); background: red; color: white; padding: 10px; border-radius: 5px; z-index: 9999;';
    errorDiv.textContent = message;
    document.body.appendChild(errorDiv);
    
    setTimeout(() => {
        document.body.removeChild(errorDiv);
    }, 5000);
}

// Wait for DOM to be fully loaded
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', run);
} else {
    // DOM is already loaded
    run();
}