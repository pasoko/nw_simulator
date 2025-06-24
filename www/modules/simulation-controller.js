/**
 * Simulation Controller Module
 * Manages simulation state and timing
 */

import stateManager from './state-manager.js';
import eventLogger from './event-logger.js';
import routerManager from './router-manager.js';
import canvasRenderer from './canvas-renderer.js';

class SimulationController {
    constructor() {
        this.simulationStepInterval = 100; // ms
        this.simulationStepDelta = 0.1; // simulation time units
    }
    
    startSimulation() {
        if (stateManager.simulationRunning) {
            this.stopSimulation();
            return;
        }
        
        if (!stateManager.simulationPaused) {
            // First time starting - reset time
            stateManager.resetSimulationTime();
            
            if (stateManager.packetVisualizer) {
                stateManager.packetVisualizer.clear();
            }
            
            eventLogger.log('Starting OSPF simulation...');
            
            // Sync routers and connections with simulator before starting
            stateManager.syncWithSimulator(stateManager.simulator);
            this.updateRoutersList();
        } else {
            // Resuming from pause
            eventLogger.log(`Resuming simulation from ${stateManager.simulationTime.toFixed(1)}s...`);
        }
        
        stateManager.simulator.start_simulation();
        stateManager.setSimulationRunning(true);
        stateManager.setSimulationPaused(false);
        
        const btn = document.getElementById('simulate-btn');
        btn.textContent = 'Stop Simulation';
        btn.classList.add('running');
        
        // Show timer
        const timer = document.getElementById('simulation-timer');
        timer.style.display = 'block';
        this.updateTimer();
        
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
        
        const btn = document.getElementById('simulate-btn');
        btn.textContent = 'Resume Simulation';
        btn.classList.remove('running');
        
        // Keep timer visible to show paused time
        const timer = document.getElementById('simulation-timer');
        timer.textContent = `Time: ${stateManager.simulationTime.toFixed(1)}s (Paused)`;
        
        if (stateManager.simulationInterval) {
            clearInterval(stateManager.simulationInterval);
            stateManager.simulationInterval = null;
        }
    }
    
    stepSimulation() {
        stateManager.simulator.step_simulation(this.simulationStepDelta);
        stateManager.incrementSimulationTime(this.simulationStepDelta);
        this.updateSimulationDisplay();
        this.updateTimer();
        
        // Update packet positions
        if (stateManager.packetVisualizer) {
            stateManager.packetVisualizer.update(stateManager.simulationTime);
        }
        
        canvasRenderer.render();
        
        // Update router details in real-time
        this.updateRoutersList();
    }
    
    updateTimer() {
        const timer = document.getElementById('simulation-timer');
        timer.textContent = `Time: ${stateManager.simulationTime.toFixed(1)}s`;
    }
    
    updateSimulationDisplay() {
        // Update routers and connections from simulator
        const routersJson = stateManager.simulator.get_routers_json();
        const connectionsJson = stateManager.simulator.get_connections_json();
        
        console.log('Routers from simulator:', routersJson);
        console.log('Connections from simulator:', connectionsJson);
        
        if (routersJson) {
            const newRouters = JSON.parse(routersJson);
            // Only update if there's a change in routers
            if (JSON.stringify(stateManager.routers) !== JSON.stringify(newRouters)) {
                console.log('Updating routers:', newRouters);
                stateManager.routers = newRouters;
                this.updateRoutersList();
            }
        }
        
        if (connectionsJson) {
            stateManager.connections = JSON.parse(connectionsJson);
        }
        
        // Update router summaries for display
        stateManager.routers.forEach(router => {
            const summaryJson = stateManager.simulator.get_router_summary_json(router.id);
            if (summaryJson) {
                router.summary = JSON.parse(summaryJson);
            }
        });
        
        // Get recent events and display them
        const eventsJson = stateManager.simulator.get_recent_events_json(50);
        const packetEvents = eventLogger.processSimulationEvents(eventsJson);
        
        // Add packet visualizations
        if (packetEvents && stateManager.packetVisualizer) {
            packetEvents.forEach(event => {
                if (event.type === 'packet') {
                    stateManager.packetVisualizer.addPacket(
                        event.from,
                        event.to,
                        event.packetType,
                        event.timestamp
                    );
                }
            });
        }
    }
    
    updateRoutersList() {
        const list = document.getElementById('routers-list');
        list.innerHTML = '';
        
        stateManager.routers.forEach(router => {
            const item = document.createElement('div');
            item.className = 'router-item' + (router.ospf_enabled ? ' ospf-enabled' : '');
            item.dataset.routerId = router.id;
            
            // Get detailed router information
            const details = routerManager.getRouterDetails(router.id);
            
            item.innerHTML = `
                <div class="router-header">
                    <div>
                        <strong>${router.name}</strong> (ID: ${router.id})
                        ${router.ospf_enabled ? 
                            `<span style="color: #4CAF50; font-size: 11px;"> • OSPF: ${details.ospf_neighbors || 0} neighbors</span>` : 
                            '<span style="color: #999; font-size: 11px;"> • OSPF Disabled</span>'
                        }
                    </div>
                </div>
                <button class="button" data-router-id="${router.id}" style="margin-top: 5px;">
                    ${router.ospf_enabled ? 'Disable' : 'Enable'} OSPF
                </button>
                <div class="router-details" style="display: block;">
                    ${routerManager.renderRouterDetails(details)}
                </div>
            `;
            list.appendChild(item);
            
            // Add event listener to button
            const button = item.querySelector('button');
            if (button) {
                button.addEventListener('click', () => {
                    routerManager.toggleOSPF(router.id);
                    this.updateRoutersList();
                });
            }
        });
    }
}

// Export as singleton
export default new SimulationController();