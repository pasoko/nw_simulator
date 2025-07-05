/**
 * Sidebar UI Module
 * Handles modern sidebar interactions and rendering
 */

import stateManager from './state-manager.js';
import eventLogger from './event-logger.js';

class SidebarUI {
    constructor() {
        this.collapsed = false;
        this.modeIcons = {
            'add-router': '➕',
            'move-router': '✋',
            'connect-routers': '🔗',
            'delete-router': '🗑️',
            'disconnect-routers': '✂️',
            'toggle-failure': '⚠️'
        };
        this.modeColors = {
            'add-router': '#2196F3',
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
                    <button class="action-button primary" id="simulate-btn">
                        <span>🚀</span>
                        <span>Start Simulation</span>
                    </button>
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
            { id: 'move-router', label: 'Move', icon: '✋', mode: 'move-router' },
            { id: 'connect-routers', label: 'Connect', icon: '🔗', mode: 'connect-routers' },
            { id: 'disconnect-routers', label: 'Disconnect', icon: '✂️', mode: 'disconnect-routers' },
            { id: 'delete-router', label: 'Delete', icon: '🗑️', mode: 'delete-router' },
            { id: 'toggle-failure', label: 'Failure', icon: '⚠️', mode: 'toggle-failure' }
        ];

        return tools.map(tool => `
            <button class="tool-button" id="${tool.id}-btn" data-mode="${tool.mode}">
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
                this.setMode(mode);
            });
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
            
            routerList.innerHTML = routers.map(router => this.createRouterCard(router)).join('');
            
            // Add click handlers for router cards
            const routerCards = routerList.querySelectorAll('.router-card');
            routerCards.forEach(card => {
                card.addEventListener('click', (e) => {
                    const routerId = parseInt(card.dataset.routerId);
                    this.handleRouterCardClick(routerId);
                });
            });
            
        } catch (error) {
            console.error('Error updating routers list:', error);
            routerList.innerHTML = '<p style="color: red;">Error loading routers</p>';
        }
    }

    createRouterCard(router) {
        const statusBadges = [];
        if (router.ospf_enabled) {
            statusBadges.push('<span class="status-badge status-ospf">OSPF</span>');
        }
        if (router.is_failed) {
            statusBadges.push('<span class="status-badge status-failed">FAILED</span>');
        }
        
        const classes = ['router-card'];
        if (router.ospf_enabled) classes.push('ospf-enabled');
        if (router.is_failed) classes.push('failed');
        
        // Get router details if available
        let detailsHtml = '';
        if (router.summary) {
            detailsHtml = `
                <div class="router-details">
                    <div class="detail-item">
                        <span class="detail-label">Neighbors</span>
                        <span class="detail-value">${router.summary.neighbor_count || 0}</span>
                    </div>
                    <div class="detail-item">
                        <span class="detail-label">Routes</span>
                        <span class="detail-value">${router.summary.route_count || 0}</span>
                    </div>
                </div>
            `;
        }
        
        return `
            <div class="${classes.join(' ')}" data-router-id="${router.id}">
                <div class="router-header">
                    <span class="router-name">${router.name} (${router.id})</span>
                    <div class="router-status">
                        ${statusBadges.join('')}
                    </div>
                </div>
                ${detailsHtml}
            </div>
        `;
    }

    handleRouterCardClick(routerId) {
        const mode = stateManager.getMode();
        
        // In certain modes, clicking a router card selects it
        if (mode === 'connect-routers' || mode === 'disconnect-routers') {
            // Trigger router selection
            const router = stateManager.findRouterById(routerId);
            if (router) {
                // Simulate click on canvas at router position
                const event = new CustomEvent('routerCardClicked', {
                    detail: { router }
                });
                window.dispatchEvent(event);
            }
        } else if (mode === 'toggle-failure') {
            // Toggle router failure directly
            if (stateManager.simulator) {
                stateManager.simulator.toggle_router_failure(routerId);
                this.updateRoutersList();
                eventLogger.log(`Toggled failure state for router ${routerId}`);
            }
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
}

// Export singleton instance
export default new SidebarUI();