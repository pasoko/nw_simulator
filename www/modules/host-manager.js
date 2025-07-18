/**
 * Host Manager Module
 * Handles host device management, rendering, and ping functionality
 */

import stateManager from './state-manager.js';
import canvasRenderer from './canvas-renderer.js';

class HostManager {
    constructor() {
        this.hosts = new Map();
        this.selectedHost = null;
        this.init();
    }

    init() {
        // Listen for host updates
        window.addEventListener('hostsUpdated', () => this.updateHosts());
        
        // Listen for canvas clicks to select hosts
        window.addEventListener('canvasClick', (e) => this.handleCanvasClick(e.detail));
    }

    updateHosts() {
        if (!stateManager.simulator) return;
        
        const hostsJson = stateManager.simulator.get_hosts_json();
        if (hostsJson) {
            const hostsArray = JSON.parse(hostsJson);
            this.hosts.clear();
            hostsArray.forEach(host => {
                this.hosts.set(host.id, host);
            });
        }
    }

    drawHosts(ctx) {
        this.hosts.forEach(host => {
            this.drawHost(ctx, host);
        });
    }

    drawHost(ctx, host) {
        const x = host.x;
        const y = host.y;
        
        // Draw host icon (computer/laptop style)
        ctx.save();
        
        // Shadow for depth
        ctx.shadowColor = 'rgba(0, 0, 0, 0.2)';
        ctx.shadowBlur = 5;
        ctx.shadowOffsetX = 2;
        ctx.shadowOffsetY = 2;
        
        // Host body (monitor)
        ctx.fillStyle = host.is_failed ? '#ffcccc' : '#e0e0e0';
        ctx.strokeStyle = host.is_failed ? '#cc0000' : '#666666';
        ctx.lineWidth = 2;
        
        // Monitor
        ctx.beginPath();
        ctx.roundRect(x - 20, y - 15, 40, 25, 3);
        ctx.fill();
        ctx.stroke();
        
        // Screen
        ctx.fillStyle = host.is_failed ? '#ff6666' : '#4a90e2';
        ctx.fillRect(x - 17, y - 12, 34, 19);
        
        // Base
        ctx.fillStyle = host.is_failed ? '#ffcccc' : '#e0e0e0';
        ctx.fillRect(x - 8, y + 10, 16, 3);
        ctx.fillRect(x - 12, y + 13, 24, 3);
        
        ctx.restore();
        
        // Draw host name
        ctx.fillStyle = '#333';
        ctx.font = '12px Arial';
        ctx.textAlign = 'center';
        ctx.fillText(host.name, x, y + 30);
        
        // Draw IP address
        ctx.font = '10px Arial';
        ctx.fillStyle = '#666';
        ctx.fillText(host.ip_address, x, y + 42);
        
        // Selection indicator
        if (this.selectedHost === host.id) {
            ctx.strokeStyle = '#2196F3';
            ctx.lineWidth = 2;
            ctx.setLineDash([5, 5]);
            ctx.beginPath();
            ctx.arc(x, y, 35, 0, Math.PI * 2);
            ctx.stroke();
            ctx.setLineDash([]);
        }
    }

    handleCanvasClick(detail) {
        const { x, y } = detail;
        
        // Check if click is on a host
        this.hosts.forEach(host => {
            const distance = Math.sqrt(
                Math.pow(x - host.x, 2) + Math.pow(y - host.y, 2)
            );
            
            if (distance < 30) {
                this.selectHost(host.id);
            }
        });
    }

    selectHost(hostId) {
        this.selectedHost = hostId;
        this.showHostDetails(hostId);
        canvasRenderer.render();
    }

    showHostDetails(hostId) {
        if (!stateManager.simulator) return;
        
        const detailsJson = stateManager.simulator.get_host_details_json(hostId);
        if (!detailsJson) return;
        
        const details = JSON.parse(detailsJson);
        this.showHostDetailsDialog(details);
    }

    showHostDetailsDialog(details) {
        // Remove existing dialog if any
        const existingDialog = document.getElementById('host-details-dialog');
        if (existingDialog) {
            existingDialog.remove();
        }
        
        const dialog = document.createElement('div');
        dialog.id = 'host-details-dialog';
        dialog.className = 'host-details-dialog';
        dialog.innerHTML = `
            <div class="dialog-content">
                <div class="dialog-header">
                    <h3>${details.name} Details</h3>
                    <button class="close-btn" onclick="window.hostManager.closeHostDetailsDialog()">×</button>
                </div>
                <div class="dialog-body">
                    <div class="detail-row">
                        <span class="detail-label">IP Address:</span>
                        <span class="detail-value">${details.ip_address}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">Netmask:</span>
                        <span class="detail-value">${details.netmask}</span>
                    </div>
                    <div class="detail-row">
                        <span class="detail-label">Default Gateway:</span>
                        <span class="detail-value">${details.default_gateway}</span>
                    </div>
                    ${details.connected_router_id ? `
                    <div class="detail-row">
                        <span class="detail-label">Connected Router:</span>
                        <span class="detail-value">Router ${details.connected_router_id}</span>
                    </div>
                    ` : ''}
                    
                    <h4>ARP Table</h4>
                    <div class="arp-table">
                        ${details.arp_table.length > 0 ? details.arp_table.map(entry => `
                            <div class="arp-entry">
                                <span>${entry.ip_address}</span>
                                <span>${entry.mac_address}</span>
                            </div>
                        `).join('') : '<p class="no-data">No ARP entries</p>'}
                    </div>
                    
                    <h4>Ping Command</h4>
                    <div class="ping-section">
                        <input type="text" id="ping-destination" placeholder="Enter destination IP" class="ping-input">
                        <button onclick="window.hostManager.executePing(${details.id})" class="ping-btn">Ping</button>
                    </div>
                    
                    <div id="ping-results" class="ping-results"></div>
                </div>
            </div>
        `;
        
        document.body.appendChild(dialog);
    }

    closeHostDetailsDialog() {
        const dialog = document.getElementById('host-details-dialog');
        if (dialog) {
            dialog.remove();
        }
    }

    showAddHostDialog() {
        const dialog = document.createElement('div');
        dialog.className = 'dialog-overlay';
        dialog.innerHTML = `
            <div class="dialog-overlay" onclick="window.hostManager.closeAddHostDialog()"></div>
            <div class="dialog">
                <h3>Add Host Device</h3>
                <form id="add-host-form">
                    <div class="form-group">
                        <label>Host Name:</label>
                        <input type="text" id="host-name" required>
                    </div>
                    <div class="form-group">
                        <label>IP Address:</label>
                        <input type="text" id="host-ip" pattern="^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$" required>
                    </div>
                    <div class="form-group">
                        <label>Netmask:</label>
                        <input type="text" id="host-netmask" value="255.255.255.0" pattern="^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$" required>
                    </div>
                    <div class="form-group">
                        <label>Default Gateway:</label>
                        <input type="text" id="host-gateway" pattern="^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$" required>
                    </div>
                    <div class="form-actions">
                        <button type="submit">Add Host</button>
                        <button type="button" onclick="window.hostManager.closeAddHostDialog()">キャンセル</button>
                    </div>
                </form>
            </div>
        `;
        
        document.body.appendChild(dialog);
        
        // Handle form submission
        const form = document.getElementById('add-host-form');
        form.addEventListener('submit', (e) => {
            e.preventDefault();
            this.handleAddHost();
        });
    }

    closeAddHostDialog() {
        const overlays = document.querySelectorAll('.dialog-overlay');
        overlays.forEach(overlay => overlay.remove());
    }

    handleAddHost() {
        const name = document.getElementById('host-name').value;
        const ip = document.getElementById('host-ip').value;
        const netmask = document.getElementById('host-netmask').value;
        const gateway = document.getElementById('host-gateway').value;
        
        if (!stateManager.simulator) return;
        
        // Add host at a random position
        const x = 100 + Math.random() * 400;
        const y = 100 + Math.random() * 300;
        
        const hostId = stateManager.simulator.add_host(name, ip, netmask, gateway, x, y);
        console.log(`Host ${name} added with ID ${hostId}`);
        
        this.closeAddHostDialog();
        
        // Update display
        window.dispatchEvent(new Event('hostsUpdated'));
        canvasRenderer.render();
    }

    executePing(hostId) {
        const destinationInput = document.getElementById('ping-destination');
        const destination = destinationInput.value.trim();
        
        if (!destination) {
            alert('Please enter a destination IP address');
            return;
        }
        
        if (!stateManager.simulator) return;
        
        try {
            const identifier = stateManager.simulator.send_ping(hostId, destination);
            console.log(`Ping sent from host ${hostId} to ${destination}, identifier: ${identifier}`);
            
            // Show ping sent message
            const resultsDiv = document.getElementById('ping-results');
            resultsDiv.innerHTML += `<div class="ping-entry">Pinging ${destination}...</div>`;
            
            // Check for results after a short delay
            setTimeout(() => this.checkPingResults(), 100);
            
        } catch (error) {
            console.error('Failed to send ping:', error);
            const resultsDiv = document.getElementById('ping-results');
            resultsDiv.innerHTML += `<div class="ping-entry error">Error: ${error}</div>`;
        }
    }

    checkPingResults() {
        if (!stateManager.simulator) return;
        
        const resultsJson = stateManager.simulator.get_ping_results_json(10);
        if (!resultsJson) return;
        
        const results = JSON.parse(resultsJson);
        const resultsDiv = document.getElementById('ping-results');
        
        results.forEach(result => {
            if (result.success) {
                resultsDiv.innerHTML += `<div class="ping-entry success">
                    Reply from ${result.destination_ip}: time=${result.rtt_ms.toFixed(2)}ms TTL=${result.ttl}
                </div>`;
            } else {
                resultsDiv.innerHTML += `<div class="ping-entry error">
                    Request to ${result.destination_ip} timed out
                </div>`;
            }
        });
        
        // Scroll to bottom
        resultsDiv.scrollTop = resultsDiv.scrollHeight;
    }
}

// Create singleton instance
const hostManager = new HostManager();

// Export as default and make available globally
export default hostManager;
window.hostManager = hostManager; // グローバルアクセス用