/**
 * Terminal Manager Module
 * Handles terminal device management, rendering, and enhanced ping functionality
 */

import stateManager from './state-manager.js';
import canvasRenderer from './canvas-renderer.js';

class TerminalManager {
    constructor() {
        this.terminals = new Map();
        this.selectedTerminal = null;
        this.activePingSessions = new Map();
        this.init();
    }

    init() {
        // Listen for terminal updates
        window.addEventListener('terminalsUpdated', () => this.updateTerminals());
        
        // Listen for canvas clicks to select terminals
        window.addEventListener('canvasClick', (e) => this.handleCanvasClick(e.detail));
        
        // Set up toolbar button
        this.setupToolbarButton();
    }

    setupToolbarButton() {
        // Remove any direct onclick handler from add-terminal button
        // The button should only switch modes, not show dialog immediately
        const addTerminalBtn = document.getElementById('add-terminal-btn');
        if (addTerminalBtn) {
            // Ensure no onclick handler is set
            addTerminalBtn.onclick = null;
        }
        
        // Also handle the legacy add-host button if it exists
        const addHostBtn = document.getElementById('add-host-btn');
        if (addHostBtn) {
            addHostBtn.onclick = null;
        }
    }

    updateTerminals() {
        if (!stateManager.simulator) return;
        
        const terminalsJson = stateManager.simulator.get_all_terminals_json();
        if (terminalsJson) {
            const terminalsArray = JSON.parse(terminalsJson);
            this.terminals.clear();
            terminalsArray.forEach(terminal => {
                this.terminals.set(terminal.id, terminal);
            });
        }
    }

    drawTerminals(ctx) {
        // Draw terminals from stateManager (for positioning) if available
        if (stateManager.terminals && stateManager.terminals.length > 0) {
            stateManager.terminals.forEach(terminal => {
                // Merge with stored position if available
                const storedPosition = stateManager.terminalPositions.get(terminal.id);
                if (storedPosition) {
                    terminal = { ...terminal, x: storedPosition.x, y: storedPosition.y };
                }
                this.drawTerminal(ctx, terminal);
            });
        } else {
            // Fallback to internal terminals map
            this.terminals.forEach(terminal => {
                this.drawTerminal(ctx, terminal);
            });
        }
    }

    drawTerminal(ctx, terminal) {
        const x = terminal.x || 0;
        const y = terminal.y || 0;
        console.log(`Drawing terminal ${terminal.id} at position (${x}, ${y}), original coords:`, { origX: terminal.x, origY: terminal.y });
        
        // Draw terminal icon (modern computer/laptop style)
        ctx.save();
        
        // Shadow for depth
        ctx.shadowColor = 'rgba(0, 0, 0, 0.2)';
        ctx.shadowBlur = 5;
        ctx.shadowOffsetX = 2;
        ctx.shadowOffsetY = 2;
        
        // Terminal body (monitor)
        ctx.fillStyle = terminal.is_failed ? '#ffcccc' : '#f0f0f0';
        ctx.strokeStyle = terminal.is_failed ? '#cc0000' : '#555555';
        ctx.lineWidth = 2;
        
        // Monitor frame
        ctx.beginPath();
        ctx.roundRect(x - 22, y - 16, 44, 28, 3);
        ctx.fill();
        ctx.stroke();
        
        // Screen
        ctx.fillStyle = terminal.is_failed ? '#ff6666' : '#2c3e50';
        ctx.fillRect(x - 19, y - 13, 38, 22);
        
        // Terminal prompt on screen
        if (!terminal.is_failed) {
            ctx.fillStyle = '#00ff00';
            ctx.font = '8px monospace';
            ctx.fillText('>', x - 17, y - 5);
            ctx.fillRect(x - 12, y - 7, 15, 1);
        }
        
        // Base/stand
        ctx.fillStyle = terminal.is_failed ? '#ffcccc' : '#d0d0d0';
        ctx.fillRect(x - 10, y + 12, 20, 3);
        ctx.fillRect(x - 15, y + 15, 30, 4);
        
        ctx.restore();
        
        // Draw terminal name
        ctx.fillStyle = '#2c3e50';
        ctx.font = 'bold 12px Arial';
        ctx.textAlign = 'center';
        ctx.fillText(terminal.name, x, y + 33);
        
        // Draw IP address
        ctx.font = '10px Arial';
        ctx.fillStyle = '#7f8c8d';
        ctx.fillText(terminal.ip_address, x, y + 45);
        
        // Connection status indicator
        if (terminal.connected_router_id) {
            ctx.fillStyle = '#27ae60';
            ctx.beginPath();
            ctx.arc(x + 20, y - 10, 4, 0, Math.PI * 2);
            ctx.fill();
        }
        
        // Selection indicator
        if (this.selectedTerminal === terminal.id) {
            ctx.strokeStyle = '#3498db';
            ctx.lineWidth = 2;
            ctx.setLineDash([5, 5]);
            ctx.beginPath();
            ctx.arc(x, y, 40, 0, Math.PI * 2);
            ctx.stroke();
            ctx.setLineDash([]);
        }
        
        // Active ping indicator
        if (this.activePingSessions.has(terminal.id)) {
            ctx.fillStyle = '#e74c3c';
            ctx.beginPath();
            ctx.arc(x - 20, y - 10, 3, 0, Math.PI * 2);
            ctx.fill();
            
            // Ping animation
            const time = Date.now() / 1000;
            const radius = (time % 1) * 20;
            ctx.strokeStyle = 'rgba(231, 76, 60, ' + (1 - radius/20) + ')';
            ctx.lineWidth = 2;
            ctx.beginPath();
            ctx.arc(x - 20, y - 10, radius, 0, Math.PI * 2);
            ctx.stroke();
        }
    }

    handleCanvasClick(detail) {
        const { x, y } = detail;
        
        // Check if click is on a terminal from stateManager
        if (stateManager.terminals && stateManager.terminals.length > 0) {
            stateManager.terminals.forEach(terminal => {
                // Get position from stored positions or terminal data
                const storedPosition = stateManager.terminalPositions.get(terminal.id);
                const terminalX = storedPosition ? storedPosition.x : (terminal.x || 0);
                const terminalY = storedPosition ? storedPosition.y : (terminal.y || 0);
                
                const distance = Math.sqrt(
                    Math.pow(x - terminalX, 2) + 
                    Math.pow(y - terminalY, 2)
                );
                
                if (distance < 35) {
                    this.selectTerminal(terminal.id);
                }
            });
        } else {
            // Fallback to internal terminals map
            this.terminals.forEach(terminal => {
                const distance = Math.sqrt(
                    Math.pow(x - (terminal.x || 0), 2) + 
                    Math.pow(y - (terminal.y || 0), 2)
                );
                
                if (distance < 35) {
                    this.selectTerminal(terminal.id);
                }
            });
        }
    }

    selectTerminal(terminalId) {
        this.selectedTerminal = terminalId;
        this.showTerminalDetails(terminalId);
        canvasRenderer.render();
    }

    showTerminalDetails(terminalId) {
        if (!stateManager.simulator) return;
        
        const terminal = this.terminals.get(terminalId);
        if (!terminal) return;
        
        this.showTerminalDetailsDialog(terminal);
    }

    showTerminalDetailsDialog(terminal) {
        // Remove existing dialog if any
        const existingDialog = document.getElementById('terminal-details-dialog');
        if (existingDialog) {
            existingDialog.remove();
        }
        
        const dialog = document.createElement('div');
        dialog.id = 'terminal-details-dialog';
        dialog.className = 'terminal-details-dialog';
        dialog.innerHTML = `
            <div class="dialog-content">
                <div class="dialog-header">
                    <h3><i class="fas fa-desktop"></i> ${terminal.name}</h3>
                    <button class="close-btn" onclick="window.terminalManager.closeTerminalDetailsDialog()">×</button>
                </div>
                <div class="dialog-body">
                    <div class="terminal-info">
                        <div class="detail-row">
                            <span class="detail-label">IP Address:</span>
                            <span class="detail-value">${terminal.ip_address}</span>
                        </div>
                        <div class="detail-row">
                            <span class="detail-label">Netmask:</span>
                            <span class="detail-value">${terminal.netmask}</span>
                        </div>
                        <div class="detail-row">
                            <span class="detail-label">Default Gateway:</span>
                            <span class="detail-value">${terminal.default_gateway}</span>
                        </div>
                        <div class="detail-row">
                            <span class="detail-label">Status:</span>
                            <span class="detail-value ${terminal.is_failed ? 'failed' : 'active'}">
                                ${terminal.is_failed ? 'Failed' : 'Active'}
                            </span>
                        </div>
                        ${terminal.connected_router_id ? `
                        <div class="detail-row">
                            <span class="detail-label">Connected to:</span>
                            <span class="detail-value">Router ${terminal.connected_router_id}</span>
                        </div>
                        ` : ''}
                    </div>
                    
                    <div class="ping-section">
                        <h4><i class="fas fa-network-wired"></i> Enhanced Ping</h4>
                        <div class="ping-controls">
                            <div class="ping-form">
                                <input type="text" id="ping-destination" 
                                    placeholder="Destination IP" 
                                    class="ping-input"
                                    pattern="^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$">
                                <div class="ping-options">
                                    <label>
                                        Count: 
                                        <input type="number" id="ping-count" 
                                            min="0" max="100" value="4" 
                                            title="0 for continuous">
                                    </label>
                                    <label>
                                        TTL: 
                                        <input type="number" id="ping-ttl" 
                                            min="1" max="255" value="64">
                                    </label>
                                    <label>
                                        Size: 
                                        <input type="number" id="ping-size" 
                                            min="8" max="1472" value="56">
                                    </label>
                                </div>
                                <div class="ping-buttons">
                                    <button onclick="window.terminalManager.startPing(${terminal.id})" 
                                        class="ping-btn primary">
                                        <i class="fas fa-play"></i> Start Ping
                                    </button>
                                    <button onclick="window.terminalManager.stopPing(${terminal.id})" 
                                        class="ping-btn secondary"
                                        id="stop-ping-btn" style="display: none;">
                                        <i class="fas fa-stop"></i> Stop
                                    </button>
                                </div>
                            </div>
                        </div>
                        
                        <div id="ping-sessions" class="ping-sessions"></div>
                        <div id="ping-results" class="ping-results"></div>
                    </div>
                    
                    <div class="terminal-actions">
                        <button onclick="window.terminalManager.toggleTerminalFailure(${terminal.id})" 
                            class="action-btn ${terminal.is_failed ? 'recover' : 'fail'}">
                            <i class="fas fa-${terminal.is_failed ? 'check-circle' : 'times-circle'}"></i>
                            ${terminal.is_failed ? 'Recover' : 'Fail'} Terminal
                        </button>
                        ${!terminal.connected_router_id ? `
                        <button onclick="window.terminalManager.showConnectDialog(${terminal.id})" 
                            class="action-btn connect">
                            <i class="fas fa-plug"></i> Connect to Router
                        </button>
                        ` : `
                        <button onclick="window.terminalManager.disconnectTerminal(${terminal.id})" 
                            class="action-btn disconnect">
                            <i class="fas fa-unlink"></i> Disconnect
                        </button>
                        `}
                    </div>
                </div>
            </div>
        `;
        
        document.body.appendChild(dialog);
        
        // Update ping session display
        this.updatePingSessionDisplay(terminal.id);
    }

    closeTerminalDetailsDialog() {
        const dialog = document.getElementById('terminal-details-dialog');
        if (dialog) {
            dialog.remove();
        }
    }

    showAddTerminalDialogAtPosition(x, y) {
        // Store the position for use when terminal is created
        this.pendingTerminalPosition = { x, y };
        this.showAddTerminalDialog();
    }

    showAddTerminalDialog() {
        const dialog = document.createElement('div');
        dialog.className = 'modal-overlay';
        dialog.innerHTML = `
            <div class="modal">
                <div class="modal-header">
                    <h3><i class="fas fa-desktop"></i> Add Terminal Device</h3>
                    <button class="close-btn" onclick="window.terminalManager.closeAddTerminalDialog()">×</button>
                </div>
                <form id="add-terminal-form" class="modal-form">
                    <div class="form-group">
                        <label><i class="fas fa-tag"></i> Terminal Name:</label>
                        <input type="text" id="terminal-name" required 
                            placeholder="e.g., PC1, Laptop1">
                    </div>
                    <div class="form-group">
                        <label><i class="fas fa-network-wired"></i> IP Address:</label>
                        <input type="text" id="terminal-ip" 
                            pattern="^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$" 
                            required placeholder="e.g., 192.168.1.100">
                    </div>
                    <div class="form-group">
                        <label><i class="fas fa-mask"></i> Netmask:</label>
                        <input type="text" id="terminal-netmask" 
                            value="255.255.255.0" 
                            pattern="^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$" 
                            required>
                    </div>
                    <div class="form-group">
                        <label><i class="fas fa-route"></i> Default Gateway:</label>
                        <input type="text" id="terminal-gateway" 
                            pattern="^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$" 
                            required placeholder="e.g., 192.168.1.1">
                    </div>
                    <div class="form-actions">
                        <button type="submit" class="btn primary">
                            <i class="fas fa-plus"></i> Add Terminal
                        </button>
                        <button type="button" class="btn secondary" 
                            onclick="window.terminalManager.closeAddTerminalDialog()">
                            Cancel
                        </button>
                    </div>
                </form>
            </div>
        `;
        
        document.body.appendChild(dialog);
        
        // Handle form submission
        const form = document.getElementById('add-terminal-form');
        form.addEventListener('submit', (e) => {
            e.preventDefault();
            this.handleAddTerminal();
        });
        
        // Focus on first input
        document.getElementById('terminal-name').focus();
    }

    closeAddTerminalDialog() {
        const modal = document.querySelector('.modal-overlay');
        if (modal) {
            modal.remove();
        }
    }

    handleAddTerminal() {
        const name = document.getElementById('terminal-name').value;
        const ip = document.getElementById('terminal-ip').value;
        const netmask = document.getElementById('terminal-netmask').value;
        const gateway = document.getElementById('terminal-gateway').value;
        
        if (!stateManager.simulator) return;
        
        // Use the position stored when the user clicked on the canvas
        let x, y;
        if (this.pendingTerminalPosition) {
            x = this.pendingTerminalPosition.x;
            y = this.pendingTerminalPosition.y;
            // Clear the stored position
            this.pendingTerminalPosition = null;
        } else {
            // Fallback to center position if no position was stored
            const canvas = document.getElementById('network-canvas');
            x = canvas.width / 2 + (Math.random() - 0.5) * 200;
            y = canvas.height / 2 + (Math.random() - 0.5) * 200;
        }
        
        try {
            const terminalId = stateManager.simulator.add_terminal(name, ip, netmask, gateway, x, y);
            console.log(`Terminal ${name} added with ID ${terminalId} at position (${x}, ${y})`);
            
            // Explicitly update terminal position in WebAssembly to ensure it's stored
            stateManager.simulator.update_terminal_position(terminalId, x, y);
            
            // Store position locally
            stateManager.terminalPositions.set(terminalId, { x, y });
            
            this.closeAddTerminalDialog();
            
            // Update display
            window.dispatchEvent(new Event('terminalsUpdated'));
            
            // Update terminals from simulator
            if (window.canvasInteraction) {
                window.canvasInteraction.updateTerminalsFromSimulator();
            }
            
            canvasRenderer.render();
            
            // Select the new terminal
            setTimeout(() => {
                this.selectTerminal(terminalId);
            }, 100);
            
        } catch (error) {
            alert(`Failed to add terminal: ${error}`);
        }
    }

    startPing(terminalId) {
        const destination = document.getElementById('ping-destination').value.trim();
        const count = parseInt(document.getElementById('ping-count').value) || 4;
        const ttl = parseInt(document.getElementById('ping-ttl').value) || 64;
        const size = parseInt(document.getElementById('ping-size').value) || 56;
        
        if (!destination) {
            alert('Please enter a destination IP address');
            return;
        }
        
        if (!stateManager.simulator) return;
        
        const terminal = this.terminals.get(terminalId);
        if (!terminal) return;
        
        try {
            const sessionId = stateManager.simulator.start_enhanced_ping(
                terminalId,
                terminal.ip_address,
                destination,
                count,
                1.0,  // 1 second interval
                size,
                ttl
            );
            
            console.log(`Started ping session ${sessionId} from terminal ${terminalId}`);
            
            // Store session info
            this.activePingSessions.set(terminalId, {
                sessionId,
                destination,
                startTime: Date.now(),
                count,
                sent: 0
            });
            
            // Update UI
            document.getElementById('stop-ping-btn').style.display = 'inline-block';
            this.updatePingSessionDisplay(terminalId);
            
            // Start sending pings
            this.sendNextPing(terminalId);
            
        } catch (error) {
            console.error('Failed to start ping:', error);
            this.showPingError(error.toString());
        }
    }

    stopPing(terminalId) {
        const session = this.activePingSessions.get(terminalId);
        if (!session || !stateManager.simulator) return;
        
        try {
            const summaryJson = stateManager.simulator.stop_ping_session(session.sessionId);
            const summary = JSON.parse(summaryJson);
            
            this.showPingSummary(summary);
            this.activePingSessions.delete(terminalId);
            
            // Update UI
            document.getElementById('stop-ping-btn').style.display = 'none';
            canvasRenderer.render();
            
        } catch (error) {
            console.error('Failed to stop ping:', error);
        }
    }

    sendNextPing(terminalId) {
        const session = this.activePingSessions.get(terminalId);
        if (!session || !stateManager.simulator) return;
        
        // Check if session is complete
        if (session.count > 0 && session.sent >= session.count) {
            this.stopPing(terminalId);
            return;
        }
        
        // Send next ping
        const sent = stateManager.simulator.send_next_ping(session.sessionId);
        if (sent) {
            session.sent++;
            this.updatePingSessionDisplay(terminalId);
            
            // Schedule next ping after interval
            setTimeout(() => {
                this.checkPingResults(terminalId);
                this.sendNextPing(terminalId);
            }, 1000);
        }
    }

    checkPingResults(terminalId) {
        const session = this.activePingSessions.get(terminalId);
        if (!session || !stateManager.simulator) return;
        
        const detailsJson = stateManager.simulator.get_ping_session_details(session.sessionId);
        const details = JSON.parse(detailsJson);
        
        this.updatePingResults(details);
    }

    updatePingSessionDisplay(terminalId) {
        const sessionsDiv = document.getElementById('ping-sessions');
        if (!sessionsDiv) return;
        
        const session = this.activePingSessions.get(terminalId);
        if (session) {
            const statsJson = stateManager.simulator.get_ping_session_details(session.sessionId);
            const stats = JSON.parse(statsJson);
            
            sessionsDiv.innerHTML = `
                <div class="ping-session-info">
                    <h5>Active Ping Session</h5>
                    <div class="session-stats">
                        <span>Destination: ${session.destination}</span>
                        <span>Sent: ${stats.packets_sent}</span>
                        <span>Received: ${stats.packets_received}</span>
                        <span>Lost: ${stats.packets_lost}</span>
                        ${stats.avg_rtt_ms ? `<span>Avg RTT: ${stats.avg_rtt_ms.toFixed(2)}ms</span>` : ''}
                    </div>
                </div>
            `;
        } else {
            sessionsDiv.innerHTML = '';
        }
    }

    updatePingResults(details) {
        const resultsDiv = document.getElementById('ping-results');
        if (!resultsDiv) return;
        
        // Clear old results if too many
        if (resultsDiv.children.length > 20) {
            resultsDiv.innerHTML = '';
        }
        
        // Add new results
        details.results.forEach(result => {
            const existingEntry = resultsDiv.querySelector(`[data-seq="${result.sequence_number}"]`);
            if (!existingEntry) {
                const entry = document.createElement('div');
                entry.className = `ping-entry ${result.success ? 'success' : 'error'}`;
                entry.setAttribute('data-seq', result.sequence_number);
                
                if (result.success) {
                    entry.innerHTML = `
                        <i class="fas fa-check-circle"></i>
                        Reply from ${details.destination_ip}: 
                        bytes=${details.config.packet_size} 
                        time=${result.rtt_ms.toFixed(2)}ms 
                        TTL=${result.reply_ttl || 'N/A'}
                    `;
                } else {
                    entry.innerHTML = `
                        <i class="fas fa-times-circle"></i>
                        ${result.error_message || 'Request timed out'}
                    `;
                }
                
                resultsDiv.appendChild(entry);
                resultsDiv.scrollTop = resultsDiv.scrollHeight;
            }
        });
    }

    showPingSummary(summary) {
        const resultsDiv = document.getElementById('ping-results');
        if (!resultsDiv) return;
        
        const summaryDiv = document.createElement('div');
        summaryDiv.className = 'ping-summary';
        summaryDiv.innerHTML = `
            <h5>Ping Statistics for ${summary.destination_ip}:</h5>
            <p>Packets: Sent = ${summary.packets_sent}, Received = ${summary.packets_received}, Lost = ${summary.packets_lost} (${summary.loss_percentage.toFixed(1)}% loss)</p>
            ${summary.packets_received > 0 ? `
            <p>Round trip times: Min = ${summary.min_rtt_ms.toFixed(2)}ms, Max = ${summary.max_rtt_ms.toFixed(2)}ms, Avg = ${summary.avg_rtt_ms.toFixed(2)}ms</p>
            ` : ''}
        `;
        
        resultsDiv.appendChild(summaryDiv);
    }

    showPingError(error) {
        const resultsDiv = document.getElementById('ping-results');
        if (!resultsDiv) return;
        
        const errorDiv = document.createElement('div');
        errorDiv.className = 'ping-entry error';
        errorDiv.innerHTML = `<i class="fas fa-exclamation-triangle"></i> Error: ${error}`;
        resultsDiv.appendChild(errorDiv);
    }

    toggleTerminalFailure(terminalId) {
        if (!stateManager.simulator) return;
        
        const terminal = this.terminals.get(terminalId);
        if (!terminal) return;
        
        const newState = !terminal.is_failed;
        const success = stateManager.simulator.set_terminal_failed(terminalId, newState);
        
        if (success) {
            console.log(`Terminal ${terminalId} ${newState ? 'failed' : 'recovered'}`);
            
            // Update display
            window.dispatchEvent(new Event('terminalsUpdated'));
            canvasRenderer.render();
            
            // Refresh dialog
            this.showTerminalDetails(terminalId);
        }
    }

    showConnectDialog(terminalId) {
        if (!stateManager.simulator) return;
        
        // Get available routers
        const routersJson = stateManager.simulator.get_routers_json();
        const routers = JSON.parse(routersJson);
        
        const dialog = document.createElement('div');
        dialog.className = 'modal-overlay small';
        dialog.innerHTML = `
            <div class="modal">
                <div class="modal-header">
                    <h3>Connect Terminal to Router</h3>
                    <button class="close-btn" onclick="window.terminalManager.closeConnectDialog()">×</button>
                </div>
                <div class="modal-body">
                    <p>Select a router to connect to:</p>
                    <select id="router-select" class="router-select">
                        ${routers.map(router => `
                            <option value="${router.id}">
                                ${router.name} (${router.router_id})
                            </option>
                        `).join('')}
                    </select>
                </div>
                <div class="modal-actions">
                    <button onclick="window.terminalManager.connectToRouter(${terminalId})" 
                        class="btn primary">
                        <i class="fas fa-plug"></i> Connect
                    </button>
                    <button onclick="window.terminalManager.closeConnectDialog()" 
                        class="btn secondary">
                        Cancel
                    </button>
                </div>
            </div>
        `;
        
        document.body.appendChild(dialog);
    }

    closeConnectDialog() {
        const modal = document.querySelector('.modal-overlay.small');
        if (modal) {
            modal.remove();
        }
    }

    connectToRouter(terminalId) {
        const routerSelect = document.getElementById('router-select');
        const routerId = parseInt(routerSelect.value);
        
        if (!stateManager.simulator) return;
        
        try {
            stateManager.simulator.connect_terminal_to_router(terminalId, routerId);
            console.log(`Connected terminal ${terminalId} to router ${routerId}`);
            
            this.closeConnectDialog();
            
            // Update display
            window.dispatchEvent(new Event('terminalsUpdated'));
            canvasRenderer.render();
            
            // Refresh dialog
            this.showTerminalDetails(terminalId);
            
        } catch (error) {
            alert(`Failed to connect: ${error}`);
        }
    }

    disconnectTerminal(terminalId) {
        if (!stateManager.simulator) return;
        
        try {
            stateManager.simulator.disconnect_terminal(terminalId);
            console.log(`Disconnected terminal ${terminalId}`);
            
            // Update display
            window.dispatchEvent(new Event('terminalsUpdated'));
            canvasRenderer.render();
            
            // Refresh dialog
            this.showTerminalDetails(terminalId);
            
        } catch (error) {
            alert(`Failed to disconnect: ${error}`);
        }
    }
}

// Create singleton instance
const terminalManager = new TerminalManager();

// Export as default and make available globally
export default terminalManager;
window.terminalManager = terminalManager;