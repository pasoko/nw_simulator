/**
 * Sidebar UI Module
 * 
 * Handles modern sidebar interactions and rendering including:
 * - Router list display with real-time updates
 * - Router details panels with tab navigation
 * - Event logging and export functionality
 * - Router/terminal creation and management
 * - Flicker-free differential DOM updates
 * - Event delegation for dynamic content
 * - Mode switching and visual feedback
 * 
 * Recent improvements (2025-07):
 * - Differential updates to prevent screen flickering
 * - Real-time data synchronization from WebAssembly simulator
 * - Optimized rendering with requestAnimationFrame
 * - Automatic updates without requiring user interaction
 * - Event delegation to avoid event handler duplication
 */

import stateManager from './state-manager.js';
import eventLogger from './event-logger.js';
import routerDetailsUI from './router-details-ui.js';

class SidebarUI {
    constructor() {
        this.collapsed = false;  // サイドバーの折りたたみ状態
        this.updateDebounceTimer = null;  // 更新デバウンス用タイマー
        this.lastRoutersJson = null;  // 前回のルーター情報（未使用）
        this.isMouseDown = false;  // マウスダウン状態の追跡
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
        this.setupEventDelegation();
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
                
                console.log('Tool button clicked:', { mode, action, buttonId: e.currentTarget.id });
                
                if (mode) {
                    console.log('Setting mode to:', mode);
                    this.setMode(mode);
                    
                    // Special handling for add-terminal mode
                    if (mode === 'add-terminal') {
                        console.log('add-terminal mode activated - waiting for canvas click');
                        // Do NOT show dialog here - wait for canvas click
                        return;
                    }
                } else if (action === 'addHost') {
                    console.log('Host action triggered - showing host dialog');
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

    /**
     * イベント委譲パターンの設定
     * 動的に生成されるルーター要素に対して、
     * イベントハンドラの重複を避けるため委譲を使用
     */
    setupEventDelegation() {
        // Set up event delegation for router list
        const routerList = document.getElementById('router-list');
        if (!routerList) return;
        
        // Track mouse down/up for interaction detection
        document.addEventListener('mousedown', () => {
            this.isMouseDown = true;
        });
        
        document.addEventListener('mouseup', () => {
            this.isMouseDown = false;
        });
        
        // Handle all router card clicks
        routerList.addEventListener('click', (e) => {
            const target = e.target;
            
            // Handle router header clicks
            const clickableHeader = target.closest('.router-header-clickable');
            if (clickableHeader) {
                const routerId = parseInt(clickableHeader.dataset.routerId);
                if (routerId) {
                    e.preventDefault();
                    e.stopPropagation();
                    routerDetailsUI.toggleRouterDetails(routerId);
                }
                return;
            }
            
            // Handle config button clicks
            const configBtn = target.closest('.router-config-btn');
            if (configBtn) {
                const routerId = parseInt(configBtn.dataset.routerId);
                if (routerId) {
                    e.preventDefault();
                    e.stopPropagation();
                    routerDetailsUI.openRouterConfig(routerId);
                }
                return;
            }
            
            // Handle tab clicks
            const tab = target.closest('.tab-button');
            if (tab) {
                const routerId = parseInt(tab.dataset.routerId);
                const tabName = tab.dataset.tab;
                if (routerId && tabName) {
                    e.preventDefault();
                    e.stopPropagation();
                    routerDetailsUI.switchTab(routerId, tabName);
                }
            }
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
        console.log('SidebarUI.setMode called with:', mode);
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
        console.log('Mode set to:', mode, 'Active button:', document.querySelector('.tool-button.active')?.id);
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

    /**
     * ルーターリスト表示の更新
     * このメソッドは定期的（2秒ごと）およびオンデマンドで呼び出される
     * デバウンス処理により高頻度の再描画を防止
     */
    updateRoutersList() {
        // Clear any pending debounce timer
        if (this.updateDebounceTimer) {
            clearTimeout(this.updateDebounceTimer);
        }
        
        // Debounce updates to prevent rapid re-renders
        this.updateDebounceTimer = setTimeout(() => {
            this._performRouterListUpdate();
        }, 100);
    }

    /**
     * 実際のルーターリスト更新処理
     * WebAssemblyシミュレータからデータを取得し、
     * 差分更新を実行してちらつきを防止
     * 
     * @private
     */
    _performRouterListUpdate() {
        const routerList = document.getElementById('router-list');
        const routerCount = document.getElementById('router-count');
        
        if (!stateManager.simulator) return;
        
        try {
            const routersJson = stateManager.simulator.get_routers_json();
            
            // Remove the JSON comparison check to ensure updates are always processed
            // The differential update will handle unnecessary re-renders
            // if (routersJson === this.lastRoutersJson) {
            //     return;
            // }
            // this.lastRoutersJson = routersJson;
            
            if (!routersJson) {
                if (routerList.children.length !== 1 || !routerList.querySelector('p')) {
                    routerList.innerHTML = '<p style="text-align: center; color: #999;">No routers</p>';
                }
                routerCount.textContent = '0';
                return;
            }
            
            const routers = JSON.parse(routersJson);
            routerCount.textContent = routers.length;
            
            if (routers.length === 0) {
                if (routerList.children.length !== 1 || !routerList.querySelector('p')) {
                    routerList.innerHTML = '<p style="text-align: center; color: #999;">No routers</p>';
                }
                return;
            }
            
            // Use differential update to prevent flickering
            this.updateRoutersDifferentially(routerList, routers);
            
            // Trigger router list updated event for routerDetailsUI
            window.dispatchEvent(new CustomEvent('routerListUpdated'));
            
        } catch (error) {
            console.error('Error updating routers list:', error);
            routerList.innerHTML = '<p style="color: red;">Error loading routers</p>';
        }
    }

    updateRoutersDifferentially(routerList, routers) {
        // Use RequestAnimationFrame for smooth updates
        requestAnimationFrame(() => {
            this._performDifferentialUpdate(routerList, routers);
        });
    }

    isUserInteracting(element) {
        // Check if user is specifically interacting with this element
        // Allow updates to other elements
        const focusedElement = document.activeElement;
        
        // Check if user is typing in an input field
        if (focusedElement && (focusedElement.tagName === 'INPUT' || focusedElement.tagName === 'TEXTAREA')) {
            return element.contains(focusedElement);
        }
        
        // Check if user is selecting text
        const selection = window.getSelection();
        if (selection && selection.toString().length > 0) {
            const range = selection.getRangeAt(0);
            return element.contains(range.commonAncestorContainer);
        }
        
        // Check if mouse is down (dragging)
        if (this.isMouseDown) {
            return true;
        }
        
        return false;
    }

    _performDifferentialUpdate(routerList, routers) {
        const existingCards = new Map();
        const cardElements = routerList.querySelectorAll('.router-card');
        
        // Build map of existing cards
        cardElements.forEach(card => {
            const routerId = card.dataset.routerId;
            if (routerId) {
                existingCards.set(routerId, card);
            }
        });
        
        // Track which routers we've seen
        const processedIds = new Set();
        
        // Update or create cards
        routers.forEach((router, index) => {
            processedIds.add(router.id);
            
            let card = existingCards.get(router.id);
            if (card) {
                // Update existing card without recreating DOM
                this.updateRouterCard(card, router);
                
                // Ensure correct position
                const expectedIndex = index;
                const currentIndex = Array.from(routerList.children).indexOf(card);
                if (currentIndex !== expectedIndex) {
                    const referenceNode = routerList.children[expectedIndex];
                    routerList.insertBefore(card, referenceNode);
                }
            } else {
                // Create new card
                card = this.createRouterCardElement(router);
                
                // Insert at correct position
                if (index < routerList.children.length) {
                    routerList.insertBefore(card, routerList.children[index]);
                } else {
                    routerList.appendChild(card);
                }
            }
        });
        
        // Remove cards for deleted routers
        existingCards.forEach((card, routerId) => {
            if (!processedIds.has(routerId)) {
                // Fade out before removing
                card.style.opacity = '0';
                setTimeout(() => card.remove(), 300);
            }
        });
    }

    updateRouterCard(card, router) {
        // Always update - no longer skip for user interaction
        
        // Update only the parts that have changed
        const nameElement = card.querySelector('.router-name');
        const currentName = `${router.name} (ID: ${router.id})`;
        if (nameElement && nameElement.textContent !== currentName) {
            nameElement.textContent = currentName;
        }
        
        // Update status badges without recreating them
        const statusContainer = card.querySelector('.router-status');
        if (statusContainer) {
            this.updateStatusBadges(statusContainer, router);
        }
        
        // Update classes
        const newClasses = routerDetailsUI.getRouterClasses(router);
        card.className = newClasses.join(' ');
        
        // Update expand icon
        const expandIcon = card.querySelector('.expand-icon');
        if (expandIcon) {
            expandIcon.textContent = routerDetailsUI.expandedRouters.has(router.id) ? '▼' : '▶';
        }
        
        // Always update expanded content if expanded (real-time updates)
        if (routerDetailsUI.expandedRouters.has(router.id)) {
            const contentElement = card.querySelector('.router-content');
            if (contentElement && contentElement.classList.contains('expanded')) {
                // Force update content for real-time data
                this.updateExpandedContent(contentElement, router.id);
            }
        }
    }

    updateStatusBadges(container, router) {
        const badges = [];
        if (router.ospf_enabled) badges.push('OSPF');
        if (router.is_failed) badges.push('FAILED');
        if (router.is_dr) badges.push('DR');
        if (router.is_bdr) badges.push('BDR');
        
        // Only update if badges have changed
        const currentBadges = Array.from(container.querySelectorAll('.status-badge'))
            .map(badge => badge.textContent);
        
        if (JSON.stringify(currentBadges) !== JSON.stringify(badges)) {
            // Clear and recreate badges
            container.innerHTML = '';
            badges.forEach(badge => {
                const span = document.createElement('span');
                span.className = 'status-badge';
                if (badge === 'FAILED') span.classList.add('status-failed');
                else if (badge === 'DR') span.classList.add('status-dr');
                else if (badge === 'BDR') span.classList.add('status-bdr');
                else span.classList.add('status-ospf');
                span.textContent = badge;
                container.appendChild(span);
            });
        }
    }

    updateExpandedContent(contentElement, routerId) {
        // Check if the content element exists and has the router details structure
        const routerTabs = contentElement.querySelector('.router-tabs');
        const tabContent = contentElement.querySelector('.tab-content');
        
        if (!routerTabs || !tabContent) {
            // Full rebuild if structure is missing
            contentElement.innerHTML = routerDetailsUI.createRouterDetailsContent(routerId);
            return;
        }
        
        // Preserve active tab and scroll position
        const activeTab = routerDetailsUI.activeTab.get(routerId) || 'summary';
        const scrollTop = tabContent.scrollTop;
        
        // Update only the tab content, not the entire structure
        const newTabContent = routerDetailsUI.getTabContent(routerId, activeTab);
        tabContent.innerHTML = newTabContent;
        tabContent.scrollTop = scrollTop;
        
        // Update tab button states
        const tabButtons = routerTabs.querySelectorAll('.tab-button');
        tabButtons.forEach(btn => {
            const isActive = btn.dataset.tab === activeTab;
            btn.classList.toggle('active', isActive);
        });
    }

    createRouterCardElement(router) {
        const card = document.createElement('div');
        card.className = routerDetailsUI.getRouterClasses(router).join(' ');
        card.dataset.routerId = router.id;
        
        // Create header
        const header = document.createElement('div');
        header.className = 'router-header';
        
        const clickable = document.createElement('div');
        clickable.className = 'router-header-clickable';
        clickable.dataset.routerId = router.id;
        
        const headerLeft = document.createElement('div');
        headerLeft.className = 'router-header-left';
        
        const expandIcon = document.createElement('span');
        expandIcon.className = 'expand-icon';
        expandIcon.textContent = routerDetailsUI.expandedRouters.has(router.id) ? '▼' : '▶';
        
        const nameSpan = document.createElement('span');
        nameSpan.className = 'router-name';
        nameSpan.textContent = `${router.name} (ID: ${router.id})`;
        
        headerLeft.appendChild(expandIcon);
        headerLeft.appendChild(nameSpan);
        
        const statusDiv = document.createElement('div');
        statusDiv.className = 'router-status';
        this.updateStatusBadges(statusDiv, router);
        
        clickable.appendChild(headerLeft);
        clickable.appendChild(statusDiv);
        
        const configBtn = document.createElement('button');
        configBtn.className = 'router-config-btn';
        configBtn.dataset.routerId = router.id;
        configBtn.title = 'Router Configuration';
        configBtn.innerHTML = `
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="3"/>
                <path d="M12 1v6m0 6v6m6.364-15.364l-4.243 4.243m-4.242 4.242l-4.243 4.243m20.364-6.364h-6m-6 0h-6m15.364 6.364l-4.243-4.243m-4.242-4.242l-4.243-4.243"/>
            </svg>
        `;
        
        header.appendChild(clickable);
        header.appendChild(configBtn);
        
        const content = document.createElement('div');
        content.className = `router-content ${routerDetailsUI.expandedRouters.has(router.id) ? 'expanded' : 'collapsed'}`;
        content.id = `router-content-${router.id}`;
        
        if (routerDetailsUI.expandedRouters.has(router.id)) {
            content.innerHTML = routerDetailsUI.createRouterDetailsContent(router.id);
        }
        
        card.appendChild(header);
        card.appendChild(content);
        
        return card;
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

// Create singleton instance
const sidebarUI = new SidebarUI();

// Make it globally accessible
window.sidebarUI = sidebarUI;

// Export singleton instance
export default sidebarUI;