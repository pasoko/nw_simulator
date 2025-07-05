/**
 * UI Controller Module
 * Manages UI mode states, button interactions, and DOM updates
 */

import stateManager from './state-manager.js';
import eventLogger from './event-logger.js';
import sidebarUI from './sidebar-ui.js';

class UIController {
    constructor() {
        this.mode = 'add-router';
        this.selectedRouters = [];
    }

    init() {
        // Initialize modern sidebar UI
        sidebarUI.init();
        
        this.setupEventListeners();
        this.setMode('add-router');
    }

    setupEventListeners() {
        // Mode buttons
        const addBtn = document.getElementById('add-router-btn');
        if (addBtn) {
            addBtn.addEventListener('click', () => {
                this.setMode('add-router');
            });
        }
        
        const moveBtn = document.getElementById('move-router-btn');
        if (moveBtn) {
            moveBtn.addEventListener('click', () => {
                this.setMode('move-router');
            });
        }
        
        const connectBtn = document.getElementById('connect-routers-btn');
        if (connectBtn) {
            connectBtn.addEventListener('click', () => {
                this.setMode('connect-routers');
            });
        }
        
        // Simulation button event listener is now handled in sidebar-ui.js to avoid duplication
        
        const exportBtn = document.getElementById('export-log-btn');
        if (exportBtn) {
            exportBtn.addEventListener('click', () => {
                this.exportLog();
            });
        }
        
        const clearBtn = document.getElementById('clear-log-btn');
        if (clearBtn) {
            clearBtn.addEventListener('click', () => {
                this.clearLog();
            });
        }
        
        const deleteBtn = document.getElementById('delete-router-btn');
        if (deleteBtn) {
            deleteBtn.addEventListener('click', () => {
                this.setMode('delete-router');
            });
        }
        
        const disconnectBtn = document.getElementById('disconnect-routers-btn');
        if (disconnectBtn) {
            disconnectBtn.addEventListener('click', () => {
                this.setMode('disconnect-routers');
            });
        }
        
        const toggleFailureBtn = document.getElementById('toggle-failure-btn');
        if (toggleFailureBtn) {
            toggleFailureBtn.addEventListener('click', () => {
                this.setMode('toggle-failure');
            });
        }
    }

    setMode(newMode) {
        this.mode = newMode;
        this.selectedRouters = [];
        const canvas = stateManager.canvas;
        
        // Update cursor based on mode
        switch(this.mode) {
            case 'add-router':
                canvas.style.cursor = 'crosshair';
                break;
            case 'move-router':
                canvas.style.cursor = 'grab';
                break;
            default:
                canvas.style.cursor = 'pointer';
                break;
        }
        
        // Update sidebar UI mode display
        sidebarUI.setMode(newMode);
        
        // Update state manager
        stateManager.setMode(newMode);
        
        // Trigger re-render
        if (stateManager.canvasRenderer) {
            stateManager.canvasRenderer.render();
        }
    }

    getMode() {
        return this.mode;
    }

    getSelectedRouters() {
        return this.selectedRouters;
    }

    selectRouter(router) {
        this.selectedRouters.push(router);
    }

    clearSelectedRouters() {
        this.selectedRouters = [];
    }

    updateTimer(simulationTime) {
        const timer = document.getElementById('simulation-timer');
        if (timer) {
            if (stateManager.simulationPaused) {
                timer.textContent = `Time: ${simulationTime.toFixed(1)}s (Paused)`;
            } else {
                timer.textContent = `Time: ${simulationTime.toFixed(1)}s`;
            }
        }
    }

    showTimer() {
        const timer = document.getElementById('simulation-timer');
        if (timer) {
            timer.style.display = 'block';
        }
    }

    updateSimulationButton(isRunning) {
        // Update sidebar UI simulation button
        sidebarUI.updateSimulationButton(isRunning);
    }

    exportLog() {
        if (!stateManager.simulator) return;
        
        const eventsJson = stateManager.simulator.get_all_events_json();
        if (eventsJson) {
            const events = JSON.parse(eventsJson);
            const dataStr = JSON.stringify(events, null, 2);
            const dataBlob = new Blob([dataStr], {type: 'application/json'});
            
            const link = document.createElement('a');
            link.href = URL.createObjectURL(dataBlob);
            link.download = `ospf_simulation_log_${new Date().toISOString().slice(0, 19).replace(/:/g, '-')}.json`;
            document.body.appendChild(link);
            link.click();
            document.body.removeChild(link);
            
            eventLogger.log('Event log exported successfully');
        }
    }

    clearLog() {
        eventLogger.clearLog();
    }

    updateRoutersList() {
        // Delegate to sidebar UI for modern router list
        sidebarUI.updateRoutersList();
    }

    createRouterListElement(router) {
        const div = document.createElement('div');
        div.className = 'router-item';
        div.innerHTML = `
            <div class="router-header">
                <span class="router-name">${router.name} (ID: ${router.id})</span>
                <div class="router-status">
                    ${router.ospf_enabled ? '<span class="status-ospf">OSPF</span>' : ''}
                    ${router.is_failed ? '<span class="status-failed">FAILED</span>' : '<span class="status-active">ACTIVE</span>'}
                </div>
            </div>
            <div class="router-details" id="router-details-${router.id}">
                <div class="loading">Loading details...</div>
            </div>
        `;
        
        // Load detailed router information immediately
        this.loadRouterDetails(router.id);
        
        return div;
    }

    updateExistingRouterDetails(router) {
        // Update only the details of existing router elements to prevent flicker
        const existingElement = document.querySelector(`[data-router-id="${router.id}"]`);
        if (existingElement) {
            // Update status badges
            const statusDiv = existingElement.querySelector('.router-status');
            if (statusDiv) {
                statusDiv.innerHTML = `
                    ${router.ospf_enabled ? '<span class="status-ospf">OSPF</span>' : ''}
                    ${router.is_failed ? '<span class="status-failed">FAILED</span>' : '<span class="status-active">ACTIVE</span>'}
                `;
            }
            
            // Update router details
            this.loadRouterDetails(router.id);
        }
    }

    async loadRouterDetails(routerId) {
        if (!stateManager.simulator) return;
        
        const detailsContainer = document.getElementById(`router-details-${routerId}`);
        if (!detailsContainer) return;
        
        try {
            // Ensure routerId is a number
            const numericRouterId = typeof routerId === 'string' ? parseInt(routerId, 10) : routerId;
            
            const summaryJson = stateManager.simulator.get_router_summary_json(numericRouterId);
            const detailsJson = stateManager.simulator.get_router_details_json(numericRouterId);
            
            if (summaryJson && detailsJson) {
                const summary = JSON.parse(summaryJson);
                const details = JSON.parse(detailsJson);
                
                const newContent = `
                    <div class="detail-row">
                        <span class="detail-label">OSPF Neighbors:</span>
                        <span class="detail-value">${summary.neighbor_count || 0}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">Routing Table Entries:</span>
                        <span class="detail-value">${summary.route_count || 0}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">LSA Database Size:</span>
                        <span class="detail-value">${details.lsa_database_size || 0}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">Latest Event:</span>
                        <span class="detail-value">${summary.latest_event || 'None'}</span>
                    </div>
                `;
                
                // Only update if content changed to prevent unnecessary DOM manipulation
                if (detailsContainer.innerHTML !== newContent) {
                    detailsContainer.innerHTML = newContent;
                }
            } else {
                detailsContainer.innerHTML = '<div class="error">No data available</div>';
            }
        } catch (error) {
            console.error(`Error loading details for router ${routerId}:`, error);
            detailsContainer.innerHTML = '<div class="error">Error loading details</div>';
        }
    }
}

// Create singleton instance
const uiController = new UIController();

export default uiController;