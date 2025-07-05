/**
 * Application Initializer Module
 * Handles WASM loading, app setup, and initialization
 */

import initWasm, { NetworkSimulator } from '../pkg/nw_simulator.js';
import { PacketVisualizer } from '../packet-visualizer.js';
import stateManager from './state-manager.js';
import canvasRenderer from './canvas-renderer.js';
import uiController from './ui-controller.js';
import canvasInteraction from './canvas-interaction.js';
import simulationController from './simulation-controller.js';
import eventLogger from './event-logger.js';
import resizablePanel from './resizable-panel.js';

class ApplicationInitializer {
    constructor() {
        this.initialized = false;
        this.updateInterval = null;
    }

    async init() {
        if (this.initialized) {
            console.warn('Application already initialized');
            return;
        }

        try {
            console.log('Starting application initialization...');
            
            // Initialize WASM
            console.log('Step 1: Initializing WASM...');
            await this.initWasm();
            console.log('✓ WASM initialized successfully');
            
            // Create simulator instance
            console.log('Step 2: Creating simulator instance...');
            stateManager.simulator = new NetworkSimulator();
            // Make simulator and stateManager globally accessible for testing and theme manager
            window.networkSimulator = stateManager.simulator;
            window.stateManager = stateManager;
            console.log('✓ Simulator instance created successfully');
            
            // Setup canvas
            console.log('Step 3: Setting up canvas...');
            this.setupCanvas();
            console.log('✓ Canvas setup completed');
            
            // Initialize modules
            console.log('Step 4: Initializing modules...');
            this.initializeModules();
            console.log('✓ Modules initialized successfully');
            
            // Setup periodic updates
            console.log('Step 5: Setting up periodic updates...');
            this.setupPeriodicUpdates();
            console.log('✓ Periodic updates configured');
            
            // Initial render
            console.log('Step 6: Performing initial render...');
            canvasRenderer.render();
            console.log('✓ Initial render completed');
            
            this.initialized = true;
            console.log('🎉 Application initialized successfully');
            
        } catch (error) {
            console.error('❌ Application initialization failed:', error);
            console.error('Error stack:', error.stack);
            this.showError(`Failed to initialize application: ${error.message}. Please refresh the page.`);
        }
    }

    async initWasm() {
        try {
            console.log('Initializing WASM module using default resolution...');
            // Let wasm-pack handle the path resolution automatically
            await initWasm();
            console.log('WASM module loaded successfully');
        } catch (error) {
            console.error('WASM loading failed:', error);
            throw new Error(`WASM initialization failed: ${error.message}`);
        }
    }

    setupCanvas() {
        try {
            console.log('Looking for canvas element with ID: network-canvas');
            const canvas = document.getElementById('network-canvas');
            if (!canvas) {
                throw new Error('Canvas element with ID "network-canvas" not found in DOM');
            }
            console.log('✓ Canvas element found:', canvas);
            
            console.log('Getting 2D context from canvas...');
            const ctx = canvas.getContext('2d');
            if (!ctx) {
                throw new Error('Failed to get 2D context from canvas - browser may not support Canvas API');
            }
            console.log('✓ 2D context obtained successfully');
            
            // Store in state manager
            console.log('Storing canvas and context in state manager...');
            stateManager.canvas = canvas;
            stateManager.ctx = ctx;
            
            // Setup canvas properties
            console.log('Configuring canvas properties...');
            this.configureCanvas(canvas);
            
            // Initialize packet visualizer
            console.log('Initializing packet visualizer...');
            stateManager.packetVisualizer = new PacketVisualizer(canvas, ctx);
            
            console.log('Canvas setup completed successfully');
        } catch (error) {
            console.error('Canvas setup failed:', error);
            throw new Error(`Canvas initialization failed: ${error.message}`);
        }
    }

    configureCanvas(canvas) {
        // Set canvas size to match container
        const container = canvas.parentElement;
        const rect = container.getBoundingClientRect();
        
        canvas.width = rect.width;
        canvas.height = rect.height;
        
        // Handle canvas resize
        window.addEventListener('resize', () => {
            const newRect = container.getBoundingClientRect();
            canvas.width = newRect.width;
            canvas.height = newRect.height;
            canvasRenderer.render();
        });
        
        // Prevent default drag behavior
        canvas.ondragstart = () => false;
    }

    initializeModules() {
        try {
            console.log('Initializing individual modules...');
            
            console.log('- Initializing event logger...');
            eventLogger.init();
            
            console.log('- Initializing UI controller...');
            uiController.init();
            
            console.log('- Initializing canvas interaction...');
            canvasInteraction.init(stateManager.canvas);
            
            console.log('- Initializing canvas renderer...');
            canvasRenderer.init(stateManager.canvas, stateManager.ctx);
            
            console.log('- Initializing simulation controller...');
            simulationController.init();
            
            console.log('- Initializing resizable panel...');
            resizablePanel.init();
            
            // Setup event listeners for inter-module communication
            console.log('- Setting up event listeners...');
            this.setupEventListeners();
            
            console.log('✓ All modules initialized successfully');
        } catch (error) {
            console.error('Module initialization failed:', error);
            throw new Error(`Module initialization failed: ${error.message}`);
        }
    }
    
    setupEventListeners() {
        // Listen for simulation toggle events from UI
        window.addEventListener('toggleSimulation', () => {
            simulationController.toggleSimulation();
        });
        
        // Listen for UI update events from simulation controller
        window.addEventListener('updateSimulationButton', (event) => {
            uiController.updateSimulationButton(event.detail.isRunning);
        });
        
        window.addEventListener('showTimer', () => {
            uiController.showTimer();
        });
        
        window.addEventListener('updateTimer', (event) => {
            uiController.updateTimer(event.detail.time);
        });
        
        window.addEventListener('updateRoutersList', () => {
            uiController.updateRoutersList();
        });
    }

    setupPeriodicUpdates() {
        // Initial update
        uiController.updateRoutersList();
        
        // Start periodic updates only when not simulating
        this.updateInterval = setInterval(() => {
            // Only update when simulation is not running to prevent flickering
            if (!stateManager.simulationRunning) {
                uiController.updateRoutersList();
            }
        }, 3000); // Increased interval to reduce flicker
        
        console.log('Periodic updates started (paused during simulation)');
    }

    showError(message) {
        // Create or update error display
        let errorDiv = document.getElementById('app-error');
        if (!errorDiv) {
            errorDiv = document.createElement('div');
            errorDiv.id = 'app-error';
            errorDiv.style.cssText = `
                position: fixed;
                top: 50%;
                left: 50%;
                transform: translate(-50%, -50%);
                background: #f44336;
                color: white;
                padding: 20px;
                border-radius: 8px;
                box-shadow: 0 4px 8px rgba(0,0,0,0.2);
                z-index: 1000;
                max-width: 400px;
                text-align: center;
            `;
            document.body.appendChild(errorDiv);
        }
        
        errorDiv.innerHTML = `
            <h3>Application Error</h3>
            <p>${message}</p>
            <button onclick="window.location.reload()" style="
                background: white;
                color: #f44336;
                border: none;
                padding: 8px 16px;
                border-radius: 4px;
                cursor: pointer;
                margin-top: 10px;
            ">Reload Page</button>
        `;
    }

    // Cleanup method for when the app is destroyed
    cleanup() {
        if (this.updateInterval) {
            clearInterval(this.updateInterval);
            this.updateInterval = null;
        }
        
        // Cleanup other resources
        simulationController.stopSimulation();
        
        this.initialized = false;
        console.log('Application cleaned up');
    }

    // Getter for initialization status
    isInitialized() {
        return this.initialized;
    }
}

// Create singleton instance
const appInitializer = new ApplicationInitializer();

export default appInitializer;