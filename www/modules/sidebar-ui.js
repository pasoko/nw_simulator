/**
 * Sidebar UI Module
 * Handles modern sidebar interactions and rendering
 */

import stateManager from './state-manager.js';
import eventLogger from './event-logger.js';
import routerDetailsUI from './router-details-ui.js';

class SidebarUI {
    constructor() {
        this.collapsed = false;
        this.modeIcons = {
            'add-router': '➕',
            'add-terminal': '🖥️',
            'move-router': '✋',
            'connect-routers': '🔗',
            'delete-router': '🗑️',
            'disconnect-routers': '✂️',
            'toggle-failure': '⚠️'
        };
        this.modeColors = {
            'add-router': '#2196F3',
            'add-terminal': '#673AB7',
            'move-router': '#00BCD4',
            'connect-routers': '#4CAF50',
            'delete-router': '#F44336',
            'disconnect-routers': '#FF5722',
            'toggle-failure': '#FF9800'
        };
    }

    init() {
        this.setupSidebarStructure();
        this.setupEventListeners();
        this.updateModeDisplay(stateManager.getMode());
        routerDetailsUI.init();
    }

    setupSidebarStructure() {
        const sidebar = document.getElementById('sidebar');
        sidebar.innerHTML = `
            <!-- Header -->
            <div class="sidebar-header">
                <div class="app-logo">
                    <svg width="32" height="32" viewBox="0 0 32 32" fill="none">
                        <circle cx="8" cy="16" r="3" fill="#1976D2"/>
                        <circle cx="24" cy="8" r="3" fill="#1976D2"/>
                        <circle cx="24" cy="24" r="3" fill="#1976D2"/>
                        <path d="M11 16H21M21 16L19 11M21 16L19 21" stroke="#1976D2" stroke-width="2"/>
                    </svg>
                    <span>Network Simulator</span>
                </div>
                <button class="sidebar-toggle" id="sidebar-toggle">
                    <svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
                        <path d="M3 5h14M3 10h14M3 15h14" stroke="currentColor" stroke-width="2"/>
                    </svg>
                </button>
            </div>
            
            <!-- Mode Card -->
            <div class="mode-card" id="mode-card">
                <div class="mode-icon" id="mode-icon">➕</div>
                <div class="mode-info">
                    <span class="mode-label">Current Mode</span>
                    <span class="mode-value" id="mode-value">Add Router</span>
                </div>
            </div>
            
            <!-- Tools Section -->
            <div class="sidebar-section">
                <h3 class="section-title">
                    <span class="section-icon">🛠️</span>
                    <span>Tools</span>
                </h3>
                <div class="tool-grid" id="tool-grid">
                    ${this.createToolButtons()}
                </div>
            </div>
            
            <!-- Routers Section -->
            <div class="sidebar-section scrollable">
                <h3 class="section-title">
                    <span class="section-icon">📡</span>
                    <span>Routers</span>
                    <span class="router-count" id="router-count">0</span>
                </h3>
                <div class="router-list" id="router-list">
                    <!-- Router cards will be inserted here -->
                </div>
            </div>
            
            <!-- Footer Actions -->
            <div class="sidebar-footer">
                <div class="action-group">
                    <div style="display: flex; gap: 8px; align-items: center;">
                        <button class="speed-toggle-btn" id="speed-toggle-btn" title="Toggle simulation speed">
                            <span>×1</span>
                        </button>
                        <button class="action-button primary" id="simulate-btn" style="flex: 1;">
                            <span>🚀</span>
                            <span>Start Simulation</span>
                        </button>
                    </div>
                    <div style="display: flex; gap: 8px;">
                        <button class="action-button secondary" id="export-log-btn" style="flex: 1;">
                            <span>💾</span>
                            <span>Export</span>
                        </button>
                        <button class="action-button secondary" id="clear-log-btn" style="flex: 1;">
                            <span>🗑️</span>
                            <span>Clear</span>
                        </button>
                    </div>
                </div>
            </div>
        `;
    }

    createToolButtons() {
        const tools = [
            { id: 'add-router', label: 'Add', icon: '➕', mode: 'add-router' },
            { id: 'add-terminal', label: 'Terminal', icon: '🖥️', mode: 'add-terminal' },
            { id: 'move-router', label: 'Move', icon: '✋', mode: 'move-router' },
            { id: 'connect-routers', label: 'Connect', icon: '🔗', mode: 'connect-routers' },
            { id: 'disconnect-routers', label: 'Disconnect', icon: '✂️', mode: 'disconnect-routers' },
            { id: 'delete-router', label: 'Delete', icon: '🗑️', mode: 'delete-router' },
            { id: 'toggle-failure', label: 'Failure', icon: '⚠️', mode: 'toggle-failure' },
            { id: 'add-host', label: 'Host', icon: '💻', action: 'addHost' }
        ];

        return tools.map(tool => `
            <button class="tool-button" id="${tool.id}-btn" 
                    ${tool.mode ? `data-mode="${tool.mode}"` : ''}
                    ${tool.action ? `data-action="${tool.action}"` : ''}>
                <span class="tool-icon">${tool.icon}</span>
                <span class="tool-label">${tool.label}</span>
            </button>
        `).join('');
    }

    setupEventListeners() {
        // Sidebar toggle
        const toggleBtn = document.getElementById('sidebar-toggle');
        toggleBtn.addEventListener('click', () => this.toggleSidebar());

        // Tool buttons
        const toolButtons = document.querySelectorAll('.tool-button');
        toolButtons.forEach(btn => {
            btn.addEventListener('click', (e) => {
                const mode = e.currentTarget.dataset.mode;
                const action = e.currentTarget.dataset.action;
                
                if (mode) {
                    this.setMode(mode);
                } else if (action === 'addHost') {
                    window.hostManager.showAddHostDialog();
                }
            });
        });

        // Speed toggle button
        const speedToggleBtn = document.getElementById('speed-toggle-btn');
        speedToggleBtn.addEventListener('click', () => {
            this.toggleSimulationSpeed();
        });

        // Simulation button
        const simulateBtn = document.getElementById('simulate-btn');
        simulateBtn.addEventListener('click', () => {
            window.dispatchEvent(new CustomEvent('toggleSimulation'));
        });

        // Export/Clear buttons
        const exportBtn = document.getElementById('export-log-btn');
        exportBtn.addEventListener('click', () => {
            eventLogger.exportLog();
        });

        const clearBtn = document.getElementById('clear-log-btn');
        clearBtn.addEventListener('click', () => {
            eventLogger.clearLog();
        });
    }

    toggleSidebar() {
        const sidebar = document.getElementById('sidebar');
        this.collapsed = !this.collapsed;
        
        if (this.collapsed) {
            sidebar.classList.add('collapsed');
        } else {
            sidebar.classList.remove('collapsed');
        }
    }

    setMode(mode) {
        stateManager.setMode(mode);
        this.updateModeDisplay(mode);
        
        // Update active tool button
        const toolButtons = document.querySelectorAll('.tool-button');
        toolButtons.forEach(btn => {
            if (btn.dataset.mode === mode) {
                btn.classList.add('active');
            } else {
                btn.classList.remove('active');
            }
        });
    }

    updateModeDisplay(mode) {
        const modeIcon = document.getElementById('mode-icon');
        const modeValue = document.getElementById('mode-value');
        const modeCard = document.getElementById('mode-card');
        
        modeIcon.textContent = this.modeIcons[mode] || '❓';
        modeValue.textContent = this.getModeDisplayName(mode);
        
        // Update mode card color
        const color = this.modeColors[mode] || '#666';
        modeIcon.style.background = color;
        modeIcon.style.color = 'white';
    }

    getModeDisplayName(mode) {
        const names = {
            'add-router': 'Add Router',
            'add-terminal': 'Add Terminal',
            'move-router': 'Move Router',
            'connect-routers': 'Connect Routers',
            'delete-router': 'Delete Router',
            'disconnect-routers': 'Disconnect Routers',
            'toggle-failure': 'Toggle Failure'
        };
        return names[mode] || 'Unknown';
    }

    updateRoutersList() {
        const routerList = document.getElementById('router-list');
        const routerCount = document.getElementById('router-count');
        
        if (!stateManager.simulator) return;
        
        try {
            const routersJson = stateManager.simulator.get_routers_json();
            if (!routersJson) {
                routerList.innerHTML = '<p style="text-align: center; color: #999;">No routers</p>';
                routerCount.textContent = '0';
                return;
            }
            
            const routers = JSON.parse(routersJson);
            routerCount.textContent = routers.length;
            
            if (routers.length === 0) {
                routerList.innerHTML = '<p style="text-align: center; color: #999;">No routers</p>';
                return;
            }
            
            routerList.innerHTML = routers.map(router => routerDetailsUI.createRouterCard(router)).join('');
            
            // Trigger router list updated event for routerDetailsUI
            window.dispatchEvent(new CustomEvent('routerListUpdated'));
            
        } catch (error) {
            console.error('Error updating routers list:', error);
            routerList.innerHTML = '<p style="color: red;">Error loading routers</p>';
        }
    }



    updateSimulationButton(isRunning) {
        const btn = document.getElementById('simulate-btn');
        if (isRunning) {
            btn.innerHTML = '<span>⏸️</span><span>Stop Simulation</span>';
            btn.classList.add('running');
        } else {
            btn.innerHTML = '<span>🚀</span><span>Start Simulation</span>';
            btn.classList.remove('running');
        }
    }

    toggleSimulationSpeed() {
        const btn = document.getElementById('speed-toggle-btn');
        const stateManager = window.stateManager;
        
        if (stateManager.simulationSpeed === 1.0) {
            stateManager.simulationSpeed = 0.1;
            btn.innerHTML = '<span>×0.1</span>';
            btn.classList.add('slow');
        } else {
            stateManager.simulationSpeed = 1.0;
            btn.innerHTML = '<span>×1</span>';
            btn.classList.remove('slow');
        }
        
        // Dispatch event to notify other components
        window.dispatchEvent(new CustomEvent('simulationSpeedChanged', { 
            detail: { speed: stateManager.simulationSpeed } 
        }));
    }
}

// Export singleton instance
export default new SidebarUI();