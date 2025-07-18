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
        this.dragOffset = { x: 0, y: 0 };
        this.hoveredRouter = null;
    }

    init(canvas) {
        this.setupCanvasEventListeners(canvas);
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
        
        // Check if clicking on a router for dragging (only in move-router mode)
        const clickedRouter = this.findRouterAt(x, y);
        if (clickedRouter && uiController.getMode() === 'move-router') {
            this.draggingRouter = clickedRouter;
            this.dragOffset.x = x - clickedRouter.x;
            this.dragOffset.y = y - clickedRouter.y;
            stateManager.canvas.style.cursor = 'grabbing';
            event.preventDefault(); // Prevent text selection
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
        if (router) {
            stateManager.canvas.style.cursor = mode === 'move-router' ? 'grab' : 'pointer';
        } else {
            stateManager.canvas.style.cursor = mode === 'add-router' ? 'crosshair' : 'default';
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
    }

    handleMouseUp(event) {
        if (this.draggingRouter) {
            this.draggingRouter = null;
            stateManager.canvas.style.cursor = 'grab';
        }
    }

    handleClick(event) {
        // Don't handle click if we were dragging
        if (this.draggingRouter) {
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

    handleConnectRouters(x, y) {
        const clickedRouter = this.findRouterAt(x, y);
        if (!clickedRouter) return;
        
        const selectedRouters = uiController.getSelectedRouters();
        
        if (selectedRouters.length === 0) {
            // Select first router
            uiController.selectRouter(clickedRouter);
            eventLogger.log(`Selected router "${clickedRouter.name}" as first router`);
            
            // Add selection animation
            animationEffects.animateRouterSelection(stateManager.canvasRenderer.ctx, clickedRouter);
            
            // Update mode indicator - removed as it no longer exists in modern UI
        } else if (selectedRouters.length === 1) {
            // Connect to second router
            const firstRouter = selectedRouters[0];
            
            if (firstRouter.id === clickedRouter.id) {
                eventLogger.log('Cannot connect router to itself');
                return;
            }
            
            // Check if connection already exists
            const existingConnection = stateManager.connections.find(conn => 
                (conn.from_router_id === firstRouter.id && conn.to_router_id === clickedRouter.id) ||
                (conn.from_router_id === clickedRouter.id && conn.to_router_id === firstRouter.id)
            );
            
            if (existingConnection) {
                eventLogger.log('Connection already exists between these routers');
                uiController.clearSelectedRouters();
                uiController.setMode('connect-routers');
                return;
            }
            
            const cost = parseInt(prompt('Enter link cost (1-100):', '10'));
            if (isNaN(cost) || cost < 1 || cost > 100) {
                eventLogger.log('Invalid cost. Please enter a number between 1 and 100.');
                return;
            }
            
            if (stateManager.simulator) {
                stateManager.simulator.connect_routers(firstRouter.id, clickedRouter.id, cost);
                eventLogger.log(`Connected "${firstRouter.name}" to "${clickedRouter.name}" with cost ${cost}`);
                
                // Skip connection animation for now to avoid visual issues
                // TODO: Fix animation persistence issue
                /*
                animationEffects.animateConnectionChange(
                    stateManager.canvasRenderer.ctx,
                    firstRouter,
                    clickedRouter,
                    true,
                    300  // Shorter duration to ensure it completes quickly
                );
                */
                
                // Update connections list
                this.updateConnectionsFromSimulator();
                
                // Re-render
                if (stateManager.canvasRenderer) {
                    stateManager.canvasRenderer.render();
                }
            }
            
            // Reset selection
            uiController.clearSelectedRouters();
            uiController.setMode('connect-routers');
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