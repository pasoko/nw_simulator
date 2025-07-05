/**
 * Router Icon Module
 * Provides SVG-based router icon rendering
 */

import themeManager from './theme-manager.js';

export class RouterIcon {
    constructor() {
        // Define router icon paths for different states
        this.iconSize = 50; // Size of the router icon
        this.updateColors();
    }
    
    updateColors() {
        const isDark = themeManager.isDarkMode();
        this.colors = {
            normal: {
                fill: isDark ? '#1E3A5F' : '#E3F2FD',
                stroke: isDark ? '#42A5F5' : '#1976D2',
                text: isDark ? '#90CAF9' : '#0D47A1'
            },
            ospfEnabled: {
                fill: isDark ? '#1B5E20' : '#E8F5E9',
                stroke: isDark ? '#66BB6A' : '#4CAF50',
                text: isDark ? '#A5D6A7' : '#1B5E20'
            },
            failed: {
                fill: isDark ? '#5D1F1F' : '#FFEBEE',
                stroke: isDark ? '#EF5350' : '#F44336',
                text: isDark ? '#FFCDD2' : '#B71C1C'
            },
            selected: {
                fill: isDark ? '#5D4037' : '#FFF3E0',
                stroke: isDark ? '#FFB74D' : '#FF9800',
                text: isDark ? '#FFCC80' : '#E65100'
            },
            dragging: {
                fill: isDark ? '#01579B' : '#E1F5FE',
                stroke: isDark ? '#29B6F6' : '#03A9F4',
                text: isDark ? '#81D4FA' : '#01579B'
            }
        };
    }

    /**
     * Draw router icon on canvas
     * @param {CanvasRenderingContext2D} ctx - Canvas context
     * @param {number} x - X coordinate
     * @param {number} y - Y coordinate
     * @param {string} routerId - Router ID to display
     * @param {Object} state - Router state (normal, ospfEnabled, failed, selected, dragging)
     */
    draw(ctx, x, y, routerId, state = {}) {
        // Update colors if theme has changed
        this.updateColors();
        const colors = this.getColors(state);
        const size = this.iconSize;
        const halfSize = size / 2;

        ctx.save();
        
        // Shadow for depth
        ctx.shadowColor = 'rgba(0, 0, 0, 0.2)';
        ctx.shadowBlur = 8;
        ctx.shadowOffsetX = 2;
        ctx.shadowOffsetY = 2;

        // Main router body (rounded rectangle)
        this.drawRouterBody(ctx, x - halfSize, y - halfSize, size, size, colors);
        
        // Reset shadow for internal elements
        ctx.shadowColor = 'transparent';
        
        // Draw network arrows
        this.drawNetworkArrows(ctx, x, y, halfSize * 0.7, colors);
        
        // Draw router ID
        this.drawRouterId(ctx, x, y, routerId, colors);
        
        // Draw status indicators
        if (state.ospfEnabled) {
            this.drawOSPFIndicator(ctx, x + halfSize - 8, y - halfSize + 8);
        }
        
        if (state.failed) {
            this.drawFailureIndicator(ctx, x + halfSize - 8, y + halfSize - 8);
        }

        ctx.restore();
    }

    /**
     * Get colors based on router state
     */
    getColors(state) {
        if (state.failed) return this.colors.failed;
        if (state.selected) return this.colors.selected;
        if (state.dragging) return this.colors.dragging;
        if (state.ospfEnabled) return this.colors.ospfEnabled;
        return this.colors.normal;
    }

    /**
     * Draw router body (rounded rectangle)
     */
    drawRouterBody(ctx, x, y, width, height, colors) {
        const radius = 8;
        
        ctx.beginPath();
        ctx.moveTo(x + radius, y);
        ctx.lineTo(x + width - radius, y);
        ctx.quadraticCurveTo(x + width, y, x + width, y + radius);
        ctx.lineTo(x + width, y + height - radius);
        ctx.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
        ctx.lineTo(x + radius, y + height);
        ctx.quadraticCurveTo(x, y + height, x, y + height - radius);
        ctx.lineTo(x, y + radius);
        ctx.quadraticCurveTo(x, y, x + radius, y);
        ctx.closePath();
        
        // Fill
        ctx.fillStyle = colors.fill;
        ctx.fill();
        
        // Stroke
        ctx.strokeStyle = colors.stroke;
        ctx.lineWidth = 2;
        ctx.stroke();
    }

    /**
     * Draw network arrows (crossing arrows pattern)
     */
    drawNetworkArrows(ctx, centerX, centerY, size, colors) {
        ctx.strokeStyle = colors.stroke;
        ctx.lineWidth = 2;
        ctx.lineCap = 'round';
        
        // Draw four arrows pointing outward
        const angles = [0, Math.PI/2, Math.PI, Math.PI * 1.5];
        const arrowLength = size * 0.6;
        const arrowHeadSize = 6;
        
        angles.forEach(angle => {
            const startX = centerX + Math.cos(angle) * (size * 0.3);
            const startY = centerY + Math.sin(angle) * (size * 0.3);
            const endX = centerX + Math.cos(angle) * arrowLength;
            const endY = centerY + Math.sin(angle) * arrowLength;
            
            // Arrow line
            ctx.beginPath();
            ctx.moveTo(startX, startY);
            ctx.lineTo(endX, endY);
            ctx.stroke();
            
            // Arrow head
            ctx.beginPath();
            ctx.moveTo(endX, endY);
            ctx.lineTo(
                endX - Math.cos(angle - Math.PI/6) * arrowHeadSize,
                endY - Math.sin(angle - Math.PI/6) * arrowHeadSize
            );
            ctx.moveTo(endX, endY);
            ctx.lineTo(
                endX - Math.cos(angle + Math.PI/6) * arrowHeadSize,
                endY - Math.sin(angle + Math.PI/6) * arrowHeadSize
            );
            ctx.stroke();
        });
    }

    /**
     * Draw router ID text
     */
    drawRouterId(ctx, x, y, routerId, colors) {
        ctx.font = 'bold 14px Arial';
        ctx.fillStyle = colors.text;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        
        // Extract just the number from router ID
        const routerNumber = routerId.toString().replace(/\D/g, '');
        ctx.fillText(routerNumber || routerId, x, y);
    }

    /**
     * Draw OSPF enabled indicator
     */
    drawOSPFIndicator(ctx, x, y) {
        const radius = 6;
        
        ctx.beginPath();
        ctx.arc(x, y, radius, 0, Math.PI * 2);
        ctx.fillStyle = '#4CAF50';
        ctx.fill();
        ctx.strokeStyle = '#2E7D32';
        ctx.lineWidth = 1;
        ctx.stroke();
        
        // Draw "O" for OSPF
        ctx.font = 'bold 8px Arial';
        ctx.fillStyle = 'white';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('O', x, y);
    }

    /**
     * Draw failure indicator
     */
    drawFailureIndicator(ctx, x, y) {
        const size = 12;
        
        ctx.beginPath();
        ctx.moveTo(x, y - size/2);
        ctx.lineTo(x + size/2, y + size/2);
        ctx.lineTo(x - size/2, y + size/2);
        ctx.closePath();
        
        ctx.fillStyle = '#F44336';
        ctx.fill();
        ctx.strokeStyle = '#D32F2F';
        ctx.lineWidth = 1;
        ctx.stroke();
        
        // Draw exclamation mark
        ctx.font = 'bold 8px Arial';
        ctx.fillStyle = 'white';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('!', x, y);
    }

    /**
     * Get hover state for router
     */
    isPointInRouter(x, y, routerX, routerY) {
        const halfSize = this.iconSize / 2;
        return x >= routerX - halfSize && x <= routerX + halfSize &&
               y >= routerY - halfSize && y <= routerY + halfSize;
    }
}