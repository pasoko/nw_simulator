/**
 * Canvas Renderer Module
 * Handles all canvas drawing operations
 */

import stateManager from './state-manager.js';
import { RouterIcon } from './router-icon.js';
import themeManager from './theme-manager.js';
import animationEffects from './animation-effects.js';
import hostManager from './host-manager.js';
import terminalManager from './terminal-manager.js';

class CanvasRenderer {
    constructor() {
        this.canvas = null;
        this.ctx = null;
        this.packetVisualizer = null;
        this.routerIcon = new RouterIcon();
    }
    
    init(canvas, ctx) {
        this.canvas = canvas;
        this.ctx = ctx;
        // Get packetVisualizer from stateManager since it's already initialized there
        this.packetVisualizer = stateManager.packetVisualizer;
        this.setupCanvas();
        
        // Store reference in stateManager for other modules
        stateManager.canvasRenderer = this;
    }
    
    setupCanvas() {
        const container = document.getElementById('canvas-container');
        this.canvas.width = container.clientWidth;
        this.canvas.height = container.clientHeight;
        
        window.addEventListener('resize', () => {
            this.canvas.width = container.clientWidth;
            this.canvas.height = container.clientHeight;
            this.render();
        });
        
        // Listen for sidebar resize events
        window.addEventListener('sidebarResized', () => {
            this.canvas.width = container.clientWidth;
            this.canvas.height = container.clientHeight;
            this.render();
        });
    }
    
    render() {
        if (!this.ctx) return;
        
        // Clear canvas with gradient background
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        
        // Draw gradient background based on theme
        const gradient = this.ctx.createLinearGradient(0, 0, this.canvas.width, this.canvas.height);
        if (themeManager.isDarkMode()) {
            gradient.addColorStop(0, '#1a1a1a');
            gradient.addColorStop(1, '#2d2d2d');
        } else {
            gradient.addColorStop(0, '#f5f7fa');
            gradient.addColorStop(1, '#c3cfe2');
        }
        this.ctx.fillStyle = gradient;
        this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
        
        // Update router summaries if not in simulation
        if (!stateManager.simulationRunning) {
            stateManager.routers.forEach(router => {
                const summaryJson = stateManager.simulator.get_router_summary_json(router.id);
                if (summaryJson) {
                    router.summary = JSON.parse(summaryJson);
                }
            });
        }
        
        // Draw connections
        this.drawConnections();
        
        // Draw packets
        if (this.packetVisualizer) {
            this.packetVisualizer.draw();
        }
        
        // Draw routers
        this.drawRouters();
        
        // Draw hosts
        hostManager.drawHosts(this.ctx);
        
        // Draw terminals
        console.log('Rendering terminals, count:', stateManager.terminals ? stateManager.terminals.length : 0);
        terminalManager.drawTerminals(this.ctx);
        
        // Draw packet statistics
        this.drawPacketStats();
    }
    
    drawConnections() {
        stateManager.connections.forEach(conn => {
            const from = stateManager.findRouterById(conn.from_router_id);
            const to = stateManager.findRouterById(conn.to_router_id);
            
            if (from && to) {
                this.drawConnection(from, to, conn);
            }
        });
        
        // Animation effects will be drawn separately
    }
    
    drawConnection(from, to, conn) {
        // Calculate direction vector
        const dx = to.x - from.x;
        const dy = to.y - from.y;
        const distance = Math.sqrt(dx * dx + dy * dy);
        const unitX = dx / distance;
        const unitY = dy / distance;
        
        // Adjust start and end points to not overlap with router icons (half size = 25)
        const routerEdgeDistance = 30; // Slightly larger than icon half-size for better spacing
        const startX = from.x + unitX * routerEdgeDistance;
        const startY = from.y + unitY * routerEdgeDistance;
        const endX = to.x - unitX * routerEdgeDistance;
        const endY = to.y - unitY * routerEdgeDistance;
        
        // Save current context state
        this.ctx.save();
        
        // Apply failure styling if connection is failed
        if (conn.is_failed) {
            this.ctx.strokeStyle = '#ff0000'; // Bright red for failed connection
            this.ctx.lineWidth = 4; // Thicker line for failed connection
            this.ctx.setLineDash([8, 4]); // Larger dash pattern
        } else {
            this.ctx.strokeStyle = themeManager.isDarkMode() ? '#999' : '#666';
            this.ctx.lineWidth = 2;
        }
        
        // Draw main line
        this.ctx.beginPath();
        this.ctx.moveTo(startX, startY);
        this.ctx.lineTo(endX, endY);
        this.ctx.stroke();
        
        // Restore context state
        this.ctx.restore();
        
        // Draw bidirectional arrows
        this.drawArrows(startX, startY, endX, endY, dx, dy);
        
        // Draw interface labels
        this.drawInterfaceLabels(from, to, conn, unitX, unitY);
        
        // Draw cost label
        this.drawCostLabel(from, to, conn);
        
        // Draw failure X mark if connection is failed
        if (conn.is_failed) {
            const midX = (from.x + to.x) / 2;
            const midY = (from.y + to.y) / 2;
            this.drawFailureMark(midX, midY, 15);
        }
    }
    
    drawArrows(startX, startY, endX, endY, dx, dy) {
        const arrowLength = 10;
        const arrowAngle = Math.PI / 6; // 30 degrees
        
        // Arrow at 'to' end
        this.ctx.beginPath();
        this.ctx.moveTo(endX, endY);
        this.ctx.lineTo(
            endX - arrowLength * Math.cos(Math.atan2(dy, dx) - arrowAngle),
            endY - arrowLength * Math.sin(Math.atan2(dy, dx) - arrowAngle)
        );
        this.ctx.moveTo(endX, endY);
        this.ctx.lineTo(
            endX - arrowLength * Math.cos(Math.atan2(dy, dx) + arrowAngle),
            endY - arrowLength * Math.sin(Math.atan2(dy, dx) + arrowAngle)
        );
        this.ctx.stroke();
        
        // Arrow at 'from' end
        this.ctx.beginPath();
        this.ctx.moveTo(startX, startY);
        this.ctx.lineTo(
            startX + arrowLength * Math.cos(Math.atan2(dy, dx) - arrowAngle),
            startY + arrowLength * Math.sin(Math.atan2(dy, dx) - arrowAngle)
        );
        this.ctx.moveTo(startX, startY);
        this.ctx.lineTo(
            startX + arrowLength * Math.cos(Math.atan2(dy, dx) + arrowAngle),
            startY + arrowLength * Math.sin(Math.atan2(dy, dx) + arrowAngle)
        );
        this.ctx.stroke();
    }
    
    drawInterfaceLabels(from, to, conn, unitX, unitY) {
        this.ctx.font = 'bold 20px Arial';
        this.ctx.fillStyle = themeManager.isDarkMode() ? '#E0E0E0' : '#000';
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        
        // Get actual interface names from routers
        const fromInterfaceName = this.getInterfaceName(from.id, conn.from_interface_id);
        const toInterfaceName = this.getInterfaceName(to.id, conn.to_interface_id);
        
        // Interface number at 'from' end (adjusted for larger router icon)
        const fromLabelX = from.x + unitX * 50;
        const fromLabelY = from.y + unitY * 50;
        this.drawInterfaceLabel(fromLabelX, fromLabelY, fromInterfaceName);
        
        // Interface number at 'to' end (adjusted for larger router icon)
        const toLabelX = to.x - unitX * 50;
        const toLabelY = to.y - unitY * 50;
        this.drawInterfaceLabel(toLabelX, toLabelY, toInterfaceName);
    }
    
    getInterfaceName(routerId, interfaceId) {
        // Try to get router details from the simulator
        if (stateManager.simulator) {
            try {
                const detailsJson = stateManager.simulator.get_router_details_json(routerId);
                if (detailsJson) {
                    const details = JSON.parse(detailsJson);
                    if (details.interfaces) {
                        const iface = details.interfaces.find(i => i.id === interfaceId);
                        if (iface && iface.name) {
                            return iface.name;
                        }
                    }
                }
            } catch (e) {
                console.error('Error getting interface name:', e);
            }
        }
        // Fallback to generic name
        return `if${interfaceId || '?'}`;
    }
    
    drawInterfaceLabel(x, y, text) {
        // Measure text width
        const metrics = this.ctx.measureText(text);
        const padding = 4;
        const boxWidth = metrics.width + padding * 2;
        const boxHeight = 24;
        
        if (themeManager.isDarkMode()) {
            // Dark mode styling
            // Draw dark background
            this.ctx.fillStyle = 'rgba(30, 30, 30, 0.9)';
            this.ctx.fillRect(x - boxWidth / 2, y - boxHeight / 2, boxWidth, boxHeight);
            
            // Draw border
            this.ctx.strokeStyle = '#666';
            this.ctx.lineWidth = 1;
            this.ctx.strokeRect(x - boxWidth / 2, y - boxHeight / 2, boxWidth, boxHeight);
            
            // Draw text in white
            this.ctx.fillStyle = '#ffffff';
        } else {
            // Light mode styling
            // Draw white background
            this.ctx.fillStyle = 'rgba(255, 255, 255, 0.9)';
            this.ctx.fillRect(x - boxWidth / 2, y - boxHeight / 2, boxWidth, boxHeight);
            
            // Draw border
            this.ctx.strokeStyle = '#333';
            this.ctx.lineWidth = 1;
            this.ctx.strokeRect(x - boxWidth / 2, y - boxHeight / 2, boxWidth, boxHeight);
            
            // Draw text in black
            this.ctx.fillStyle = '#000';
        }
        
        this.ctx.fillText(text, x, y);
        
        // Reset line width
        this.ctx.lineWidth = 2;
    }
    
    drawCostLabel(from, to, conn) {
        const midX = (from.x + to.x) / 2;
        const midY = (from.y + to.y) / 2;
        this.ctx.font = 'bold 12px Arial';
        
        // Use white text in dark mode, dark text in light mode
        if (themeManager.isDarkMode()) {
            // Add background for better visibility in dark mode
            const text = `Cost: ${conn.cost}`;
            const metrics = this.ctx.measureText(text);
            const padding = 4;
            const boxWidth = metrics.width + padding * 2;
            const boxHeight = 18;
            
            // Semi-transparent background
            this.ctx.fillStyle = 'rgba(0, 0, 0, 0.7)';
            this.ctx.fillRect(midX - boxWidth / 2, midY - boxHeight / 2, boxWidth, boxHeight);
            
            // White text
            this.ctx.fillStyle = '#ffffff';
        } else {
            // Light mode - dark text with semi-transparent white background
            const text = `Cost: ${conn.cost}`;
            const metrics = this.ctx.measureText(text);
            const padding = 4;
            const boxWidth = metrics.width + padding * 2;
            const boxHeight = 18;
            
            // Semi-transparent background
            this.ctx.fillStyle = 'rgba(255, 255, 255, 0.8)';
            this.ctx.fillRect(midX - boxWidth / 2, midY - boxHeight / 2, boxWidth, boxHeight);
            
            // Dark text
            this.ctx.fillStyle = '#000000';
        }
        
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText(`Cost: ${conn.cost}`, midX, midY);
        
        // Reset text alignment
        this.ctx.textAlign = 'left';
        this.ctx.textBaseline = 'alphabetic';
    }
    
    drawFailureMark(x, y, size) {
        // Draw red X mark to indicate failure
        this.ctx.save();
        this.ctx.strokeStyle = '#ff0000';
        this.ctx.lineWidth = 3;
        
        // Draw X
        this.ctx.beginPath();
        this.ctx.moveTo(x - size/2, y - size/2);
        this.ctx.lineTo(x + size/2, y + size/2);
        this.ctx.moveTo(x + size/2, y - size/2);
        this.ctx.lineTo(x - size/2, y + size/2);
        this.ctx.stroke();
        
        // Draw circle around X
        this.ctx.beginPath();
        this.ctx.arc(x, y, size, 0, 2 * Math.PI);
        this.ctx.stroke();
        
        this.ctx.restore();
    }
    
    drawRouters() {
        stateManager.routers.forEach(router => {
            this.drawRouter(router);
        });
        
        // Draw all animation effects
        animationEffects.drawAnimations(this.ctx);
    }
    
    drawRouter(router) {
        const isSelected = stateManager.isRouterSelected(router.id);
        const isDragging = stateManager.draggingRouter && stateManager.draggingRouter.id === router.id;
        const mode = stateManager.getMode();
        
        // Prepare router state
        const routerState = {
            normal: !isSelected && !isDragging && !router.is_failed,
            ospfEnabled: router.ospf_enabled,
            failed: router.is_failed,
            selected: isSelected,
            dragging: isDragging
        };
        
        // Draw selection ring for various modes
        if (isSelected && (mode === 'connect-routers' || mode === 'disconnect-routers')) {
            this.ctx.save();
            this.ctx.beginPath();
            this.ctx.arc(router.x, router.y, 35, 0, 2 * Math.PI);
            if (mode === 'connect-routers') {
                this.ctx.strokeStyle = stateManager.selectedRouters.indexOf(router.id) === 0 ? '#ff9800' : '#4caf50';
            } else {
                this.ctx.strokeStyle = stateManager.selectedRouters.indexOf(router.id) === 0 ? '#dc3545' : '#f44336';
            }
            this.ctx.lineWidth = 4;
            this.ctx.stroke();
            this.ctx.restore();
        }
        
        // Highlight router on hover in delete mode
        if (mode === 'delete-router') {
            if (this.routerIcon.isPointInRouter(stateManager.lastMouseX, stateManager.lastMouseY, router.x, router.y)) {
                this.ctx.save();
                this.ctx.beginPath();
                this.ctx.arc(router.x, router.y, 35, 0, 2 * Math.PI);
                this.ctx.strokeStyle = '#dc3545';
                this.ctx.lineWidth = 3;
                this.ctx.setLineDash([5, 5]);
                this.ctx.stroke();
                this.ctx.restore();
            }
        }
        
        // Draw router icon using the new RouterIcon class
        this.routerIcon.draw(this.ctx, router.x, router.y, router.id, routerState);
        
        // Draw router details
        this.drawRouterDetails(router);
    }
    
    drawRouterDetails(router) {
        if (!router.summary) return;
        
        this.ctx.fillStyle = themeManager.isDarkMode() ? '#E0E0E0' : '#000';
        this.ctx.font = '10px Arial';
        this.ctx.textAlign = 'center';
        
        let yOffset = 35;
        
        // OSPF status
        if (router.summary.ospf_enabled) {
            this.ctx.fillText(`Neighbors: ${router.summary.neighbor_count}`, router.x, router.y + yOffset);
            yOffset += 12;
            
            this.ctx.fillText(`Routes: ${router.summary.route_count}`, router.x, router.y + yOffset);
            yOffset += 12;
            
            // Latest event (truncate if too long)
            const event = router.summary.latest_event;
            const maxLength = 30;
            const displayEvent = event.length > maxLength ? 
                event.substring(0, maxLength) + '...' : event;
            this.ctx.font = '9px Arial';
            this.ctx.fillStyle = themeManager.isDarkMode() ? '#A0A0A0' : '#666';
            this.ctx.fillText(displayEvent, router.x, router.y + yOffset);
        } else {
            this.ctx.fillStyle = themeManager.isDarkMode() ? '#666' : '#999';
            this.ctx.fillText('OSPF Disabled', router.x, router.y + yOffset);
        }
    }
    
    drawPacketStats() {
        if (!this.packetVisualizer || !stateManager.simulationRunning) return;
        
        const stats = this.packetVisualizer.getPacketsByType();
        const activeCount = this.packetVisualizer.getActivePacketCount();
        
        // Set text color based on theme
        this.ctx.fillStyle = themeManager.isDarkMode() ? '#ffffff' : '#000000';
        this.ctx.font = 'bold 12px Arial';
        this.ctx.textAlign = 'left';
        
        // Draw background for better visibility
        const bgColor = themeManager.isDarkMode() ? 'rgba(0, 0, 0, 0.7)' : 'rgba(255, 255, 255, 0.8)';
        this.ctx.fillStyle = bgColor;
        this.ctx.fillRect(5, 30, 150, 25);
        
        // Draw active packets count
        this.ctx.fillStyle = themeManager.isDarkMode() ? '#ffffff' : '#000000';
        this.ctx.fillText(`Active Packets: ${activeCount}`, 10, 50);
        
        // Draw background for packet type stats
        const statsCount = Object.keys(stats).length;
        if (statsCount > 0) {
            const bgColor = themeManager.isDarkMode() ? 'rgba(0, 0, 0, 0.7)' : 'rgba(255, 255, 255, 0.8)';
            this.ctx.fillStyle = bgColor;
            this.ctx.fillRect(5, 60, 150, statsCount * 20 + 10);
        }
        
        let y = 70;
        Object.entries(stats).forEach(([type, count]) => {
            // Handle both packetColors and packetConfigs for compatibility
            let color = '#666';
            if (this.packetVisualizer.packetColors && this.packetVisualizer.packetColors[type]) {
                color = this.packetVisualizer.packetColors[type];
            } else if (this.packetVisualizer.packetConfigs && this.packetVisualizer.packetConfigs[type]) {
                color = this.packetVisualizer.packetConfigs[type].color || '#666';
            }
            
            // Use theme-aware colors
            if (themeManager.isDarkMode() && color === '#666') {
                color = '#999';
            }
            
            this.ctx.fillStyle = color;
            this.ctx.fillText(`${type}: ${count}`, 10, y);
            y += 20;
        });
    }
    
    updateCursor(cursor) {
        if (this.canvas) {
            this.canvas.style.cursor = cursor;
        }
    }
    
    updateColors() {
        // Re-render when theme changes
        this.render();
    }
    
}

// Export as singleton
export default new CanvasRenderer();