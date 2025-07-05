/**
 * Simulation Controller Module
 * Manages simulation state and timing
 */

import stateManager from './state-manager.js';
import eventLogger from './event-logger.js';
import canvasRenderer from './canvas-renderer.js';
import displayUpdater from './display-updater.js';
// Removed direct import to avoid circular dependency - using custom events instead

class SimulationController {
    constructor() {
        this.simulationStepInterval = 100; // ms
        this.simulationStepDelta = 0.1; // simulation time units
        this.lastRouterUpdateTime = 0; // Track last router list update
        this.routerUpdateInterval = 2.0; // Update router list every 2 seconds during simulation
    }
    
    init() {
        // Initialization if needed
    }
    
    toggleSimulation() {
        if (stateManager.simulationRunning) {
            this.stopSimulation();
        } else {
            this.startSimulation();
        }
    }
    
    startSimulation() {
        if (stateManager.simulationRunning) {
            this.stopSimulation();
            return;
        }
        
        if (!stateManager.simulationPaused) {
            // First time starting - reset time and router update tracker
            stateManager.resetSimulationTime();
            this.lastRouterUpdateTime = 0;
            
            eventLogger.log('Starting OSPF simulation...');
            
            // Sync initial state
            displayUpdater.syncInitialState();
            window.dispatchEvent(new CustomEvent('updateRoutersList'));
        } else {
            // Resuming from pause
            eventLogger.log(`Resuming simulation from ${stateManager.simulationTime.toFixed(1)}s...`);
        }
        
        try {
            console.log('About to call start_simulation()...');
            stateManager.simulator.start_simulation();
            console.log('start_simulation() called successfully');
            stateManager.setSimulationRunning(true);
            stateManager.setSimulationPaused(false);
        } catch (error) {
            console.error('Error starting simulation:', error);
            console.error('Error stack:', error.stack);
            eventLogger.log(`Failed to start simulation: ${error.message}`);
            return;
        }
        
        // Update UI
        window.dispatchEvent(new CustomEvent('updateSimulationButton', { detail: { isRunning: true } }));
        window.dispatchEvent(new CustomEvent('showTimer'));
        window.dispatchEvent(new CustomEvent('updateTimer', { detail: { time: stateManager.simulationTime } }));
        
        // Start simulation loop
        stateManager.simulationInterval = setInterval(() => {
            this.stepSimulation();
        }, this.simulationStepInterval);
    }
    
    stopSimulation() {
        if (!stateManager.simulationRunning) return;
        
        eventLogger.log(`Pausing simulation at ${stateManager.simulationTime.toFixed(1)}s...`);
        stateManager.simulator.stop_simulation();
        stateManager.setSimulationRunning(false);
        stateManager.setSimulationPaused(true);
        
        // Update UI
        window.dispatchEvent(new CustomEvent('updateSimulationButton', { detail: { isRunning: false } }));
        window.dispatchEvent(new CustomEvent('updateTimer', { detail: { time: stateManager.simulationTime } }));
        
        if (stateManager.simulationInterval) {
            clearInterval(stateManager.simulationInterval);
            stateManager.simulationInterval = null;
        }
    }
    
    stepSimulation() {
        try {
            // Log first step to debug
            if (stateManager.simulationTime === 0) {
                console.log('First step_simulation call...');
            }
            
            stateManager.simulator.step_simulation(this.simulationStepDelta);
            stateManager.incrementSimulationTime(this.simulationStepDelta);
            
            // Update display with new simulation data
            displayUpdater.updateSimulationDisplay();
            displayUpdater.updateSimulationTime(stateManager.simulationTime);
            
            // Update timer
            window.dispatchEvent(new CustomEvent('updateTimer', { detail: { time: stateManager.simulationTime } }));
            
            // Update router details periodically (not every step to prevent flicker)
            if (stateManager.simulationTime - this.lastRouterUpdateTime >= this.routerUpdateInterval) {
                window.dispatchEvent(new CustomEvent('updateRoutersList'));
                this.lastRouterUpdateTime = stateManager.simulationTime;
            }
        } catch (error) {
            console.error('Error in stepSimulation:', error);
            console.error('Error stack:', error.stack);
            eventLogger.log(`Simulation error: ${error.message}`);
            this.stopSimulation();
        }
    }
    
    
    
}

// Export as singleton
export default new SimulationController();