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
            case 'add-terminal':
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
}

// Create singleton instance
const uiController = new UIController();

export default uiController;