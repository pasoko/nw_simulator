/**
 * Canvas Interaction Module
 * Handles mouse events, dragging, clicking, and canvas interactions
 */

import stateManager from './state-manager.js';
import routerManager from './router-manager.js';
import connectionManager from './connection-manager.js';
import uiController from './ui-controller.js';
import eventLogger from './event-logger.js';
import animationEffects from './animation-effects.js';

class CanvasInteraction {
    constructor() {
        this.lastMouseX = 0;
        this.lastMouseY = 0;
        this.draggingRouter = null;
        this.draggingTerminal = null;
        this.dragOffset = { x: 0, y: 0 };
        this.hoveredRouter = null;
        this.hoveredTerminal = null;
    }

    init(canvas) {
        this.setupCanvasEventListeners(canvas);
        // Initial update of terminals list
        this.updateTerminalsFromSimulator();
    }

    setupCanvasEventListeners(canvas) {
        canvas.addEventListener('mousedown', (event) => this.handleMouseDown(event));
        canvas.addEventListener('mousemove', (event) => this.handleMouseMove(event));
        canvas.addEventListener('mouseup', (event) => this.handleMouseUp(event));
        canvas.addEventListener('click', (event) => this.handleClick(event));
        
        // Prevent context menu on right click
        canvas.addEventListener('contextmenu', (event) => {
            event.preventDefault();
        });
    }

    handleMouseDown(event) {
        const rect = stateManager.canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;
        
        // Check if clicking on a router or terminal for dragging (only in move-router mode)
        const clickedRouter = this.findRouterAt(x, y);
        const clickedTerminal = this.findTerminalAt(x, y);
        
        if (uiController.getMode() === 'move-router') {
            if (clickedRouter) {
                this.draggingRouter = clickedRouter;
                this.dragOffset.x = x - clickedRouter.x;
                this.dragOffset.y = y - clickedRouter.y;
                stateManager.canvas.style.cursor = 'grabbing';
                event.preventDefault(); // Prevent text selection
            } else if (clickedTerminal) {
                this.draggingTerminal = clickedTerminal;
                this.dragOffset.x = x - clickedTerminal.x;
                this.dragOffset.y = y - clickedTerminal.y;
                stateManager.canvas.style.cursor = 'grabbing';
                event.preventDefault(); // Prevent text selection
            }
        }
    }

    handleMouseMove(event) {
        const rect = stateManager.canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;
        
        this.lastMouseX = x;
        this.lastMouseY = y;
        
        // Handle hover effects
        const router = this.findRouterAt(x, y);
        const terminal = this.findTerminalAt(x, y);
        
        if (router && router !== this.hoveredRouter) {
            if (this.hoveredRouter) {
                animationEffects.stopRouterHover(this.hoveredRouter.id);
            }
            this.hoveredRouter = router;
            animationEffects.animateRouterHover(stateManager.canvasRenderer.ctx, router);
        } else if (!router && this.hoveredRouter) {
            animationEffects.stopRouterHover(this.hoveredRouter.id);
            this.hoveredRouter = null;
        }
        
        // Update cursor
        const mode = uiController.getMode();
        if (router || terminal) {
            stateManager.canvas.style.cursor = mode === 'move-router' ? 'grab' : 'pointer';
        } else {
            stateManager.canvas.style.cursor = (mode === 'add-router' || mode === 'add-terminal') ? 'crosshair' : 'default';
        }
        
        // Handle router dragging
        if (this.draggingRouter && uiController.getMode() === 'move-router') {
            const newX = x - this.dragOffset.x;
            const newY = y - this.dragOffset.y;
            
            // Update router position
            this.draggingRouter.x = newX;
            this.draggingRouter.y = newY;
            
            // Update simulator with new position
            if (stateManager.simulator) {
                stateManager.simulator.update_router_position(this.draggingRouter.id, newX, newY);
            }
            
            // Re-render canvas
            if (stateManager.canvasRenderer) {
                stateManager.canvasRenderer.render();
            }
        }
        
        // Handle terminal dragging
        if (this.draggingTerminal && uiController.getMode() === 'move-router') {
            const newX = x - this.dragOffset.x;
            const newY = y - this.dragOffset.y;
            
            // Update terminal position
            this.draggingTerminal.x = newX;
            this.draggingTerminal.y = newY;
            
            // Update simulator with new position
            if (stateManager.simulator) {
                stateManager.simulator.update_terminal_position(this.draggingTerminal.id, newX, newY);
            }
            
            // Re-render canvas
            if (stateManager.canvasRenderer) {
                stateManager.canvasRenderer.render();
            }
        }
    }

    handleMouseUp(event) {
        if (this.draggingRouter) {
            this.draggingRouter = null;
            stateManager.canvas.style.cursor = 'grab';
        }
        if (this.draggingTerminal) {
            this.draggingTerminal = null;
            stateManager.canvas.style.cursor = 'grab';
        }
    }

    handleClick(event) {
        // Don't handle click if we were dragging
        if (this.draggingRouter || this.draggingTerminal) {
            return;
        }
        
        const rect = stateManager.canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;
        
        // Dispatch canvas click event for other modules (like host-manager)
        window.dispatchEvent(new CustomEvent('canvasClick', {
            detail: { x, y }
        }));
        
        // Check for double-click to show router details
        if (event.detail === 2) {
            const router = this.findRouterAt(x, y);
            if (router) {
                this.showRouterDetails(router);
                return;
            }
        }
        
        const mode = uiController.getMode();
        
        switch (mode) {
            case 'add-router':
                this.handleAddRouter(x, y);
                break;
            case 'add-terminal':
                this.handleAddTerminal(x, y);
                break;
            case 'connect-routers':
                this.handleConnectRouters(x, y);
                break;
            case 'delete-router':
                this.handleDeleteRouter(x, y);
                break;
            case 'disconnect-routers':
                this.handleDisconnectRouters(x, y);
                break;
            case 'toggle-failure':
                this.handleToggleFailure(x, y);
                break;
        }
    }

    handleAddRouter(x, y) {
        // Check if there's already a router at this position
        const existingRouter = this.findRouterAt(x, y);
        if (existingRouter) {
            eventLogger.log('Cannot place router: position already occupied');
            return;
        }
        
        const routerName = prompt('Enter router name:');
        if (!routerName) return;
        
        if (stateManager.simulator) {
            const routerId = stateManager.simulator.add_router(routerName, x, y);
            eventLogger.log(`Router "${routerName}" added with ID ${routerId} at (${x.toFixed(0)}, ${y.toFixed(0)})`);
            
            // Update routers list
            this.updateRoutersFromSimulator();
            
            // Enable OSPF by default
            stateManager.simulator.enable_ospf(routerId);
            eventLogger.log(`OSPF enabled on router "${routerName}"`);
            
            // Update UI
            uiController.updateRoutersList();
            
            // Re-render
            if (stateManager.canvasRenderer) {
                stateManager.canvasRenderer.render();
            }
        }
    }

    handleAddTerminal(x, y) {
        // Check if there's already a router or terminal at this position
        const existingRouter = this.findRouterAt(x, y);
        const existingTerminal = this.findTerminalAt(x, y);
        if (existingRouter || existingTerminal) {
            eventLogger.log('Cannot place terminal: position already occupied');
            return;
        }
        
        const terminalName = prompt('Enter terminal name:');
        if (!terminalName) return;
        
        const ipAddress = prompt('Enter IP address (e.g., 192.168.1.100):');
        if (!ipAddress) return;
        
        const netmask = prompt('Enter netmask (e.g., 255.255.255.0):', '255.255.255.0');
        if (!netmask) return;
        
        const defaultGateway = prompt('Enter default gateway (e.g., 192.168.1.1):');
        if (!defaultGateway) return;
        
        if (stateManager.simulator) {
            try {
                const terminalId = stateManager.simulator.add_terminal(terminalName, ipAddress, netmask, defaultGateway, x, y);
                eventLogger.log(`Terminal "${terminalName}" added with ID ${terminalId} at (${x.toFixed(0)}, ${y.toFixed(0)})`);
                
                // Update terminals and connections list
                this.updateTerminalsFromSimulator();
                this.updateConnectionsFromSimulator();
                
                // Update UI
                uiController.updateRoutersList();
                
                // Re-render
                if (stateManager.canvasRenderer) {
                    stateManager.canvasRenderer.render();
                }
            } catch (error) {
                eventLogger.log(`Failed to add terminal: ${error}`);
            }
        }
    }

    handleConnectRouters(x, y) {
        const clickedRouter = this.findRouterAt(x, y);
        const clickedTerminal = this.findTerminalAt(x, y);
        
        if (!clickedRouter && !clickedTerminal) return;
        
        const selectedRouters = uiController.getSelectedRouters();
        
        if (selectedRouters.length === 0) {
            // Select first device (router or terminal)
            const selectedDevice = clickedRouter || clickedTerminal;
            uiController.selectRouter(selectedDevice);
            eventLogger.log(`Selected ${clickedRouter ? 'router' : 'terminal'} "${selectedDevice.name}" as first device`);
            
            // Add selection animation for routers only
            if (clickedRouter) {
                animationEffects.animateRouterSelection(stateManager.canvasRenderer.ctx, clickedRouter);
            }
            
            // Update mode indicator - removed as it no longer exists in modern UI
        } else if (selectedRouters.length === 1) {
            // Connect to second device
            const firstDevice = selectedRouters[0];
            const secondDevice = clickedRouter || clickedTerminal;
            
            if (firstDevice.id === secondDevice.id) {
                eventLogger.log('Cannot connect device to itself');
                return;
            }
            
            // Determine connection type
            const firstIsRouter = stateManager.routers.some(r => r.id === firstDevice.id);
            const secondIsRouter = clickedRouter !== null;
            
            // Handle different connection types
            if (firstIsRouter && secondIsRouter) {
                // Router to Router connection
                this.connectRouters(firstDevice, secondDevice);
            } else if (!firstIsRouter && secondIsRouter) {
                // Terminal to Router connection
                this.connectTerminalToRouter(firstDevice, secondDevice);
            } else if (firstIsRouter && !secondIsRouter) {
                // Router to Terminal connection
                this.connectTerminalToRouter(secondDevice, firstDevice);
            } else {
                eventLogger.log('Cannot connect terminal to terminal');
                uiController.clearSelectedRouters();
                return;
            }
            
            // Reset selection
            uiController.clearSelectedRouters();
            uiController.setMode('connect-routers');
        }
    }
    
    connectRouters(firstRouter, secondRouter) {
        if (firstRouter.id === secondRouter.id) {
            eventLogger.log('Cannot connect router to itself');
            return;
        }
        
        // Check if connection already exists
        const existingConnection = stateManager.connections.find(conn => 
            (conn.from_router_id === firstRouter.id && conn.to_router_id === secondRouter.id) ||
            (conn.from_router_id === secondRouter.id && conn.to_router_id === firstRouter.id)
        );
        
        if (existingConnection) {
            eventLogger.log('Connection already exists between these routers');
            return;
        }
        
        const cost = parseInt(prompt('Enter link cost (1-100):', '10'));
        if (isNaN(cost) || cost < 1 || cost > 100) {
            eventLogger.log('Invalid cost. Please enter a number between 1 and 100.');
            return;
        }
        
        if (stateManager.simulator) {
            stateManager.simulator.connect_routers(firstRouter.id, secondRouter.id, cost);
            eventLogger.log(`Connected "${firstRouter.name}" to "${secondRouter.name}" with cost ${cost}`);
            
            // Update connections list
            this.updateConnectionsFromSimulator();
            
            // Re-render
            if (stateManager.canvasRenderer) {
                stateManager.canvasRenderer.render();
            }
        }
    }
    
    connectTerminalToRouter(terminal, router) {
        if (stateManager.simulator) {
            try {
                stateManager.simulator.connect_terminal_to_router(terminal.id, router.id);
                eventLogger.log(`Connected terminal "${terminal.name}" to router "${router.name}"`);
                
                // Update connections and terminals list
                this.updateConnectionsFromSimulator();
                this.updateTerminalsFromSimulator();
                
                // Re-render
                if (stateManager.canvasRenderer) {
                    stateManager.canvasRenderer.render();
                }
            } catch (error) {
                eventLogger.log(`Failed to connect terminal to router: ${error}`);
            }
        }
    }

    handleDeleteRouter(x, y) {
        const clickedRouter = this.findRouterAt(x, y);
        if (!clickedRouter) return;
        
        const confirmDelete = confirm(`Are you sure you want to delete router "${clickedRouter.name}"?`);
        if (!confirmDelete) return;
        
        if (stateManager.simulator) {
            const success = stateManager.simulator.delete_router(clickedRouter.id);
            if (success) {
                eventLogger.log(`Router "${clickedRouter.name}" deleted`);
                
                // Update routers and connections
                this.updateRoutersFromSimulator();
                this.updateConnectionsFromSimulator();
                
                // Update UI
                uiController.updateRoutersList();
                
                // Re-render
                if (stateManager.canvasRenderer) {
                    stateManager.canvasRenderer.render();
                }
            } else {
                eventLogger.log(`Failed to delete router "${clickedRouter.name}"`);
            }
        }
    }

    handleDisconnectRouters(x, y) {
        const clickedRouter = this.findRouterAt(x, y);
        if (!clickedRouter) return;
        
        const selectedRouters = uiController.getSelectedRouters();
        
        if (selectedRouters.length === 0) {
            // Select first router
            uiController.selectRouter(clickedRouter);
            eventLogger.log(`Selected router "${clickedRouter.name}" as first router`);
            
            // Update mode indicator
            const indicator = document.getElementById('mode-indicator');
            indicator.textContent = 'Mode: Disconnect Routers - Select second router';
        } else if (selectedRouters.length === 1) {
            // Disconnect from second router
            const firstRouter = selectedRouters[0];
            
            if (firstRouter.id === clickedRouter.id) {
                eventLogger.log('Cannot disconnect router from itself');
                return;
            }
            
            if (stateManager.simulator) {
                const success = stateManager.simulator.disconnect_routers(firstRouter.id, clickedRouter.id);
                if (success) {
                    eventLogger.log(`Disconnected "${firstRouter.name}" from "${clickedRouter.name}"`);
                    
                    // Skip disconnection animation for now to avoid visual issues
                    // TODO: Fix animation persistence issue
                    /*
                    animationEffects.animateConnectionChange(
                        stateManager.canvasRenderer.ctx,
                        firstRouter,
                        clickedRouter,
                        false
                    );
                    */
                    
                    // Update connections list
                    this.updateConnectionsFromSimulator();
                    
                    // Re-render
                    if (stateManager.canvasRenderer) {
                        stateManager.canvasRenderer.render();
                    }
                } else {
                    eventLogger.log(`No connection exists between "${firstRouter.name}" and "${clickedRouter.name}"`);
                }
            }
            
            // Reset selection
            uiController.clearSelectedRouters();
            uiController.setMode('disconnect-routers');
        }
    }

    handleToggleFailure(x, y) {
        // Check if clicking on a router
        const clickedRouter = this.findRouterAt(x, y);
        if (clickedRouter) {
            if (stateManager.simulator) {
                const success = stateManager.simulator.toggle_router_failure(clickedRouter.id);
                if (success) {
                    eventLogger.log(`Toggled failure state for router "${clickedRouter.name}"`);
                    
                    // Update routers list
                    this.updateRoutersFromSimulator();
                    uiController.updateRoutersList();
                    
                    // Re-render
                    if (stateManager.canvasRenderer) {
                        stateManager.canvasRenderer.render();
                    }
                }
            }
            return;
        }
        
        // Check if clicking on a connection
        const clickedConnection = this.findConnectionAt(x, y);
        if (clickedConnection) {
            if (stateManager.simulator) {
                const success = stateManager.simulator.toggle_link_failure(
                    clickedConnection.from_router_id, 
                    clickedConnection.to_router_id
                );
                if (success) {
                    const fromRouter = stateManager.routers.find(r => r.id === clickedConnection.from_router_id);
                    const toRouter = stateManager.routers.find(r => r.id === clickedConnection.to_router_id);
                    eventLogger.log(`Toggled failure state for link between "${fromRouter?.name}" and "${toRouter?.name}"`);
                    
                    // Update connections list
                    this.updateConnectionsFromSimulator();
                    
                    // Re-render
                    if (stateManager.canvasRenderer) {
                        stateManager.canvasRenderer.render();
                    }
                }
            }
        }
    }

    findRouterAt(x, y) {
        // Use the router icon's isPointInRouter method for accurate hit detection
        if (stateManager.canvasRenderer && stateManager.canvasRenderer.routerIcon) {
            return stateManager.routers.find(router => {
                return stateManager.canvasRenderer.routerIcon.isPointInRouter(x, y, router.x, router.y);
            });
        }
        
        // Fallback to simple radius check
        const ROUTER_RADIUS = 25;
        return stateManager.routers.find(router => {
            const dx = x - router.x;
            const dy = y - router.y;
            return Math.sqrt(dx * dx + dy * dy) <= ROUTER_RADIUS;
        });
    }

    findConnectionAt(x, y) {
        const CLICK_TOLERANCE = 10;
        
        return stateManager.connections.find(connection => {
            const fromRouter = stateManager.routers.find(r => r.id === connection.from_router_id);
            const toRouter = stateManager.routers.find(r => r.id === connection.to_router_id);
            
            if (!fromRouter || !toRouter) return false;
            
            // Calculate distance from point to line segment
            const dx = toRouter.x - fromRouter.x;
            const dy = toRouter.y - fromRouter.y;
            const length = Math.sqrt(dx * dx + dy * dy);
            
            if (length === 0) return false;
            
            const t = Math.max(0, Math.min(1, ((x - fromRouter.x) * dx + (y - fromRouter.y) * dy) / (length * length)));
            const closestX = fromRouter.x + t * dx;
            const closestY = fromRouter.y + t * dy;
            
            const distance = Math.sqrt((x - closestX) * (x - closestX) + (y - closestY) * (y - closestY));
            return distance <= CLICK_TOLERANCE;
        });
    }

    updateRoutersFromSimulator() {
        if (!stateManager.simulator) return;
        
        const routersJson = stateManager.simulator.get_routers_json();
        if (routersJson) {
            stateManager.routers = JSON.parse(routersJson);
        }
    }

    updateConnectionsFromSimulator() {
        if (!stateManager.simulator) return;
        
        const connectionsJson = stateManager.simulator.get_connections_json();
        if (connectionsJson) {
            stateManager.connections = JSON.parse(connectionsJson);
        }
    }
    
    updateTerminalsFromSimulator() {
        if (!stateManager.simulator) return;
        
        const terminalsJson = stateManager.simulator.get_all_terminals_json();
        if (terminalsJson) {
            stateManager.terminals = JSON.parse(terminalsJson);
        }
    }
    
    findTerminalAt(x, y) {
        if (!stateManager.terminals) return null;
        
        const TERMINAL_RADIUS = 20;
        return stateManager.terminals.find(terminal => {
            const dx = x - terminal.x;
            const dy = y - terminal.y;
            return Math.sqrt(dx * dx + dy * dy) <= TERMINAL_RADIUS;
        });
    }
    
    showRouterDetails(router) {
        if (!stateManager.simulator) return;
        
        const detailsJson = stateManager.simulator.get_router_details_json(router.id);
        if (!detailsJson) return;
        
        try {
            const details = JSON.parse(detailsJson);
            const modal = document.getElementById('router-details-modal');
            const title = document.getElementById('router-details-title');
            const content = document.getElementById('router-details-content');
            
            title.textContent = `Router ${details.name} (ID: ${details.id}) Details`;
            
            let html = '<h3>General Information</h3>';
            html += `<p>OSPF Status: ${details.ospf_enabled ? 'Enabled' : 'Disabled'}</p>`;
            if (details.ospf_enabled) {
                html += `<p>OSPF Neighbors: ${details.ospf_neighbors}</p>`;
                html += `<p>LSA Database Size: ${details.lsa_database_size}</p>`;
            }
            
            // Interfaces
            html += '<h3>Interfaces</h3>';
            if (details.interfaces && Object.keys(details.interfaces).length > 0) {
                html += '<table class="routing-table">';
                html += '<tr><th>ID</th><th>IP Address</th><th>Connected To</th><th>Cost</th><th>Status</th></tr>';
                for (const [id, iface] of Object.entries(details.interfaces)) {
                    html += `<tr>`;
                    html += `<td>${id}</td>`;
                    html += `<td>${iface.ip_address}/${iface.netmask}</td>`;
                    html += `<td>${iface.connected_router_id || 'N/A'}</td>`;
                    html += `<td>${iface.cost}</td>`;
                    html += `<td>${iface.enabled ? 'Up' : 'Down'}</td>`;
                    html += `</tr>`;
                }
                html += '</table>';
            } else {
                html += '<p>No interfaces configured</p>';
            }
            
            // Routing Table
            html += '<h3>Routing Table</h3>';
            if (details.routing_table && details.routing_table.length > 0) {
                html += '<table class="routing-table">';
                html += '<tr><th>Destination</th><th>Netmask</th><th>Next Hop</th><th>Interface</th><th>Metric</th><th>Protocol</th></tr>';
                for (const route of details.routing_table) {
                    html += `<tr>`;
                    html += `<td>${route.destination}</td>`;
                    html += `<td>${route.netmask}</td>`;
                    html += `<td>${route.next_hop}</td>`;
                    html += `<td>${route.interface_id}</td>`;
                    html += `<td>${route.metric}</td>`;
                    html += `<td>${route.protocol}</td>`;
                    html += `</tr>`;
                }
                html += '</table>';
            } else {
                html += '<p>No routes in routing table</p>';
            }
            
            content.innerHTML = html;
            modal.style.display = 'block';
            
            // Setup close handlers
            const closeBtn = modal.querySelector('.close');
            closeBtn.onclick = () => modal.style.display = 'none';
            
            window.onclick = (event) => {
                if (event.target === modal) {
                    modal.style.display = 'none';
                }
            };
            
        } catch (error) {
            console.error('Error showing router details:', error);
            eventLogger.log('Error displaying router details');
        }
    }
}

// Create singleton instance
const canvasInteraction = new CanvasInteraction();

export default canvasInteraction;