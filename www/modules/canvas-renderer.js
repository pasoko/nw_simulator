/**
 * Canvas Renderer Module
 * Handles all canvas drawing operations
 */

import stateManager from './state-manager.js';

class CanvasRenderer {
    constructor() {
        this.canvas = null;
        this.ctx = null;
        this.packetVisualizer = null;
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
    }
    
    render() {
        if (!this.ctx) return;
        
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        
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
    }
    
    drawConnection(from, to, conn) {
        // Calculate direction vector
        const dx = to.x - from.x;
        const dy = to.y - from.y;
        const distance = Math.sqrt(dx * dx + dy * dy);
        const unitX = dx / distance;
        const unitY = dy / distance;
        
        // Adjust start and end points to not overlap with router circles (radius 20)
        const startX = from.x + unitX * 20;
        const startY = from.y + unitY * 20;
        const endX = to.x - unitX * 20;
        const endY = to.y - unitY * 20;
        
        // Save current context state
        this.ctx.save();
        
        // Apply failure styling if connection is failed
        if (conn.is_failed) {
            this.ctx.strokeStyle = '#ff0000'; // Bright red for failed connection
            this.ctx.lineWidth = 4; // Thicker line for failed connection
            this.ctx.setLineDash([8, 4]); // Larger dash pattern
        } else {
            this.ctx.strokeStyle = '#666';
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
            this.drawFailureX(midX, midY, 15);
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
        this.ctx.fillStyle = '#000';
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        
        // Interface number at 'from' end
        const fromLabelX = from.x + unitX * 40;
        const fromLabelY = from.y + unitY * 40;
        this.drawInterfaceLabel(fromLabelX, fromLabelY, `if${conn.from_interface_id || '?'}`);
        
        // Interface number at 'to' end
        const toLabelX = to.x - unitX * 40;
        const toLabelY = to.y - unitY * 40;
        this.drawInterfaceLabel(toLabelX, toLabelY, `if${conn.to_interface_id || '?'}`);
    }
    
    drawInterfaceLabel(x, y, text) {
        // Measure text width
        const metrics = this.ctx.measureText(text);
        const padding = 4;
        const boxWidth = metrics.width + padding * 2;
        const boxHeight = 24;
        
        // Draw white background
        this.ctx.fillStyle = 'rgba(255, 255, 255, 0.9)';
        this.ctx.fillRect(x - boxWidth / 2, y - boxHeight / 2, boxWidth, boxHeight);
        
        // Draw border
        this.ctx.strokeStyle = '#333';
        this.ctx.lineWidth = 1;
        this.ctx.strokeRect(x - boxWidth / 2, y - boxHeight / 2, boxWidth, boxHeight);
        
        // Draw text
        this.ctx.fillStyle = '#000';
        this.ctx.fillText(text, x, y);
        
        // Reset line width
        this.ctx.lineWidth = 2;
    }
    
    drawCostLabel(from, to, conn) {
        const midX = (from.x + to.x) / 2;
        const midY = (from.y + to.y) / 2;
        this.ctx.font = '12px Arial';
        this.ctx.fillStyle = '#000';
        this.ctx.fillText(`Cost: ${conn.cost}`, midX, midY);
    }
    
    drawRouters() {
        stateManager.routers.forEach(router => {
            this.drawRouter(router);
        });
    }
    
    drawRouter(router) {
        const isSelected = stateManager.isRouterSelected(router.id);
        const isDragging = stateManager.draggingRouter && stateManager.draggingRouter.id === router.id;
        const mode = stateManager.getMode();
        
        // Draw dragging highlight
        if (isDragging) {
            this.ctx.beginPath();
            this.ctx.arc(router.x, router.y, 25, 0, 2 * Math.PI);
            this.ctx.strokeStyle = '#2196F3';
            this.ctx.lineWidth = 3;
            this.ctx.stroke();
        }
        
        // Draw selection ring for various modes
        if (isSelected && (mode === 'connect-routers' || mode === 'disconnect-routers')) {
            this.ctx.beginPath();
            this.ctx.arc(router.x, router.y, 25, 0, 2 * Math.PI);
            if (mode === 'connect-routers') {
                this.ctx.strokeStyle = stateManager.selectedRouters.indexOf(router.id) === 0 ? '#ff9800' : '#4caf50';
            } else {
                this.ctx.strokeStyle = stateManager.selectedRouters.indexOf(router.id) === 0 ? '#dc3545' : '#f44336';
            }
            this.ctx.lineWidth = 4;
            this.ctx.stroke();
        }
        
        // Highlight router on hover in delete mode
        if (mode === 'delete-router') {
            const dx = router.x - stateManager.lastMouseX;
            const dy = router.y - stateManager.lastMouseY;
            if (dx * dx + dy * dy < 400) { // 20px radius
                this.ctx.beginPath();
                this.ctx.arc(router.x, router.y, 25, 0, 2 * Math.PI);
                this.ctx.strokeStyle = '#dc3545';
                this.ctx.lineWidth = 3;
                this.ctx.setLineDash([5, 5]);
                this.ctx.stroke();
                this.ctx.setLineDash([]);
            }
        }
        
        // Draw router circle
        this.ctx.beginPath();
        this.ctx.arc(router.x, router.y, 20, 0, 2 * Math.PI);
        
        // Set fill color based on failure state and OSPF status
        if (router.is_failed) {
            this.ctx.fillStyle = '#ff0000'; // Bright red for failed router
        } else {
            this.ctx.fillStyle = router.ospf_enabled ? '#4CAF50' : '#2196F3';
        }
        this.ctx.fill();
        
        // Draw failure indicator border
        if (router.is_failed) {
            this.ctx.strokeStyle = '#8b0000'; // Dark red border
            this.ctx.lineWidth = 3;
            this.ctx.stroke();
        }
        
        if (isSelected && mode === 'connect-routers') {
            this.ctx.strokeStyle = '#000';
            this.ctx.lineWidth = 2;
            this.ctx.stroke();
        }
        
        // Draw router name
        this.ctx.fillStyle = '#fff';
        this.ctx.font = 'bold 12px Arial';
        this.ctx.textAlign = 'center';
        this.ctx.textBaseline = 'middle';
        this.ctx.fillText(router.name, router.x, router.y);
        
        // Draw failure X mark if router is failed
        if (router.is_failed) {
            this.drawFailureX(router.x, router.y, 25);
        }
        
        // Draw router details
        this.drawRouterDetails(router);
    }
    
    drawRouterDetails(router) {
        if (!router.summary) return;
        
        this.ctx.fillStyle = '#000';
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
            this.ctx.fillStyle = '#666';
            this.ctx.fillText(displayEvent, router.x, router.y + yOffset);
        } else {
            this.ctx.fillStyle = '#999';
            this.ctx.fillText('OSPF Disabled', router.x, router.y + yOffset);
        }
    }
    
    drawPacketStats() {
        if (!this.packetVisualizer || !stateManager.simulationRunning) return;
        
        const stats = this.packetVisualizer.getPacketsByType();
        const activeCount = this.packetVisualizer.getActivePacketCount();
        
        this.ctx.fillStyle = '#000';
        this.ctx.font = '12px Arial';
        this.ctx.textAlign = 'left';
        this.ctx.fillText(`Active Packets: ${activeCount}`, 10, 50);
        
        let y = 70;
        Object.entries(stats).forEach(([type, count]) => {
            this.ctx.fillStyle = this.packetVisualizer.packetColors[type] || '#666';
            this.ctx.fillText(`${type}: ${count}`, 10, y);
            y += 20;
        });
    }
    
    updateCursor(cursor) {
        if (this.canvas) {
            this.canvas.style.cursor = cursor;
        }
    }
    
    drawFailureX(x, y, size) {
        this.ctx.save();
        this.ctx.strokeStyle = '#ffffff'; // White color for better contrast
        this.ctx.lineWidth = 5;
        this.ctx.lineCap = 'round';
        
        // Draw white background X first
        const offset = size / 2.5;
        this.ctx.beginPath();
        this.ctx.moveTo(x - offset, y - offset);
        this.ctx.lineTo(x + offset, y + offset);
        this.ctx.stroke();
        
        this.ctx.beginPath();
        this.ctx.moveTo(x - offset, y + offset);
        this.ctx.lineTo(x + offset, y - offset);
        this.ctx.stroke();
        
        // Draw red X on top
        this.ctx.strokeStyle = '#ff0000';
        this.ctx.lineWidth = 3;
        
        this.ctx.beginPath();
        this.ctx.moveTo(x - offset, y - offset);
        this.ctx.lineTo(x + offset, y + offset);
        this.ctx.stroke();
        
        this.ctx.beginPath();
        this.ctx.moveTo(x - offset, y + offset);
        this.ctx.lineTo(x + offset, y - offset);
        this.ctx.stroke();
        
        this.ctx.restore();
    }
}

// Export as singleton
export default new CanvasRenderer();